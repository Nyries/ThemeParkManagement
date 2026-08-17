use rand::Rng;

use crate::balance::{
    AVOIDING_RADIUS, DENSITY_CAP, K_REPULSION, LANE_BIAS_STRENGTH, LATERAL_REPULSION_FACTOR,
    LATERAL_REPULSION_FACTOR_AT_MAX_DENSITY, SPEED_FLOOR_AT_MAX_DENSITY,
};

pub(super) fn distance(a: (f32, f32, f32), b: (f32, f32, f32)) -> f32 {
    let dx = b.0 - a.0;
    let dy = b.1 - a.1;
    let dz = b.2 - a.2;
    (dx * dx + dy * dy + dz * dz).sqrt()
}

pub(crate) fn direction(from: (f32, f32, f32), to: (f32, f32, f32)) -> (f32, f32, f32) {
    let d = distance(from, to);
    if d == 0.0 {
        return (0.0, 0.0, 0.0);
    }
    (
        (to.0 - from.0) / d,
        (to.1 - from.1) / d,
        (to.2 - from.2) / d,
    )
}

pub(super) fn normalize(v: (f32, f32, f32)) -> (f32, f32, f32) {
    let len = (v.0 * v.0 + v.1 * v.1 + v.2 * v.2).sqrt();
    if len == 0.0 {
        return (0.0, 0.0, 0.0);
    }
    (v.0 / len, v.1 / len, v.2 / len)
}

fn dot(a: (f32, f32, f32), b: (f32, f32, f32)) -> f32 {
    a.0 * b.0 + a.1 * b.1 + a.2 * b.2
}

pub(super) fn atternuate_lateral_repulsion(
    repulsion: (f32, f32, f32),
    forward: (f32, f32, f32),
    lateral_factor: f32,
) -> (f32, f32, f32) {
    if forward == (0.0, 0.0, 0.0) {
        return repulsion;
    }
    let along = dot(repulsion, forward);
    let parallel = (forward.0 * along, forward.1 * along, forward.2 * along);
    let lateral = (
        repulsion.0 - parallel.0,
        repulsion.1 - parallel.1,
        repulsion.2 - parallel.2,
    );
    (
        parallel.0 + lateral.0 * lateral_factor,
        parallel.1 + lateral.1 * lateral_factor,
        parallel.2 + lateral.2 * lateral_factor,
    )
}

/// Lateral steering strength, scaled by local crowding (low at low density, ramps
/// toward `LATERAL_REPULSION_FACTOR_AT_MAX_DENSITY` near `DENSITY_CAP`).
pub fn lateral_repulsion_factor_for(density: usize) -> f32 {
    let t = (density as f32 / DENSITY_CAP as f32).min(1.0);
    LATERAL_REPULSION_FACTOR
        + (LATERAL_REPULSION_FACTOR_AT_MAX_DENSITY - LATERAL_REPULSION_FACTOR) * t
}

/// Perpendicular to a direction of travel, in the horizontal plane. `None` if `forward`
/// is zero (no direction yet).
pub fn perpendicular_of(forward: (f32, f32, f32)) -> Option<(f32, f32, f32)> {
    if forward == (0.0, 0.0, 0.0) {
        return None;
    }
    Some((-forward.1, forward.0, 0.0))
}

/// Signed bias toward whichever side is less crowded; `None` density means unwalkable.
pub(crate) fn weighted_lane_bias(
    left_density: Option<usize>,
    right_density: Option<usize>,
    max_strength: f32,
) -> f32 {
    let (Some(left), Some(right)) = (left_density, right_density) else {
        // At most one side is walkable; steer toward it if it exists, otherwise no bias.
        return match (left_density, right_density) {
            (Some(_), None) => max_strength,
            (None, Some(_)) => -max_strength,
            _ => 0.0,
        };
    };
    let diff = (right as f32 - left as f32) / DENSITY_CAP as f32;
    max_strength * diff.clamp(-1.0, 1.0)
}

pub fn lane_bias_strength(left_density: Option<usize>, right_density: Option<usize>) -> f32 {
    weighted_lane_bias(left_density, right_density, LANE_BIAS_STRENGTH)
}

pub(super) fn lerp_direction(
    current: (f32, f32, f32),
    desired: (f32, f32, f32),
    factor: f32,
) -> (f32, f32, f32) {
    let blended = (
        current.0 + (desired.0 - current.0) * factor,
        current.1 + (desired.1 - current.1) * factor,
        current.2 + (desired.2 - current.2) * factor,
    );
    normalize(blended)
}

