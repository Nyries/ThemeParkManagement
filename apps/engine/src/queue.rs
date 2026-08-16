use std::collections::{HashSet, VecDeque};

use crate::balance::QUEUE_SPACING_FACTOR;
use crate::map::{InfrastructureShape, ParkMap};
use crate::visitor::VisitorId;

/// Minimal dedicated state for one attraction's queue — not a general building-instance
/// architecture (that stays with TPM-49/141), just what TPM-45 needs: the cached chain
/// of queue cells and who's currently waiting in it, FIFO.
#[derive(Debug, Clone, Default)]
pub struct QueueState {
    /// Ordered from the head (adjacent to the attraction) to the tail (entrance).
    pub chain: Vec<(i32, i32, i32)>,
    /// Front = next to board (closest to the head of `chain`).
    pub occupants: VecDeque<VisitorId>,
}

impl QueueState {
    /// Continuous target position for the occupant at `index` (0 = next to board),
    /// `index * visitor_diameter * QUEUE_SPACING_FACTOR` from the head of the chain.
    pub fn target_position(&self, index: usize, visitor_diameter: f32) -> Option<(f32, f32, f32)> {
        let distance = index as f32 * visitor_diameter * QUEUE_SPACING_FACTOR;
        position_along_chain(&self.chain, distance)
    }

    /// Whether the physical queue is already at capacity (see `queue_capacity`).
    pub fn is_full(&self, visitor_diameter: f32) -> bool {
        self.occupants.len() >= queue_capacity(&self.chain, visitor_diameter)
    }
}

fn neighbors(cell: (i32, i32, i32)) -> [(i32, i32, i32); 4] {
    let (x, y, z) = cell;
    [(x + 1, y, z), (x - 1, y, z), (x, y + 1, z), (x, y - 1, z)]
}

fn is_queue_cell_for(park_map: &ParkMap, cell: (i32, i32, i32), attraction_id: &str) -> bool {
    matches!(
        park_map.get_infrastructure(cell.0, cell.1, cell.2),
        Some(InfrastructureShape::Queue { attraction_id: a }) if a.building_id == attraction_id
    )
}

/// Derives the ordered chain of queue cells belonging to `attraction_id`, from the head
/// (a queue cell adjacent to the attraction's own footprint) to the tail. Empty if the
/// attraction has no queue cells, or none of them is adjacent to its footprint.
/// Assumes a single non-branching chain (no fork/merge) — walks greedily otherwise.
pub fn derive_queue_chain(park_map: &ParkMap, attraction_id: &str) -> Vec<(i32, i32, i32)> {
    let head = park_map
        .infrastructure
        .keys()
        .filter(|&&cell| is_queue_cell_for(park_map, cell, attraction_id))
        .find(|&&cell| {
            neighbors(cell).into_iter().any(|n| {
                park_map
                    .get_building(n.0, n.1, n.2)
                    .is_some_and(|b| b.building_id == attraction_id)
            })
        });

    let Some(&head) = head else {
        return Vec::new();
    };

    let mut chain = vec![head];
    let mut visited: HashSet<(i32, i32, i32)> = HashSet::from([head]);
    let mut current = head;

    loop {
        let next = neighbors(current)
            .into_iter()
            .find(|&n| !visited.contains(&n) && is_queue_cell_for(park_map, n, attraction_id));
        match next {
            Some(n) => {
                chain.push(n);
                visited.insert(n);
                current = n;
            }
            None => break,
        }
    }

    chain
}

fn dist(a: (i32, i32, i32), b: (i32, i32, i32)) -> f32 {
    let dx = (b.0 - a.0) as f32;
    let dy = (b.1 - a.1) as f32;
    let dz = (b.2 - a.2) as f32;
    (dx * dx + dy * dy + dz * dz).sqrt()
}

/// Cumulative length of the chain's polyline (sum of consecutive segment distances).
pub fn chain_length(chain: &[(i32, i32, i32)]) -> f32 {
    chain.windows(2).map(|w| dist(w[0], w[1])).sum()
}

/// Physical capacity of the chain — not "1 visitor per cell": the chain's length
/// divided by `visitor_diameter`, plus the occupant that always fits at distance 0.
pub fn queue_capacity(chain: &[(i32, i32, i32)], visitor_diameter: f32) -> usize {
    if chain.is_empty() || visitor_diameter <= 0.0 {
        return 0;
    }
    (chain_length(chain) / visitor_diameter).floor() as usize + 1
}

