use std::collections::{HashMap, HashSet};

use rand::Rng;

use super::cell_of;

use crate::{
    balance::{
        DENSITY_CAP, DETOUR_BIAS_STRENGTH, DETOUR_DENSITY_THRESHOLD, DETOUR_LOOKAHEAD_CELLS,
        STALL_DISTANCE_EPSILON, STALL_TICKS_THRESHOLD, UNSTALL_IMPULSE_MAGNITUDE,
    },
    map::{ParkMap, base_speed_for},
    visitor::{
        Visitor, VisitorId, lane_bias_strength, perpendicular_of, repulsion_force, speed_at,
        weighted_lane_bias,
    },
};

fn is_walkable_at(park_map: &ParkMap, position: (f32, f32, f32)) -> bool {
    let cell = cell_of(position);
    park_map.is_walkable(cell.0, cell.1, cell.2)
}

/// Slides along a single axis if the full move left walkable ground, only fully
/// reverting to `before` if neither axis alone works either.
pub(super) fn clamp_to_walkable_ground(
    v: &mut Visitor,
    park_map: &ParkMap,
    before: (f32, f32, f32),
) {
    let after = v.position;
    if is_walkable_at(park_map, after) {
        return;
    }

    let x_only = (after.0, before.1, before.2);
    if is_walkable_at(park_map, x_only) {
        v.position = x_only;
        return;
    }

    let y_only = (before.0, after.1, before.2);
    if is_walkable_at(park_map, y_only) {
        v.position = y_only;
        return;
    }

    v.position = before;
}

/// Tracks head-on repulsion standoffs: a visitor with a nonzero speed that still
/// barely moved this tick is one tick closer to being considered stalled. Any real
/// progress (or having no path to walk in the first place) resets the counter. Once
/// stalled long enough, nudges the visitor sideways to break the deadlock.
pub(super) fn update_stall_tracking(v: &mut Visitor, park_map: &ParkMap, speed: f32, moved: f32) {
    if speed > 0.0 && moved < STALL_DISTANCE_EPSILON && !v.path.is_empty() {
        v.stall_ticks += 1;
    } else {
        v.stall_ticks = 0;
    }

    if v.stall_ticks >= STALL_TICKS_THRESHOLD {
        apply_unstall_impulse(v, park_map);
        v.stall_ticks = 0;
    }
}

/// Nudges a stalled visitor by a small random offset, only committing it if the
/// resulting cell is still walkable — breaks a symmetric head-on repulsion deadlock
/// (see `update_stall_tracking`) without a real yielding rule (none exists yet).
fn apply_unstall_impulse(v: &mut Visitor, park_map: &ParkMap) {
    let angle: f32 = rand::thread_rng().gen_range(0.0..std::f32::consts::TAU);
    let candidate = (
        v.position.0 + angle.cos() * UNSTALL_IMPULSE_MAGNITUDE,
        v.position.1 + angle.sin() * UNSTALL_IMPULSE_MAGNITUDE,
        v.position.2,
    );
    let candidate_cell = cell_of(candidate);
    if park_map.is_walkable(candidate_cell.0, candidate_cell.1, candidate_cell.2) {
        v.position = candidate;
    }
}

pub(super) fn local_density_at(
    density: &HashMap<(i32, i32, i32), Vec<VisitorId>>,
    cell: (i32, i32, i32),
) -> usize {
    density.get(&cell).map(|bucket| bucket.len()).unwrap_or(0)
}

pub(super) fn compute_speed(
    park_map: &ParkMap,
    local_density: usize,
    cell: (i32, i32, i32),
) -> f32 {
    let base_speed = park_map
        .get_infrastructure(cell.0, cell.1, cell.2)
        .map(base_speed_for)
        .unwrap_or(0.0);
    speed_at(base_speed, local_density)
}

