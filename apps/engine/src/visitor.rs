use crate::balance::{
    AVOIDING_RADIUS, DENSITY_CAP, K_REPULSION, LATERAL_REPULSION_FACTOR, STEERING_FACTOR,
    VISIT_DURATION_TICKS,
};

pub type VisitorId = String;

pub struct Visitor {
    pub id: VisitorId,
    pub position: (f32, f32, f32),
    pub path: Vec<(i32, i32, i32)>,
    pub target: (i32, i32, i32),
    pub ticks_since_spawn: u64,
    pub heading: (f32, f32, f32),
    pub is_leaving: bool,
}

fn distance(a: (f32, f32, f32), b: (f32, f32, f32)) -> f32 {
    let dx = b.0 - a.0;
    let dy = b.1 - a.1;
    let dz = b.2 - a.2;
    (dx * dx + dy * dy + dz * dz).sqrt()
}

fn direction(from: (f32, f32, f32), to: (f32, f32, f32)) -> (f32, f32, f32) {
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

fn normalize(v: (f32, f32, f32)) -> (f32, f32, f32) {
    let len = (v.0 * v.0 + v.1 * v.1 + v.2 * v.2).sqrt();
    if len == 0.0 {
        return (0.0, 0.0, 0.0);
    }
    (v.0 / len, v.1 / len, v.2 / len)
}

fn dot(a: (f32, f32, f32), b: (f32, f32, f32)) -> f32 {
    a.0 * b.0 + a.1 * b.1 + a.2 * b.2
}

fn atternuate_lateral_repulsion(
    repulsion: (f32, f32, f32),
    forward: (f32, f32, f32),
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
        parallel.0 + lateral.0 * LATERAL_REPULSION_FACTOR,
        parallel.1 + lateral.1 * LATERAL_REPULSION_FACTOR,
        parallel.2 + lateral.2 * LATERAL_REPULSION_FACTOR,
    )
}

