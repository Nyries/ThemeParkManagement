use std::collections::{HashSet, VecDeque};

use crate::map::{InfrastructureShape, ParkMap};
use crate::visitor::VisitorId;

/// Minimal dedicated state for one attraction's queue — not a general building-instance
/// architecture (that stays with TPM-49/141), just what TPM-45 needs: the cached chain
/// of queue cells and who's currently waiting in it, FIFO.
#[derive(Debug, Clone, Default)]
pub struct QueueState {
    /// Ordered from the head (adjacent to the attraction) to the tail (entrance).
    pub chain: Vec<(i32, i32, i32)>,
    pub occupants: VecDeque<VisitorId>,
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
}