pub(super) fn compute_repulsion(
    v: &Visitor,
    density: &HashMap<(i32, i32, i32), Vec<VisitorId>>,
    positions: &HashMap<VisitorId, (f32, f32, f32)>,
    cell: (i32, i32, i32),
) -> (f32, f32, f32) {
    let mut repulsion = (0.0, 0.0, 0.0);
    for dx in -1..=1 {
        for dy in -1..=1 {
            let neighbor_cell = (cell.0 + dx, cell.1 + dy, cell.2);
            if let Some(bucket) = density.get(&neighbor_cell) {
                for other_id in bucket.iter().take(DENSITY_CAP) {
                    if *other_id == v.id {
                        continue;
                    }
                    if let Some(&other_pos) = positions.get(other_id) {
                        let force = repulsion_force(v.position, other_pos);
                        repulsion.0 += force.0;
                        repulsion.1 += force.1;
                        repulsion.2 += force.2;
                    }
                }
            }
        }
    }
    repulsion
}

pub(super) fn compute_lane_bias(
    v: &Visitor,
    park_map: &ParkMap,
    density: &HashMap<(i32, i32, i32), Vec<VisitorId>>,
    cell: (i32, i32, i32),
) -> (f32, f32, f32) {
    let Some(&next) = v.path.first() else {
        return (0.0, 0.0, 0.0);
    };
    let next_f = (next.0 as f32, next.1 as f32, next.2 as f32);
    let forward = crate::visitor::direction(v.position, next_f);
    let Some(perp) = perpendicular_of(forward) else {
        return (0.0, 0.0, 0.0);
    };

    let left_cell = (
        cell.0 + perp.0.round() as i32,
        cell.1 + perp.1.round() as i32,
        cell.2,
    );
    let right_cell = (
        cell.0 - perp.0.round() as i32,
        cell.1 - perp.1.round() as i32,
        cell.2,
    );

    let density_if_walkable = |c: (i32, i32, i32)| -> Option<usize> {
        park_map
            .is_walkable(c.0, c.1, c.2)
            .then(|| local_density_at(density, c))
    };

    let strength = lane_bias_strength(
        density_if_walkable(left_cell),
        density_if_walkable(right_cell),
    );
    (perp.0 * strength, perp.1 * strength, 0.0)
}

/// Anticipatory counterpart to `compute_lane_bias`: scans ahead on `path` for a jam and
/// steers toward the less-congested parallel cell, without touching `path` itself.
pub(super) fn compute_detour_bias(
    v: &Visitor,
    park_map: &ParkMap,
    density: &HashMap<(i32, i32, i32), Vec<VisitorId>>,
) -> (f32, f32, f32) {
    let Some(&next) = v.path.first() else {
        return (0.0, 0.0, 0.0);
    };
    let next_f = (next.0 as f32, next.1 as f32, next.2 as f32);
    let forward = crate::visitor::direction(v.position, next_f);
    let Some(perp) = perpendicular_of(forward) else {
        return (0.0, 0.0, 0.0);
    };
    let perp_step = (perp.0.round() as i32, perp.1.round() as i32);

    let density_if_walkable = |c: (i32, i32, i32)| -> Option<usize> {
        park_map
            .is_walkable(c.0, c.1, c.2)
            .then(|| local_density_at(density, c))
    };

    let mut bias = 0.0;
    for (i, &ahead) in v.path.iter().take(DETOUR_LOOKAHEAD_CELLS).enumerate() {
        if local_density_at(density, ahead) < DETOUR_DENSITY_THRESHOLD {
            continue;
        }
        let left = (ahead.0 + perp_step.0, ahead.1 + perp_step.1, ahead.2);
        let right = (ahead.0 - perp_step.0, ahead.1 - perp_step.1, ahead.2);
        let weight = 1.0 / (i + 1) as f32; // closer congestion steers harder
        bias += weighted_lane_bias(
            density_if_walkable(left),
            density_if_walkable(right),
            DETOUR_BIAS_STRENGTH,
        ) * weight;
    }

    (perp.0 * bias, perp.1 * bias, 0.0)
}

