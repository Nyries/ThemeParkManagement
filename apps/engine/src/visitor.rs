use rand::Rng;
use std::collections::HashMap;

use crate::balance::{
    AVOIDING_RADIUS, COMFORT_THRESHOLD_DEFAULT, COMFORT_THRESHOLD_VARIANCE, DENSITY_CAP,
    K_REPULSION, LATERAL_REPULSION_FACTOR, NEED_GROWTH_ENTERTAINMENT, NEED_GROWTH_FATIGUE_DISTANCE,
    NEED_GROWTH_FATIGUE_TIME, NEED_GROWTH_HUNGER, NEED_GROWTH_THIRST, NEED_GROWTH_TOILETS_DISTANCE,
    NEED_GROWTH_TOILETS_TIME, SATISFACTION_PENALTY_EXPONENT, SATISFACTION_RECENCY_WEIGHT,
    STEERING_FACTOR, VISIT_DURATION_TICKS,
};

pub type VisitorId = String;

pub const HUNGER: &str = "hunger";
pub const THIRST: &str = "thirst";
pub const FATIGUE: &str = "fatigue";
pub const TOILETS: &str = "toilets";
pub const ENTERTAINMENT: &str = "entertainment";

/// The jalon 2a subset of the ~13-need universal core (TPM-44) — the only needs with
/// a building in the current catalog able to relieve them. See Wiki des Formules
/// §Besoins et satisfaction des visiteurs.
pub const CORE_NEEDS: [&str; 5] = [HUNGER, THIRST, FATIGUE, TOILETS, ENTERTAINMENT];

#[derive(Default)]
pub struct Visitor {
    pub id: VisitorId,
    pub position: (f32, f32, f32),
    pub path: Vec<(i32, i32, i32)>,
    pub target: (i32, i32, i32),
    pub ticks_since_spawn: u64,
    pub heading: (f32, f32, f32),
    pub is_leaving: bool,
    /// Level per need, 0-100, grows over time/distance and is relieved by buildings.
    pub needs: HashMap<String, f32>,
    /// Individual comfort threshold per need, drawn at spawn (see `Visitor::new`).
    pub comfort_thresholds: HashMap<String, f32>,
    /// Cumulative satisfaction, exponential moving average of (gain - penalty). 0 = neutral.
    pub satisfaction: f32,
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

/// Growth per tick for each core need, driven by time and/or distance moved this tick.
/// See Wiki des Formules §Besoins et satisfaction des visiteurs.
pub fn grow_needs(needs: &mut HashMap<String, f32>, distance_moved: f32) {
    *needs.entry(HUNGER.to_string()).or_insert(0.0) += NEED_GROWTH_HUNGER;
    *needs.entry(THIRST.to_string()).or_insert(0.0) += NEED_GROWTH_THIRST;
    *needs.entry(TOILETS.to_string()).or_insert(0.0) +=
        NEED_GROWTH_TOILETS_TIME + NEED_GROWTH_TOILETS_DISTANCE * distance_moved;
    *needs.entry(FATIGUE.to_string()).or_insert(0.0) +=
        NEED_GROWTH_FATIGUE_TIME + NEED_GROWTH_FATIGUE_DISTANCE * distance_moved;
    *needs.entry(ENTERTAINMENT.to_string()).or_insert(0.0) += NEED_GROWTH_ENTERTAINMENT;
}

/// Relieves a single need by `amount`, floored at 0. No-op if the need is untracked.
pub fn relieve_need(needs: &mut HashMap<String, f32>, need: &str, amount: f32) {
    if let Some(level) = needs.get_mut(need) {
        *level = (*level - amount).max(0.0);
    }
}

/// Convex penalty: 0 while the need stays under its comfort threshold, growing sharply
/// past it. `penalty(need) = weight * ((threshold - level) / threshold)^p`, weight = 1.0
/// uniformly across needs for now (per-need weighting deferred to a later calibration).
pub fn penalty_for(level: f32, threshold: f32) -> f32 {
    if threshold <= 0.0 || level >= threshold {
        return 0.0;
    }
    ((threshold - level) / threshold).powf(SATISFACTION_PENALTY_EXPONENT)
}

/// Gain granted when a need is relieved, proportional to the relief's intensity and to
/// how urgent the need was right before being relieved (0 if it was already comfortable).
pub fn gain_for(relief_intensity: f32, level_before_relief: f32, threshold: f32) -> f32 {
    let urgency = if threshold <= 0.0 {
        0.0
    } else {
        ((threshold - level_before_relief) / threshold).max(0.0)
    };
    relief_intensity * urgency
}

/// Rolls a single tick's (gain - penalty) into the cumulative satisfaction, as an
/// exponential moving average — `SATISFACTION_RECENCY_WEIGHT` controls how much weight
/// recent ticks carry over the visit's history.
pub fn update_satisfaction(current: f32, gain: f32, penalty: f32) -> f32 {
    current * (1.0 - SATISFACTION_RECENCY_WEIGHT) + (gain - penalty) * SATISFACTION_RECENCY_WEIGHT
}

impl Visitor {
    /// Builds a freshly spawned visitor at `position`, with all core needs at 0 and
    /// individually-drawn comfort thresholds. Callers still need to set `path`/`target`
    /// themselves (this constructor has no map access to compute them).
    pub fn new(id: VisitorId, position: (f32, f32, f32)) -> Self {
        let mut rng = rand::thread_rng();
        let comfort_thresholds = CORE_NEEDS
            .iter()
            .map(|&need| {
                let jitter = rng.gen_range(-COMFORT_THRESHOLD_VARIANCE..=COMFORT_THRESHOLD_VARIANCE);
                (need.to_string(), COMFORT_THRESHOLD_DEFAULT + jitter)
            })
            .collect();
        let needs = CORE_NEEDS.iter().map(|&need| (need.to_string(), 0.0)).collect();

        Self {
            id,
            position,
            needs,
            comfort_thresholds,
            ..Default::default()
        }
    }