/// The point at cumulative `distance` along the chain's polyline, clamped to
/// `[0, chain_length]`. `None` if the chain is empty.
pub fn position_along_chain(chain: &[(i32, i32, i32)], distance: f32) -> Option<(f32, f32, f32)> {
    let &first = chain.first()?;
    if chain.len() == 1 {
        return Some((first.0 as f32, first.1 as f32, first.2 as f32));
    }

    let distance = distance.max(0.0);
    let mut remaining = distance;
    for w in chain.windows(2) {
        let segment_len = dist(w[0], w[1]);
        if remaining <= segment_len {
            let t = if segment_len > 0.0 { remaining / segment_len } else { 0.0 };
            let (x0, y0, z0) = (w[0].0 as f32, w[0].1 as f32, w[0].2 as f32);
            let (x1, y1, z1) = (w[1].0 as f32, w[1].1 as f32, w[1].2 as f32);
            return Some((x0 + (x1 - x0) * t, y0 + (y1 - y0) * t, z0 + (z1 - z0) * t));
        }
        remaining -= segment_len;
    }

    let &last = chain.last()?;
    Some((last.0 as f32, last.1 as f32, last.2 as f32))
}

/// Moves `current` toward `target` by at most `max_step`, without overshooting.
pub fn advance_toward(
    current: (f32, f32, f32),
    target: (f32, f32, f32),
    max_step: f32,
) -> (f32, f32, f32) {
    let d = dist_f32(current, target);
    if d <= max_step || d == 0.0 {
        return target;
    }
    let t = max_step / d;
    (
        current.0 + (target.0 - current.0) * t,
        current.1 + (target.1 - current.1) * t,
        current.2 + (target.2 - current.2) * t,
    )
}