pub(super) fn update_density_and_dirty_chunks(
    density: &mut HashMap<(i32, i32, i32), Vec<VisitorId>>,
    dirty_chunks: &mut HashSet<(i32, i32)>,
    visitor_id: &VisitorId,
    old_cell: (i32, i32, i32),
    new_cell: (i32, i32, i32),
) {
    if new_cell == old_cell {
        return;
    }
    if let Some(bucket) = density.get_mut(&old_cell) {
        bucket.retain(|id| id != visitor_id);
        if bucket.is_empty() {
            density.remove(&old_cell);
        }
    }
    density
        .entry(new_cell)
        .or_default()
        .push(visitor_id.clone());
    dirty_chunks.insert((new_cell.0, new_cell.1));
}

#[cfg(test)]
mod tests {
    use super::super::distance_moved;
    use super::*;
    use crate::balance::{STALL_TICKS_THRESHOLD, UNSTALL_IMPULSE_MAGNITUDE};
    use crate::map::{Bounds3d, InfrastructureShape, ParkMap};

    mod clamp_to_walkable_ground {
        use super::*;

        fn visitor_at(position: (f32, f32, f32)) -> Visitor {
            Visitor {
                id: "a".into(),
                position,
                ..Default::default()
            }
        }

        #[test]
        fn test_does_nothing_when_the_full_move_is_still_walkable() {
            let mut park_map = ParkMap::new("m".into(), Bounds3d::new(0, 5, 0, 5, -1, 1));
            park_map.set_infrastructure(0, 0, 0, InfrastructureShape::Path);
            let mut v = visitor_at((0.2, 0.1, 0.0)); // rounds to (0,0,0), still on the path

            clamp_to_walkable_ground(&mut v, &park_map, (0.0, 0.0, 0.0));

            assert_eq!(v.position, (0.2, 0.1, 0.0));
        }

        #[test]
        fn test_slides_along_x_when_the_y_progress_alone_would_leave_the_path() {
            // Corridor along x (0,0,0)-(1,0,0): the tick's x progress is legitimate
            // forward movement, the y drift is lateral repulsion pushing off the edge.
            // The full move rounds to (1,1,0), never set walkable — but x alone
            // (1,0,0) is, so the visitor should keep that forward progress.
            let mut park_map = ParkMap::new("m".into(), Bounds3d::new(0, 5, 0, 5, -1, 1));
            park_map.set_infrastructure(0, 0, 0, InfrastructureShape::Path);
            park_map.set_infrastructure(1, 0, 0, InfrastructureShape::Path);
            let mut v = visitor_at((0.8, 0.6, 0.0));

            clamp_to_walkable_ground(&mut v, &park_map, (0.0, 0.0, 0.0));

            assert_eq!(v.position, (0.8, 0.0, 0.0)); // kept the x progress, y clamped back
        }

        #[test]
        fn test_slides_along_y_when_the_x_progress_alone_would_leave_the_path() {
            // Same idea, corridor along y this time: full move rounds to (1,1,0),
            // unwalkable; y alone (0,1,0) is walkable, x alone (1,0,0) isn't.
            let mut park_map = ParkMap::new("m".into(), Bounds3d::new(0, 5, 0, 5, -1, 1));
            park_map.set_infrastructure(0, 0, 0, InfrastructureShape::Path);
            park_map.set_infrastructure(0, 1, 0, InfrastructureShape::Path);
            let mut v = visitor_at((0.6, 0.8, 0.0));

            clamp_to_walkable_ground(&mut v, &park_map, (0.0, 0.0, 0.0));

            assert_eq!(v.position, (0.0, 0.8, 0.0)); // kept the y progress, x clamped back
        }

        #[test]
        fn test_reverts_fully_when_neither_axis_alone_is_walkable() {
            // Only (0,0,0) is walkable: both (1,0,0) and (0,1,0) are off-path.
            let mut park_map = ParkMap::new("m".into(), Bounds3d::new(0, 5, 0, 5, -1, 1));
            park_map.set_infrastructure(0, 0, 0, InfrastructureShape::Path);
            let mut v = visitor_at((0.6, 0.6, 0.0));

            clamp_to_walkable_ground(&mut v, &park_map, (0.1, 0.1, 0.0));

            assert_eq!(v.position, (0.1, 0.1, 0.0));
        }
    }