/// Speed drops linearly with density, floored at `SPEED_FLOOR_AT_MAX_DENSITY` to avoid
/// a permanent gridlock (a stalled visitor never leaves its cell to relieve density).
pub fn speed_at(base_speed: f32, density: usize) -> f32 {
    let f = (1.0 - density as f32 / DENSITY_CAP as f32).max(SPEED_FLOOR_AT_MAX_DENSITY);
    base_speed * f
}

pub fn repulsion_force(
    my_position: (f32, f32, f32),
    neighbor_position: (f32, f32, f32),
) -> (f32, f32, f32) {
    let d = distance(my_position, neighbor_position);
    if d >= AVOIDING_RADIUS {
        return (0.0, 0.0, 0.0);
    }
    let intensity = K_REPULSION * (AVOIDING_RADIUS - d) / AVOIDING_RADIUS;

    if d == 0.0 {
        // Exact overlap has no defined direction to push apart; use a random one.
        let angle: f32 = rand::thread_rng().gen_range(0.0..std::f32::consts::TAU);
        return (intensity * angle.cos(), intensity * angle.sin(), 0.0);
    }

    let (dx, dy, dz) = direction(neighbor_position, my_position);
    (intensity * dx, intensity * dy, intensity * dz)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_distance_from_two_points() {
        let a = (0.0, 0.0, 0.0);
        let b = (3.0, 2.0, 2.0);
        let expected = (3.0_f32 * 3.0 + 2.0 * 2.0 + 2.0 * 2.0).sqrt(); // = 17.0_f32.sqrt() ≈ 4.1231

        let result = distance(a, b);

        assert_eq!(result, expected);
    }

    #[test]
    fn test_direction_when_same_point() {
        let a = (3.0, 2.0, 2.0);

        let result = direction(a, a);

        assert_eq!(result, (0.0, 0.0, 0.0))
    }

    #[test]
    fn test_direction_when_different_point() {
        let a = (0.0, 0.0, 0.0);
        let b = (3.0, 2.0, 2.0);
        let d = 17.0_f32.sqrt();
        let expected = (3.0 / d, 2.0 / d, 2.0 / d);

        let result = direction(a, b);

        assert_eq!(result, expected)
    }

    #[test]
    fn test_speed_at() {
        let base_speed = 1.0;
        let density = 2.0;
        let expected = 0.6; // (1.0 - 2.0/5.0).max(FLOOR) * 1.0 = 0.6 * 1.0

        let result = speed_at(base_speed, density as usize);

        assert_eq!(result, expected);
    }

    #[test]
    fn test_speed_at_never_reaches_zero_at_or_above_the_density_cap() {
        // Regression: without a floor, density >= DENSITY_CAP made speed exactly 0
        // forever — a visitor that never moves never leaves its cell, so density
        // there can only grow, never recover. Confirmed empirically to gridlock.
        let at_cap = speed_at(1.0, DENSITY_CAP);
        let above_cap = speed_at(1.0, DENSITY_CAP * 10);

        assert_eq!(at_cap, SPEED_FLOOR_AT_MAX_DENSITY);
        assert_eq!(above_cap, SPEED_FLOOR_AT_MAX_DENSITY);
        assert!(at_cap > 0.0);
    }

    mod lateral_repulsion_factor_for {
        use super::*;

        #[test]
        fn test_stays_at_the_baseline_factor_with_no_crowd() {
            let result = lateral_repulsion_factor_for(0);

            assert_eq!(result, LATERAL_REPULSION_FACTOR);
        }

        #[test]
        fn test_reaches_the_max_density_factor_at_the_cap() {
            let result = lateral_repulsion_factor_for(DENSITY_CAP);

            assert_eq!(result, LATERAL_REPULSION_FACTOR_AT_MAX_DENSITY);
        }

        #[test]
        fn test_never_exceeds_the_max_density_factor_beyond_the_cap() {
            let result = lateral_repulsion_factor_for(DENSITY_CAP * 10);

            assert_eq!(result, LATERAL_REPULSION_FACTOR_AT_MAX_DENSITY);
        }

        #[test]
        fn test_grows_monotonically_between_the_baseline_and_the_cap() {
            let low = lateral_repulsion_factor_for(1);
            let mid = lateral_repulsion_factor_for(DENSITY_CAP / 2);
            let high = lateral_repulsion_factor_for(DENSITY_CAP - 1);

            assert!(LATERAL_REPULSION_FACTOR < low);
            assert!(low < mid);
            assert!(mid < high);
            assert!(high < LATERAL_REPULSION_FACTOR_AT_MAX_DENSITY);
        }
    }

    mod perpendicular_of {
        use super::*;

        #[test]
        fn test_none_when_forward_has_no_direction() {
            let result = perpendicular_of((0.0, 0.0, 0.0));

            assert_eq!(result, None);
        }

        #[test]
        fn test_rotates_ninety_degrees_in_the_horizontal_plane() {
            let result = perpendicular_of((1.0, 0.0, 0.0));

            assert_eq!(result, Some((0.0, 1.0, 0.0)));
        }

        #[test]
        fn test_stays_perpendicular_to_forward() {
            let forward = (1.0, 0.0, 0.0);
            let perp = perpendicular_of(forward).unwrap();

            assert_eq!(dot(forward, perp), 0.0);
        }
    }

    mod lane_bias_strength {
        use super::*;

        #[test]
        fn test_zero_when_both_sides_equally_crowded() {
            let result = lane_bias_strength(Some(2), Some(2));

            assert_eq!(result, 0.0);
        }

        #[test]
        fn test_positive_toward_the_less_crowded_left_side() {
            let result = lane_bias_strength(Some(0), Some(DENSITY_CAP));

            assert!(result > 0.0);
        }

        #[test]
        fn test_negative_toward_the_less_crowded_right_side() {
            let result = lane_bias_strength(Some(DENSITY_CAP), Some(0));

            assert!(result < 0.0);
        }

        #[test]
        fn test_steers_toward_the_only_walkable_side() {
            let toward_left = lane_bias_strength(Some(0), None);
            let toward_right = lane_bias_strength(None, Some(0));

            assert_eq!(toward_left, LANE_BIAS_STRENGTH);
            assert_eq!(toward_right, -LANE_BIAS_STRENGTH);
        }

        #[test]
        fn test_zero_when_neither_side_is_walkable() {
            let result = lane_bias_strength(None, None);

            assert_eq!(result, 0.0);
        }
    }

    fn assert_close(actual: (f32, f32, f32), expected: (f32, f32, f32)) {
        const EPSILON: f32 = 1e-5;
        assert!(
            (actual.0 - expected.0).abs() < EPSILON,
            "x: {} != {}",
            actual.0,
            expected.0
        );
        assert!(
            (actual.1 - expected.1).abs() < EPSILON,
            "y: {} != {}",
            actual.1,
            expected.1
        );
        assert!(
            (actual.2 - expected.2).abs() < EPSILON,
            "z: {} != {}",
            actual.2,
            expected.2
        );
    }

    #[test]
    fn test_repulsion_force_is_zero_beyond_avoiding_radius() {
        let force = repulsion_force((0.0, 0.0, 0.0), (1.0, 0.0, 0.0));
        assert_close(force, (0.0, 0.0, 0.0));
    }

    #[test]
    fn test_repulsion_force_is_zero_exactly_at_avoiding_radius() {
        let force = repulsion_force((0.0, 0.0, 0.0), (0.2, 0.0, 0.0));
        assert_close(force, (0.0, 0.0, 0.0));
    }

    #[test]
    fn test_repulsion_force_pushes_away_from_close_neighbor() {
        let force = repulsion_force((0.0, 0.0, 0.0), (0.1, 0.0, 0.0));
        assert_close(force, (-0.1, 0.0, 0.0));
    }

    #[test]
    fn test_repulsion_force_pushes_apart_at_full_intensity_when_visitors_overlap_exactly() {
        // Regression: `direction()` is undefined at zero distance, so this used to
        // return (0,0,0) — a permanently stable merged state, two visitor sprites
        // stuck on top of each other forever since nothing would ever separate them.
        // The exact direction is randomized; only the magnitude is deterministic.
        for _ in 0..50 {
            let force = repulsion_force((0.0, 0.0, 0.0), (0.0, 0.0, 0.0));
            let magnitude = (force.0 * force.0 + force.1 * force.1 + force.2 * force.2).sqrt();
            assert!(
                (magnitude - K_REPULSION).abs() < 1e-5,
                "expected max intensity ({K_REPULSION}), got {magnitude}"
            );
            assert_eq!(
                force.2, 0.0,
                "overlap push-apart stays on the horizontal plane"
            );
        }
    }
}