fn lerp_direction(
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

pub fn speed_at(base_speed: f32, density: usize) -> f32 {
    let f = (1.0 - density as f32 / DENSITY_CAP as f32).max(0.0);
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
    let (dx, dy, dz) = direction(neighbor_position, my_position);
    let intensity = K_REPULSION * (AVOIDING_RADIUS - d) / AVOIDING_RADIUS;
    (intensity * dx, intensity * dy, intensity * dz)
}

impl Visitor {
    pub fn has_expired(&self) -> bool {
        self.ticks_since_spawn >= VISIT_DURATION_TICKS
    }

    pub fn advance(&mut self, speed: f32, dt: f32, repulsion: (f32, f32, f32)) {
        let Some(&next) = self.path.first() else {
            return;
        };
        let next_f = (next.0 as f32, next.1 as f32, next.2 as f32);
        let dist_to_next = distance(self.position, next_f);

        let raw_desired = direction(self.position, next_f);
        let attenuated_repulsion = atternuate_lateral_repulsion(repulsion, raw_desired);
        let combined = (
            raw_desired.0 + attenuated_repulsion.0,
            raw_desired.1 + attenuated_repulsion.1,
            raw_desired.2 + attenuated_repulsion.2,
        );
        let desired = normalize(combined);

        self.heading = if self.heading == (0.0, 0.0, 0.0) {
            desired
        } else {
            lerp_direction(self.heading, desired, STEERING_FACTOR)
        };

        let step = speed * dt;

        if step >= dist_to_next {
            self.path.remove(0);
            self.position = next_f;
        } else {
            self.position.0 += self.heading.0 * step;
            self.position.1 += self.heading.1 * step;
            self.position.2 += self.heading.2 * step;
        }
    }
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
        let expected = 0.6; // (1.0 - 2.0/5.0).max(0.0) * 1.0 = 0.6 * 1.0

        let result = speed_at(base_speed, density as usize);

        assert_eq!(result, expected);
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
    fn test_repulsion_force_is_zero_when_visitors_overlap_exactly() {
        let force = repulsion_force((0.0, 0.0, 0.0), (0.0, 0.0, 0.0));
        assert_close(force, (0.0, 0.0, 0.0));
    }

    mod has_expired {
        use super::*;

        #[test]
        fn test_has_expired_is_false_before_visit_duration() {
            let visitor = Visitor {
                id: "v1".into(),
                position: (0.0, 0.0, 0.0),
                path: vec![],
                target: (0, 0, 0),
                ticks_since_spawn: VISIT_DURATION_TICKS - 1,
                heading: (0.0, 0.0, 0.0),
                is_leaving: false,
            };

            assert!(!visitor.has_expired());
        }

        #[test]
        fn test_has_expired_is_true_exactly_at_visit_duration() {
            let visitor = Visitor {
                id: "v1".into(),
                position: (0.0, 0.0, 0.0),
                path: vec![],
                target: (0, 0, 0),
                ticks_since_spawn: VISIT_DURATION_TICKS,
                heading: (0.0, 0.0, 0.0),
                is_leaving: false,
            };

            assert!(visitor.has_expired());
        }

        #[test]
        fn test_has_expired_is_true_after_visit_duration() {
            let visitor = Visitor {
                id: "v1".into(),
                position: (0.0, 0.0, 0.0),
                path: vec![],
                target: (0, 0, 0),
                ticks_since_spawn: VISIT_DURATION_TICKS + 500,
                heading: (0.0, 0.0, 0.0),
                is_leaving: false,
            };

            assert!(visitor.has_expired());
        }
    }

    mod advance {
        use super::*;

        #[test]
        fn test_advance_does_nothing_when_path_is_empty() {
            let mut visitor = Visitor {
                id: "v1".into(),
                position: (1.0, 2.0, 3.0),
                path: vec![],
                target: (1, 2, 3),
                ticks_since_spawn: 0,
                heading: (0.0, 0.0, 0.0),
                is_leaving: false,
            };

            visitor.advance(1.0, 1.0, (0.0, 0.0, 0.0));

            assert_close(visitor.position, (1.0, 2.0, 3.0));
            assert!(visitor.path.is_empty());
        }

        #[test]
        fn test_advance_moves_partway_when_step_is_smaller_than_distance() {
            let mut visitor = Visitor {
                id: "v1".into(),
                position: (0.0, 0.0, 0.0),
                path: vec![(1, 0, 0)],
                target: (1, 0, 0),
                ticks_since_spawn: 0,
                heading: (0.0, 0.0, 0.0),
                is_leaving: false,
            };

            visitor.advance(1.0, 0.3, (0.0, 0.0, 0.0)); // step = 0.3, distance = 1.0

            assert_close(visitor.position, (0.3, 0.0, 0.0));
            assert_eq!(visitor.path, vec![(1, 0, 0)]);
        }

        #[test]
        fn test_advance_snaps_to_next_point_and_pops_path_when_step_reaches_it_exactly() {
            let mut visitor = Visitor {
                id: "v1".into(),
                position: (0.0, 0.0, 0.0),
                path: vec![(1, 0, 0)],
                target: (1, 0, 0),
                ticks_since_spawn: 0,
                heading: (0.0, 0.0, 0.0),
                is_leaving: false,
            };

            visitor.advance(1.0, 1.0, (0.0, 0.0, 0.0)); // step = 1.0 == distance

            assert_close(visitor.position, (1.0, 0.0, 0.0));
            assert!(visitor.path.is_empty());
        }

        #[test]
        fn test_advance_does_not_overshoot_into_remaining_budget_when_step_exceeds_distance() {
            let mut visitor = Visitor {
                id: "v1".into(),
                position: (0.0, 0.0, 0.0),
                path: vec![(1, 0, 0)],
                target: (1, 0, 0),
                ticks_since_spawn: 0,
                heading: (0.0, 0.0, 0.0),
                is_leaving: false,
            };

            visitor.advance(2.0, 1.0, (0.0, 0.0, 0.0)); // step = 2.0, distance = 1.0

            assert_close(visitor.position, (1.0, 0.0, 0.0));
            assert!(visitor.path.is_empty());
        }

        #[test]
        fn test_advance_only_pops_the_reached_point_when_path_has_several() {
            let mut visitor = Visitor {
                id: "v1".into(),
                position: (0.0, 0.0, 0.0),
                path: vec![(1, 0, 0), (2, 0, 0)],
                target: (2, 0, 0),
                ticks_since_spawn: 0,
                heading: (0.0, 0.0, 0.0),
                is_leaving: false,
            };

            visitor.advance(1.0, 1.0, (0.0, 0.0, 0.0));

            assert_close(visitor.position, (1.0, 0.0, 0.0));
            assert_eq!(visitor.path, vec![(2, 0, 0)]);
        }

        #[test]
        fn test_advance_sets_heading_directly_to_desired_direction_on_first_move() {
            let mut visitor = Visitor {
                id: "v1".into(),
                position: (0.0, 0.0, 0.0),
                path: vec![(1, 0, 0)],
                target: (1, 0, 0),
                ticks_since_spawn: 0,
                heading: (0.0, 0.0, 0.0),
                is_leaving: false,
            };

            visitor.advance(0.0, 1.0, (0.0, 0.0, 0.0));
            assert_close(visitor.heading, (1.0, 0.0, 0.0));
        }

        #[test]
        fn test_advance_smooths_heading_toward_desired_direction_on_subsequent_move() {
            let mut visitor = Visitor {
                id: "v1".into(),
                position: (0.0, 0.0, 0.0),
                path: vec![(0, 1, 0)],
                target: (0, 1, 0),
                ticks_since_spawn: 0,
                heading: (1.0, 0.0, 0.0),
                is_leaving: false,
            };

            visitor.advance(0.0, 1.0, (0.0, 0.0, 0.0));

            let expected = {
                let blended = (1.0 - STEERING_FACTOR, STEERING_FACTOR, 0.0);
                let len =
                    (blended.0 * blended.0 + blended.1 * blended.1 + blended.2 * blended.2).sqrt();
                (blended.0 / len, blended.1 / len, blended.2 / len)
            };
            assert_close(visitor.heading, expected);
            assert_ne!(visitor.heading, (1.0, 0.0, 0.0));
            assert_ne!(visitor.heading, (0.0, 1.0, 0.0));
        }

        #[test]
        fn test_advance_heading_stays_a_unit_vector_after_smoothing() {
            let mut visitor = Visitor {
                id: "v1".into(),
                position: (0.0, 0.0, 0.0),
                path: vec![(0, 1, 0)],
                target: (0, 1, 0),
                ticks_since_spawn: 0,
                heading: (1.0, 0.0, 0.0),
                is_leaving: false,
            };

            visitor.advance(0.0, 1.0, (0.0, 0.0, 0.0));

            let len =
                (visitor.heading.0.powi(2) + visitor.heading.1.powi(2) + visitor.heading.2.powi(2))
                    .sqrt();
            assert!(
                (len - 1.0).abs() < 1e-5,
                "heading should remain unitary, length = {len}"
            );
        }
        #[test]
        fn test_advance_repulsion_deflects_movement_away_from_desired_direction() {
            let mut visitor = Visitor {
                id: "v1".into(),
                position: (0.0, 0.0, 0.0),
                path: vec![(10, 0, 0)],
                target: (10, 0, 0),
                ticks_since_spawn: 0,
                heading: (0.0, 0.0, 0.0),
                is_leaving: false,
            };

            visitor.advance(1.0, 0.5, (0.0, 1.0, 0.0)); // répulsion latérale forte

            assert!(visitor.position.1 > 0.0, "repulsion should push lateraly");
            assert!(
                visitor.position.0 < 0.5,
                "x should be reduced by the deviation"
            );
        }

        #[test]
        fn test_advance_movement_magnitude_stays_bounded_to_speed_even_with_strong_repulsion() {
            let mut visitor = Visitor {
                id: "v1".into(),
                position: (0.0, 0.0, 0.0),
                path: vec![(10, 0, 0)],
                target: (10, 0, 0),
                ticks_since_spawn: 0,
                heading: (0.0, 0.0, 0.0),
                is_leaving: false,
            };

            let speed = 1.0;
            let dt = 0.5;
            visitor.advance(speed, dt, (5.0, 5.0, 0.0));

            let moved = distance((0.0, 0.0, 0.0), visitor.position);
            assert!((moved - speed * dt).abs() < 1e-5);
        }

        #[test]
        fn test_advance_keeps_a_visitor_within_a_single_wide_corridor_even_under_strong_lateral_repulsion()
         {
            // A visitor on a corridor one cell wide (only y=0 is walkable) should never be
            // pushed to |y| >= 0.5 by repulsion alone, since that would round onto a
            // non-walkable neighbor cell. advance() has no notion of walkable cells though:
            // it only bounds movement magnitude to speed * dt, not direction.
            let mut visitor = Visitor {
                id: "a".into(),
                position: (0.0, 0.0, 0.0),
                path: vec![(1, 0, 0)],
                target: (1, 0, 0),
                ticks_since_spawn: 0,
                heading: (0.0, 0.0, 0.0),
                is_leaving: false,
            };

            // Several neighbors packed close together can sum to a repulsion far larger than
            // any single crowd member's contribution — nothing currently caps the total.
            let strong_lateral_repulsion = (0.0, 10.0, 0.0);
            visitor.advance(1.0, 0.6, strong_lateral_repulsion);

            assert!(
                visitor.position.1.abs() < 0.5,
                "visitor left the single-wide corridor: y = {}",
                visitor.position.1
            );
        }
    }
}