    mod update_stall_tracking {
        use super::*;

        fn visitor_with_path() -> Visitor {
            Visitor {
                id: "a".into(),
                position: (0.0, 0.0, 0.0),
                path: vec![(1, 0, 0)],
                ..Default::default()
            }
        }

        #[test]
        fn test_increments_stall_ticks_when_speed_is_positive_but_movement_is_negligible() {
            let park_map = ParkMap::new("m".into(), Bounds3d::new(0, 5, 0, 5, -1, 1));
            let mut v = visitor_with_path();

            update_stall_tracking(&mut v, &park_map, 1.0, 0.0);

            assert_eq!(v.stall_ticks, 1);
        }

        #[test]
        fn test_resets_stall_ticks_on_real_progress() {
            let park_map = ParkMap::new("m".into(), Bounds3d::new(0, 5, 0, 5, -1, 1));
            let mut v = visitor_with_path();
            v.stall_ticks = 5;

            update_stall_tracking(&mut v, &park_map, 1.0, 1.0);

            assert_eq!(v.stall_ticks, 0);
        }

        #[test]
        fn test_resets_stall_ticks_when_speed_is_zero() {
            // A visitor with no infrastructure under them (speed 0) isn't "stalled by
            // another visitor" — that's a different, already-handled case.
            let park_map = ParkMap::new("m".into(), Bounds3d::new(0, 5, 0, 5, -1, 1));
            let mut v = visitor_with_path();
            v.stall_ticks = 5;

            update_stall_tracking(&mut v, &park_map, 0.0, 0.0);

            assert_eq!(v.stall_ticks, 0);
        }

        #[test]
        fn test_does_not_count_a_visitor_with_no_path_as_stalled() {
            let park_map = ParkMap::new("m".into(), Bounds3d::new(0, 5, 0, 5, -1, 1));
            let mut v = visitor_with_path();
            v.path = vec![];
            v.stall_ticks = 5;

            update_stall_tracking(&mut v, &park_map, 1.0, 0.0);

            assert_eq!(v.stall_ticks, 0);
        }

        #[test]
        fn test_applies_an_impulse_and_resets_the_counter_once_the_threshold_is_reached() {
            let mut park_map = ParkMap::new("m".into(), Bounds3d::new(0, 5, 0, 5, -1, 1));
            park_map.set_infrastructure(0, 0, 0, InfrastructureShape::Path);
            let mut v = visitor_with_path();
            v.stall_ticks = STALL_TICKS_THRESHOLD - 1;

            update_stall_tracking(&mut v, &park_map, 1.0, 0.0);

            assert_eq!(v.stall_ticks, 0);
        }
    }

    mod apply_unstall_impulse {
        use super::*;

        #[test]
        fn test_moves_within_the_impulse_magnitude_when_the_nudge_stays_walkable() {
            // A wide-open walkable area: any angle keeps the candidate cell walkable.
            let mut park_map = ParkMap::new("m".into(), Bounds3d::new(-5, 5, -5, 5, -1, 1));
            for x in -2..=2 {
                for y in -2..=2 {
                    park_map.set_infrastructure(x, y, 0, InfrastructureShape::Path);
                }
            }
            let mut v = Visitor {
                id: "a".into(),
                position: (0.0, 0.0, 0.0),
                ..Default::default()
            };

            apply_unstall_impulse(&mut v, &park_map);

            let moved = distance_moved((0.0, 0.0, 0.0), v.position);
            assert!(moved > 0.0, "the visitor should have been nudged");
            assert!(moved <= UNSTALL_IMPULSE_MAGNITUDE + 1e-5);
        }