    pub fn has_expired(&self) -> bool {
        self.ticks_since_spawn >= VISIT_DURATION_TICKS
    }

    /// True once cumulative satisfaction has collapsed net (see
    /// `EARLY_DEPARTURE_SATISFACTION_THRESHOLD`) — stricter than the ordinary per-need
    /// penalty so a visitor doesn't bail at the first uncomfortable need.
    pub fn should_leave_early(&self) -> bool {
        self.satisfaction < crate::balance::EARLY_DEPARTURE_SATISFACTION_THRESHOLD
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

    mod needs_and_satisfaction {
        use super::*;

        #[test]
        fn test_new_sets_all_core_needs_to_zero() {
            let visitor = Visitor::new("v1".into(), (0.0, 0.0, 0.0));

            for need in CORE_NEEDS {
                assert_eq!(visitor.needs.get(need), Some(&0.0));
            }
        }

        #[test]
        fn test_new_draws_comfort_thresholds_within_the_expected_range() {
            let visitor = Visitor::new("v1".into(), (0.0, 0.0, 0.0));

            for need in CORE_NEEDS {
                let threshold = *visitor.comfort_thresholds.get(need).unwrap();
                assert!(
                    (COMFORT_THRESHOLD_DEFAULT - COMFORT_THRESHOLD_VARIANCE..=COMFORT_THRESHOLD_DEFAULT + COMFORT_THRESHOLD_VARIANCE)
                        .contains(&threshold),
                    "{need} threshold {threshold} out of range"
                );
            }
        }

        #[test]
        fn test_new_starts_with_neutral_satisfaction_and_not_leaving() {
            let visitor = Visitor::new("v1".into(), (0.0, 0.0, 0.0));

            assert_eq!(visitor.satisfaction, 0.0);
            assert!(!visitor.is_leaving);
        }

        #[test]
        fn test_grow_needs_increases_every_core_need() {
            let mut needs = HashMap::new();

            grow_needs(&mut needs, 0.0);

            for need in CORE_NEEDS {
                assert!(needs[need] > 0.0, "{need} should have grown");
            }
        }

        #[test]
        fn test_grow_needs_distance_only_affects_toilets_and_fatigue() {
            let mut still = HashMap::new();
            grow_needs(&mut still, 0.0);

            let mut moved = HashMap::new();
            grow_needs(&mut moved, 10.0);

            assert_eq!(still[HUNGER], moved[HUNGER]);
            assert_eq!(still[THIRST], moved[THIRST]);
            assert_eq!(still[ENTERTAINMENT], moved[ENTERTAINMENT]);
            assert!(moved[TOILETS] > still[TOILETS]);
            assert!(moved[FATIGUE] > still[FATIGUE]);
        }

        #[test]
        fn test_relieve_need_lowers_level_floored_at_zero() {
            let mut needs = HashMap::from([(HUNGER.to_string(), 20.0)]);

            relieve_need(&mut needs, HUNGER, 35.0);

            assert_eq!(needs[HUNGER], 0.0);
        }

        #[test]
        fn test_relieve_need_is_a_noop_for_an_untracked_need() {
            let mut needs = HashMap::new();

            relieve_need(&mut needs, HUNGER, 20.0);

            assert!(needs.is_empty());
        }

        #[test]
        fn test_penalty_for_is_zero_at_or_above_threshold() {
            assert_eq!(penalty_for(70.0, 70.0), 0.0);
            assert_eq!(penalty_for(90.0, 70.0), 0.0);
        }

        #[test]
        fn test_penalty_for_grows_convexly_below_threshold() {
            let small_deficit = penalty_for(60.0, 70.0); // (10/70)^2
            let large_deficit = penalty_for(20.0, 70.0); // (50/70)^2

            assert!(small_deficit > 0.0);
            assert!(large_deficit > small_deficit);
            let expected_small = (10.0_f32 / 70.0).powf(SATISFACTION_PENALTY_EXPONENT);
            assert!((small_deficit - expected_small).abs() < 1e-5);
        }

        #[test]
        fn test_gain_for_is_zero_when_need_was_already_comfortable() {
            let gain = gain_for(20.0, 80.0, 70.0); // level_before >= threshold

            assert_eq!(gain, 0.0);
        }

        #[test]
        fn test_gain_for_scales_with_intensity_and_urgency() {
            let low_urgency = gain_for(20.0, 60.0, 70.0); // urgency = 10/70
            let high_urgency = gain_for(20.0, 10.0, 70.0); // urgency = 60/70

            assert!(high_urgency > low_urgency);
            assert!(low_urgency > 0.0);
        }

        #[test]
        fn test_update_satisfaction_moves_toward_the_new_signal() {
            let after_gain = update_satisfaction(0.0, 10.0, 0.0);
            let after_penalty = update_satisfaction(0.0, 0.0, 10.0);

            assert!(after_gain > 0.0);
            assert!(after_penalty < 0.0);
        }

        #[test]
        fn test_update_satisfaction_weighs_recent_signal_by_recency_weight() {
            let expected = 0.0 * (1.0 - SATISFACTION_RECENCY_WEIGHT) + 5.0 * SATISFACTION_RECENCY_WEIGHT;

            let result = update_satisfaction(0.0, 5.0, 0.0);

            assert!((result - expected).abs() < 1e-5);
        }

        #[test]
        fn test_should_leave_early_is_false_above_the_threshold() {
            let mut visitor = Visitor::new("v1".into(), (0.0, 0.0, 0.0));
            visitor.satisfaction = -10.0;

            assert!(!visitor.should_leave_early());
        }

        #[test]
        fn test_should_leave_early_is_true_below_the_threshold() {
            let mut visitor = Visitor::new("v1".into(), (0.0, 0.0, 0.0));
            visitor.satisfaction = -60.0;

            assert!(visitor.should_leave_early());
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
                ..Default::default()
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
                ..Default::default()
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
                ..Default::default()
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
                ..Default::default()
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
                ..Default::default()
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
                ..Default::default()
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
                ..Default::default()
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
                ..Default::default()
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
                ..Default::default()
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
                ..Default::default()
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
                ..Default::default()
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
                ..Default::default()
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
                ..Default::default()
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
                ..Default::default()
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