fn dist_f32(a: (f32, f32, f32), b: (f32, f32, f32)) -> f32 {
    let dx = b.0 - a.0;
    let dy = b.1 - a.1;
    let dz = b.2 - a.2;
    (dx * dx + dy * dy + dz * dz).sqrt()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::map::{Bounds3d, BuildingId};

    fn build_map() -> ParkMap {
        ParkMap::new("m".into(), Bounds3d::new(-5, 5, -5, 5, -1, 1))
    }

    fn set_queue(park_map: &mut ParkMap, cell: (i32, i32, i32), attraction_id: &str) {
        park_map.set_infrastructure(
            cell.0,
            cell.1,
            cell.2,
            InfrastructureShape::Queue {
                attraction_id: BuildingId {
                    building_id: attraction_id.into(),
                    template_id: "roller_coaster".into(),
                },
            },
        );
    }

    mod derive_queue_chain {
        use super::*;

        #[test]
        fn test_empty_when_no_queue_cells_exist() {
            let park_map = build_map();

            let chain = derive_queue_chain(&park_map, "coaster");

            assert!(chain.is_empty());
        }

        #[test]
        fn test_empty_when_queue_cells_exist_but_none_is_adjacent_to_the_attraction() {
            let mut park_map = build_map();
            set_queue(&mut park_map, (5, 5, 0), "coaster");
            park_map.set_building(
                0,
                0,
                0,
                BuildingId {
                    building_id: "coaster".into(),
                    template_id: "roller_coaster".into(),
                },
            );

            let chain = derive_queue_chain(&park_map, "coaster");

            assert!(chain.is_empty());
        }

        #[test]
        fn test_orders_a_straight_chain_from_the_head_adjacent_to_the_attraction() {
            let mut park_map = build_map();
            park_map.set_building(
                0,
                0,
                0,
                BuildingId {
                    building_id: "coaster".into(),
                    template_id: "roller_coaster".into(),
                },
            );
            // Chain: (1,0,0) adjacent to the attraction, then (2,0,0), then (3,0,0).
            set_queue(&mut park_map, (1, 0, 0), "coaster");
            set_queue(&mut park_map, (2, 0, 0), "coaster");
            set_queue(&mut park_map, (3, 0, 0), "coaster");

            let chain = derive_queue_chain(&park_map, "coaster");

            assert_eq!(chain, vec![(1, 0, 0), (2, 0, 0), (3, 0, 0)]);
        }

        #[test]
        fn test_follows_a_turn_in_the_chain() {
            let mut park_map = build_map();
            park_map.set_building(
                0,
                0,
                0,
                BuildingId {
                    building_id: "coaster".into(),
                    template_id: "roller_coaster".into(),
                },
            );
            set_queue(&mut park_map, (1, 0, 0), "coaster");
            set_queue(&mut park_map, (2, 0, 0), "coaster");
            set_queue(&mut park_map, (2, 1, 0), "coaster"); // turn upward

            let chain = derive_queue_chain(&park_map, "coaster");

            assert_eq!(chain, vec![(1, 0, 0), (2, 0, 0), (2, 1, 0)]);
        }

        #[test]
        fn test_ignores_queue_cells_belonging_to_a_different_attraction() {
            let mut park_map = build_map();
            park_map.set_building(
                0,
                0,
                0,
                BuildingId {
                    building_id: "coaster".into(),
                    template_id: "roller_coaster".into(),
                },
            );
            set_queue(&mut park_map, (1, 0, 0), "coaster");
            set_queue(&mut park_map, (2, 0, 0), "other_attraction");

            let chain = derive_queue_chain(&park_map, "coaster");

            assert_eq!(chain, vec![(1, 0, 0)]);
        }
    }

    mod chain_length {
        use super::*;

        #[test]
        fn test_zero_for_a_single_cell_chain() {
            assert_eq!(chain_length(&[(0, 0, 0)]), 0.0);
        }

        #[test]
        fn test_sums_consecutive_segment_distances() {
            let chain = vec![(0, 0, 0), (1, 0, 0), (1, 1, 0)];

            assert_eq!(chain_length(&chain), 2.0);
        }

        #[test]
        fn test_empty_for_an_empty_chain() {
            assert_eq!(chain_length(&[]), 0.0);
        }
    }

    mod queue_capacity {
        use super::*;

        #[test]
        fn test_zero_for_an_empty_chain() {
            assert_eq!(queue_capacity(&[], 0.2), 0);
        }

        #[test]
        fn test_a_single_cell_chain_still_holds_one_occupant() {
            assert_eq!(queue_capacity(&[(0, 0, 0)], 0.2), 1);
        }

        #[test]
        fn test_divides_chain_length_by_visitor_diameter_plus_one() {
            // Length 4.0, diameter 0.5 -> 8 spacings plus the occupant at distance 0.
            let chain = vec![(0, 0, 0), (4, 0, 0)];

            assert_eq!(queue_capacity(&chain, 0.5), 9);
        }
    }

    mod position_along_chain {
        use super::*;

        #[test]
        fn test_none_for_an_empty_chain() {
            assert_eq!(position_along_chain(&[], 1.0), None);
        }

        #[test]
        fn test_returns_the_only_point_for_a_single_cell_chain_regardless_of_distance() {
            let result = position_along_chain(&[(2, 3, 0)], 5.0);

            assert_eq!(result, Some((2.0, 3.0, 0.0)));
        }

        #[test]
        fn test_interpolates_within_a_segment() {
            let chain = vec![(0, 0, 0), (2, 0, 0)];

            let result = position_along_chain(&chain, 0.5);

            assert_eq!(result, Some((0.5, 0.0, 0.0)));
        }

        #[test]
        fn test_clamps_to_the_end_of_the_chain_past_its_length() {
            let chain = vec![(0, 0, 0), (1, 0, 0)];

            let result = position_along_chain(&chain, 100.0);

            assert_eq!(result, Some((1.0, 0.0, 0.0)));
        }

        #[test]
        fn test_clamps_to_the_start_for_a_negative_distance() {
            let chain = vec![(0, 0, 0), (1, 0, 0)];

            let result = position_along_chain(&chain, -5.0);

            assert_eq!(result, Some((0.0, 0.0, 0.0)));
        }

        #[test]
        fn test_crosses_a_turn_into_the_second_segment() {
            let chain = vec![(0, 0, 0), (1, 0, 0), (1, 1, 0)];

            let result = position_along_chain(&chain, 1.5);

            assert_eq!(result, Some((1.0, 0.5, 0.0)));
        }
    }

    mod advance_toward {
        use super::*;

        #[test]
        fn test_snaps_to_target_when_within_max_step() {
            let result = advance_toward((0.0, 0.0, 0.0), (0.1, 0.0, 0.0), 1.0);

            assert_eq!(result, (0.1, 0.0, 0.0));
        }

        #[test]
        fn test_moves_partway_when_target_is_further_than_max_step() {
            let result = advance_toward((0.0, 0.0, 0.0), (10.0, 0.0, 0.0), 1.0);

            assert_eq!(result, (1.0, 0.0, 0.0));
        }

        #[test]
        fn test_returns_current_position_when_already_at_target() {
            let result = advance_toward((3.0, 3.0, 0.0), (3.0, 3.0, 0.0), 1.0);

            assert_eq!(result, (3.0, 3.0, 0.0));
        }
    }

    mod queue_state {
        use super::*;

        #[test]
        fn test_target_position_places_the_front_occupant_at_the_head() {
            let state = QueueState {
                chain: vec![(0, 0, 0), (1, 0, 0)],
                occupants: VecDeque::new(),
            };

            let result = state.target_position(0, 0.2);

            assert_eq!(result, Some((0.0, 0.0, 0.0)));
        }

        #[test]
        fn test_target_position_spaces_occupants_by_diameter() {
            let state = QueueState {
                chain: vec![(0, 0, 0), (10, 0, 0)],
                occupants: VecDeque::new(),
            };

            let result = state.target_position(3, 0.2);

            assert_eq!(result, Some((0.6, 0.0, 0.0)));
        }

        #[test]
        fn test_is_full_compares_occupant_count_to_capacity() {
            let state = QueueState {
                chain: vec![(0, 0, 0)],
                occupants: VecDeque::from(["a".to_string()]),
            };

            assert!(state.is_full(0.2));
        }

        #[test]
        fn test_is_not_full_below_capacity() {
            let state = QueueState {
                chain: vec![(0, 0, 0), (10, 0, 0)],
                occupants: VecDeque::from(["a".to_string()]),
            };

            assert!(!state.is_full(0.2));
        }
    }
}