        #[test]
        fn test_never_commits_a_move_that_leaves_walkable_ground_even_near_an_edge() {
            // Only (0,0,0) is walkable — (1,0,0) deliberately isn't. Starting close to
            // that edge means some (but not all) random angles would cross it; run
            // enough trials to exercise both the commit and the rejection branch, and
            // check the invariant holds every time rather than asserting one outcome.
            let mut park_map = ParkMap::new("m".into(), Bounds3d::new(0, 5, 0, 5, -1, 1));
            park_map.set_infrastructure(0, 0, 0, InfrastructureShape::Path);

            for _ in 0..200 {
                let mut v = Visitor {
                    id: "a".into(),
                    position: (0.42, 0.0, 0.0),
                    ..Default::default()
                };

                apply_unstall_impulse(&mut v, &park_map);

                let cell = (
                    v.position.0.round() as i32,
                    v.position.1.round() as i32,
                    v.position.2.round() as i32,
                );
                assert!(
                    park_map.is_walkable(cell.0, cell.1, cell.2),
                    "landed at {:?}",
                    v.position
                );
            }
        }
    }

    mod compute_repulsion {
        use super::*;

        fn visitor_at(id: &str, position: (f32, f32, f32)) -> Visitor {
            Visitor {
                id: id.into(),
                position,
                path: vec![],
                target: (0, 0, 0),
                ticks_since_spawn: 0,
                heading: (0.0, 0.0, 0.0),
                is_leaving: false,
                ..Default::default()
            }
        }

        #[test]
        fn test_ignores_neighbors_beyond_density_cap_in_the_same_bucket() {
            let v = visitor_at("a", (0.0, 0.0, 0.0));

            let mut positions: HashMap<VisitorId, (f32, f32, f32)> = HashMap::new();
            let mut bucket_at_cap: Vec<VisitorId> = Vec::new();
            for i in 0..DENSITY_CAP {
                let id = format!("n{i}");
                positions.insert(id.clone(), (0.1, 0.0, 0.0));
                bucket_at_cap.push(id);
            }

            // Same first DENSITY_CAP neighbors, plus extra ones packed into the same cell.
            let mut bucket_over_cap = bucket_at_cap.clone();
            for i in DENSITY_CAP..(DENSITY_CAP + 10) {
                let id = format!("n{i}");
                positions.insert(id.clone(), (0.1, 0.0, 0.0));
                bucket_over_cap.push(id);
            }

            let mut density_at_cap: HashMap<(i32, i32, i32), Vec<VisitorId>> = HashMap::new();
            density_at_cap.insert((0, 0, 0), bucket_at_cap);

            let mut density_over_cap: HashMap<(i32, i32, i32), Vec<VisitorId>> = HashMap::new();
            density_over_cap.insert((0, 0, 0), bucket_over_cap);

            let repulsion_at_cap = compute_repulsion(&v, &density_at_cap, &positions, (0, 0, 0));
            let repulsion_over_cap =
                compute_repulsion(&v, &density_over_cap, &positions, (0, 0, 0));

            assert_eq!(
                repulsion_at_cap, repulsion_over_cap,
                "neighbors beyond DENSITY_CAP in the same bucket should not affect the result"
            );
        }
    }

    mod compute_detour_bias {
        use super::*;

        fn visitor_with_path(path: Vec<(i32, i32, i32)>) -> Visitor {
            Visitor {
                id: "a".into(),
                position: (0.0, 0.0, 0.0),
                path,
                target: (0, 0, 0),
                ..Default::default()
            }
        }

        #[test]
        fn test_zero_bias_when_the_visitor_has_no_path() {
            let park_map = ParkMap::new("m".into(), Bounds3d::new(0, 5, 0, 5, -1, 1));
            let v = visitor_with_path(vec![]);
            let density: HashMap<(i32, i32, i32), Vec<VisitorId>> = HashMap::new();

            let bias = compute_detour_bias(&v, &park_map, &density);

            assert_eq!(bias, (0.0, 0.0, 0.0));
        }

        #[test]
        fn test_zero_bias_when_nothing_ahead_reaches_the_congestion_threshold() {
            let park_map = ParkMap::new("m".into(), Bounds3d::new(0, 5, 0, 5, -1, 1));
            let v = visitor_with_path(vec![(1, 0, 0), (2, 0, 0), (3, 0, 0)]);
            let mut density: HashMap<(i32, i32, i32), Vec<VisitorId>> = HashMap::new();
            density.insert((2, 0, 0), vec!["n0".into(), "n1".into()]); // below DETOUR_DENSITY_THRESHOLD (3)

            let bias = compute_detour_bias(&v, &park_map, &density);

            assert_eq!(bias, (0.0, 0.0, 0.0));
        }

        #[test]
        fn test_steers_toward_the_walkable_side_when_a_cell_ahead_is_congested_and_the_other_side_is_blocked()
         {
            // Straight corridor along x. A jam forms two cells ahead at (2,0,0): the
            // parallel cell on the left, (2,1,0), is open ground; the right, (2,-1,0),
            // was never set walkable at all.
            let mut park_map = ParkMap::new("m".into(), Bounds3d::new(0, 5, -5, 5, -1, 1));
            park_map.set_infrastructure(1, 0, 0, InfrastructureShape::Path);
            park_map.set_infrastructure(2, 0, 0, InfrastructureShape::Path);
            park_map.set_infrastructure(3, 0, 0, InfrastructureShape::Path);
            park_map.set_infrastructure(2, 1, 0, InfrastructureShape::Path);

            let v = visitor_with_path(vec![(1, 0, 0), (2, 0, 0), (3, 0, 0)]);
            let mut density: HashMap<(i32, i32, i32), Vec<VisitorId>> = HashMap::new();
            density.insert(
                (2, 0, 0),
                vec!["n0".into(), "n1".into(), "n2".into()], // at DETOUR_DENSITY_THRESHOLD
            );

            let bias = compute_detour_bias(&v, &park_map, &density);

            assert!(
                bias.1 > 0.0,
                "should steer toward +y (left/open side), got {bias:?}"
            );
        }

        #[test]
        fn test_zero_bias_when_the_only_open_parallel_cell_is_just_as_congested() {
            let mut park_map = ParkMap::new("m".into(), Bounds3d::new(0, 5, -5, 5, -1, 1));
            park_map.set_infrastructure(1, 0, 0, InfrastructureShape::Path);
            park_map.set_infrastructure(2, 0, 0, InfrastructureShape::Path);
            park_map.set_infrastructure(3, 0, 0, InfrastructureShape::Path);
            park_map.set_infrastructure(2, 1, 0, InfrastructureShape::Path);
            park_map.set_infrastructure(2, -1, 0, InfrastructureShape::Path);

            let v = visitor_with_path(vec![(1, 0, 0), (2, 0, 0), (3, 0, 0)]);
            let mut density: HashMap<(i32, i32, i32), Vec<VisitorId>> = HashMap::new();
            let jam = vec!["n0".into(), "n1".into(), "n2".into()];
            density.insert((2, 0, 0), jam.clone());
            density.insert((2, 1, 0), jam.clone());
            density.insert((2, -1, 0), jam);

            let bias = compute_detour_bias(&v, &park_map, &density);

            assert_eq!(bias, (0.0, 0.0, 0.0));
        }

        #[test]
        fn test_closer_congestion_steers_harder_than_farther_congestion() {
            fn corridor() -> ParkMap {
                let mut park_map = ParkMap::new("m".into(), Bounds3d::new(0, 5, -5, 5, -1, 1));
                for x in 1..=5 {
                    park_map.set_infrastructure(x, 0, 0, InfrastructureShape::Path);
                    park_map.set_infrastructure(x, 1, 0, InfrastructureShape::Path);
                }
                park_map
            }
            let near = corridor();
            let far = corridor();

            let path = vec![(1, 0, 0), (2, 0, 0), (3, 0, 0), (4, 0, 0), (5, 0, 0)];
            let v_near = visitor_with_path(path.clone());
            let v_far = visitor_with_path(path);

            let jam = vec!["n0".into(), "n1".into(), "n2".into()];
            let mut density_near: HashMap<(i32, i32, i32), Vec<VisitorId>> = HashMap::new();
            density_near.insert((1, 0, 0), jam.clone()); // 1st cell ahead
            let mut density_far: HashMap<(i32, i32, i32), Vec<VisitorId>> = HashMap::new();
            density_far.insert((5, 0, 0), jam); // last cell in the lookahead window

            let bias_near = compute_detour_bias(&v_near, &near, &density_near);
            let bias_far = compute_detour_bias(&v_far, &far, &density_far);

            assert!(
                bias_near.1 > bias_far.1,
                "congestion right ahead ({bias_near:?}) should steer harder than the same congestion \
                 further out ({bias_far:?})"
            );
        }
    }

    mod update_density_and_dirty_chunks {
        use super::*;

        #[test]
        fn test_does_nothing_when_cell_is_unchanged() {
            let mut density: HashMap<(i32, i32, i32), Vec<VisitorId>> = HashMap::new();
            density.insert((0, 0, 0), vec!["a".into()]);
            let mut dirty_chunks = HashSet::new();

            update_density_and_dirty_chunks(
                &mut density,
                &mut dirty_chunks,
                &"a".to_string(),
                (0, 0, 0),
                (0, 0, 0),
            );

            assert_eq!(density.get(&(0, 0, 0)), Some(&vec!["a".to_string()]));
            assert!(dirty_chunks.is_empty());
        }

        #[test]
        fn test_moves_visitor_to_new_bucket_when_cell_changes() {
            let mut density: HashMap<(i32, i32, i32), Vec<VisitorId>> = HashMap::new();
            density.insert((0, 0, 0), vec!["a".into()]);
            let mut dirty_chunks = HashSet::new();

            update_density_and_dirty_chunks(
                &mut density,
                &mut dirty_chunks,
                &"a".to_string(),
                (0, 0, 0),
                (1, 0, 0),
            );

            assert_eq!(density.get(&(1, 0, 0)), Some(&vec!["a".to_string()]));
        }

        #[test]
        fn test_removes_old_bucket_when_it_becomes_empty() {
            let mut density: HashMap<(i32, i32, i32), Vec<VisitorId>> = HashMap::new();
            density.insert((0, 0, 0), vec!["a".into()]);
            let mut dirty_chunks = HashSet::new();

            update_density_and_dirty_chunks(
                &mut density,
                &mut dirty_chunks,
                &"a".to_string(),
                (0, 0, 0),
                (1, 0, 0),
            );

            assert!(!density.contains_key(&(0, 0, 0)));
        }

        #[test]
        fn test_keeps_old_bucket_when_other_visitors_remain() {
            let mut density: HashMap<(i32, i32, i32), Vec<VisitorId>> = HashMap::new();
            density.insert((0, 0, 0), vec!["a".into(), "b".into()]);
            let mut dirty_chunks = HashSet::new();

            update_density_and_dirty_chunks(
                &mut density,
                &mut dirty_chunks,
                &"a".to_string(),
                (0, 0, 0),
                (1, 0, 0),
            );

            assert_eq!(density.get(&(0, 0, 0)), Some(&vec!["b".to_string()]));
        }

        #[test]
        fn test_marks_new_chunk_as_dirty_when_cell_changes() {
            let mut density: HashMap<(i32, i32, i32), Vec<VisitorId>> = HashMap::new();
            density.insert((0, 0, 0), vec!["a".into()]);
            let mut dirty_chunks = HashSet::new();

            update_density_and_dirty_chunks(
                &mut density,
                &mut dirty_chunks,
                &"a".to_string(),
                (0, 0, 0),
                (1, 0, 0),
            );

            assert!(dirty_chunks.contains(&(1, 0)));
        }
    }
}
