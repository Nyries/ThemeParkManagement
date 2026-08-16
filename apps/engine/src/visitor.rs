use rand::Rng;
use std::collections::HashMap;

use crate::balance::{
    AFFINITY_DEFAULT, AVOIDING_RADIUS, COMFORT_THRESHOLD_DEFAULT, COMFORT_THRESHOLD_VARIANCE,
    DENSITY_CAP, K_REPULSION, LANE_BIAS_STRENGTH, LATERAL_REPULSION_FACTOR,
    LATERAL_REPULSION_FACTOR_AT_MAX_DENSITY, NEED_GROWTH_ENTERTAINMENT,
    NEED_GROWTH_FATIGUE_DISTANCE, NEED_GROWTH_FATIGUE_TIME, NEED_GROWTH_HUNGER, NEED_GROWTH_THIRST,
    NEED_GROWTH_TOILETS_DISTANCE, NEED_GROWTH_TOILETS_TIME, NOVELTY_FLOOR, NOVELTY_RECOVERY_TICKS,
    SATISFACTION_PENALTY_EXPONENT, SATISFACTION_RECENCY_WEIGHT, SPEED_FLOOR_AT_MAX_DENSITY,
    STEERING_FACTOR, VISIT_DURATION_TICKS,
};

pub type VisitorId = String;

pub const HUNGER: &str = "hunger";
pub const THIRST: &str = "thirst";
pub const FATIGUE: &str = "fatigue";
pub const TOILETS: &str = "toilets";
pub const ENTERTAINMENT: &str = "entertainment";

/// Subset of the universal need core with a building in the catalog able to relieve it.
pub const CORE_NEEDS: [&str; 5] = [HUNGER, THIRST, FATIGUE, TOILETS, ENTERTAINMENT];

/// affinité(profil, cible) factor: data, not an enum, keyed on the catalog's free-form
/// `BuildingTemplate.tags` — an unlisted tag is just neutral (`affinity_for`).
#[derive(Debug, Clone)]
pub struct VisitorProfile {
    pub name: &'static str,
    /// Positive = preference, negative = aversion, absent = neutral.
    pub tag_affinities: HashMap<&'static str, f32>,
}

// TODO: fixed placeholder profiles, calibrated against the catalog's current tags.
pub fn visitor_profiles() -> Vec<VisitorProfile> {
    vec![
        VisitorProfile {
            name: "Familles",
            tag_affinities: HashMap::from([("family", 0.8), ("show", 0.3), ("thrill", -0.4)]),
        },
        VisitorProfile {
            name: "Ados",
            tag_affinities: HashMap::from([("thrill", 0.9), ("social", 0.6), ("family", -0.2)]),
        },
        VisitorProfile {
            name: "Seniors",
            tag_affinities: HashMap::from([("show", 0.7), ("family", 0.2), ("thrill", -0.6)]),
        },
    ]
}

/// Mean affinity across a candidate's tags, centered on `AFFINITY_DEFAULT` rather than
/// 0 — a 0 factor would zero out `utilité × affinité` even for a strongly-wanted need.
pub fn affinity_for(profile: &VisitorProfile, tags: &[String]) -> f32 {
    if tags.is_empty() {
        return AFFINITY_DEFAULT;
    }
    let sum: f32 = tags
        .iter()
        .map(|tag| {
            profile
                .tag_affinities
                .get(tag.as_str())
                .copied()
                .unwrap_or(0.0)
        })
        .sum();
    AFFINITY_DEFAULT + sum / tags.len() as f32
}

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
    /// Tick each cell was last targeted, for the novelty factor — scoped to the visit.
    pub last_visited: HashMap<(i32, i32, i32), u64>,
    /// Consecutive near-zero-movement ticks despite nonzero speed (head-on standoff).
    pub stall_ticks: u64,
}

fn distance(a: (f32, f32, f32), b: (f32, f32, f32)) -> f32 {
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

/// Growth per tick for each core need, driven by time and/or distance moved this tick.
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

/// Convex penalty, 0 under the comfort threshold and growing sharply past it:
/// `((level - threshold) / threshold)^p`.
pub fn penalty_for(level: f32, threshold: f32) -> f32 {
    if threshold <= 0.0 || level <= threshold {
        return 0.0;
    }
    ((level - threshold) / threshold).powf(SATISFACTION_PENALTY_EXPONENT)
}

/// Gain from relieving a need, proportional to relief intensity and prior urgency.
pub fn gain_for(relief_intensity: f32, level_before_relief: f32, threshold: f32) -> f32 {
    let urgency = if threshold <= 0.0 {
        0.0
    } else {
        (level_before_relief / threshold).clamp(0.0, 1.0)
    };
    relief_intensity * urgency
}

/// Rolls a tick's (gain - penalty) into cumulative satisfaction as an EMA.
pub fn update_satisfaction(current: f32, gain: f32, penalty: f32) -> f32 {
    current * (1.0 - SATISFACTION_RECENCY_WEIGHT) + (gain - penalty) * SATISFACTION_RECENCY_WEIGHT
}

/// `utilité` term: dot product of need levels and a building's `needs_relief`.
pub fn utility_for(needs: &HashMap<String, f32>, needs_relief: &HashMap<String, u32>) -> f32 {
    needs_relief
        .iter()
        .map(|(need, &relief)| needs.get(need).copied().unwrap_or(0.0) * relief as f32)
        .sum()
}

/// `nouveauté` term: falls to `NOVELTY_FLOOR` right after a visit, recovers to 1.0
/// linearly over `NOVELTY_RECOVERY_TICKS`.
pub fn novelty_for(last_visited_tick: Option<u64>, current_tick: u64) -> f32 {
    let Some(last_tick) = last_visited_tick else {
        return 1.0;
    };
    let elapsed = current_tick.saturating_sub(last_tick) as f32;
    (NOVELTY_FLOOR + (1.0 - NOVELTY_FLOOR) * (elapsed / NOVELTY_RECOVERY_TICKS)).min(1.0)
}

/// `score = utilité × affinité × nouveauté / coût`; 0 for a non-positive cost.
pub fn score_for(utility: f32, affinity: f32, novelty: f32, cost: f32) -> f32 {
    if cost <= 0.0 {
        return 0.0;
    }
    (utility * affinity * novelty) / cost
}

impl Visitor {
    /// Fresh visitor at `position`; caller still sets `path`/`target`.
    pub fn new(id: VisitorId, position: (f32, f32, f32)) -> Self {
        let mut rng = rand::thread_rng();
        let comfort_thresholds = CORE_NEEDS
            .iter()
            .map(|&need| {
                let jitter =
                    rng.gen_range(-COMFORT_THRESHOLD_VARIANCE..=COMFORT_THRESHOLD_VARIANCE);
                (need.to_string(), COMFORT_THRESHOLD_DEFAULT + jitter)
            })
            .collect();
        let needs = CORE_NEEDS
            .iter()
            .map(|&need| (need.to_string(), 0.0))
            .collect();

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

    /// True once cumulative satisfaction has collapsed net (stricter than per-need penalty).
    pub fn should_leave_early(&self) -> bool {
        self.satisfaction < crate::balance::EARLY_DEPARTURE_SATISFACTION_THRESHOLD
    }

    pub fn advance(
        &mut self,
        speed: f32,
        dt: f32,
        repulsion: (f32, f32, f32),
        lateral_factor: f32,
    ) {
        let Some(&next) = self.path.first() else {
            return;
        };
        let next_f = (next.0 as f32, next.1 as f32, next.2 as f32);
        let dist_to_next = distance(self.position, next_f);

        let raw_desired = direction(self.position, next_f);
        let attenuated_repulsion =
            atternuate_lateral_repulsion(repulsion, raw_desired, lateral_factor);
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
                    (COMFORT_THRESHOLD_DEFAULT - COMFORT_THRESHOLD_VARIANCE
                        ..=COMFORT_THRESHOLD_DEFAULT + COMFORT_THRESHOLD_VARIANCE)
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
        fn test_penalty_for_is_zero_at_or_below_threshold() {
            assert_eq!(penalty_for(70.0, 70.0), 0.0);
            assert_eq!(penalty_for(50.0, 70.0), 0.0);
        }

        #[test]
        fn test_penalty_for_grows_convexly_past_threshold() {
            let small_overshoot = penalty_for(80.0, 70.0); // (10/70)^2
            let large_overshoot = penalty_for(100.0, 70.0); // (30/70)^2

            assert!(small_overshoot > 0.0);
            assert!(large_overshoot > small_overshoot);
            let expected_small = (10.0_f32 / 70.0).powf(SATISFACTION_PENALTY_EXPONENT);
            assert!((small_overshoot - expected_small).abs() < 1e-5);
        }

        #[test]
        fn test_gain_for_is_zero_when_need_was_far_from_urgent() {
            let gain = gain_for(20.0, 0.0, 70.0); // level_before = 0, no urgency at all

            assert_eq!(gain, 0.0);
        }

        #[test]
        fn test_gain_for_scales_with_intensity_and_urgency() {
            let low_urgency = gain_for(20.0, 20.0, 70.0); // urgency = 20/70
            let high_urgency = gain_for(20.0, 60.0, 70.0); // urgency = 60/70

            assert!(high_urgency > low_urgency);
            assert!(low_urgency > 0.0);
        }

        #[test]
        fn test_gain_for_urgency_caps_at_one_past_the_threshold() {
            let at_threshold = gain_for(20.0, 70.0, 70.0);
            let past_threshold = gain_for(20.0, 100.0, 70.0);

            assert_eq!(at_threshold, 20.0);
            assert_eq!(past_threshold, 20.0);
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
            let expected =
                0.0 * (1.0 - SATISFACTION_RECENCY_WEIGHT) + 5.0 * SATISFACTION_RECENCY_WEIGHT;

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

    mod destination_score {
        use super::*;

        #[test]
        fn test_utility_for_is_the_dot_product_of_needs_and_relief() {
            let needs = HashMap::from([(HUNGER.to_string(), 80.0), (THIRST.to_string(), 20.0)]);
            let relief = HashMap::from([(HUNGER.to_string(), 30), (THIRST.to_string(), 10)]);

            let result = utility_for(&needs, &relief);

            assert_eq!(result, 80.0 * 30.0 + 20.0 * 10.0);
        }

        #[test]
        fn test_utility_for_ignores_relief_for_untracked_needs() {
            let needs = HashMap::new();
            let relief = HashMap::from([(HUNGER.to_string(), 30)]);

            let result = utility_for(&needs, &relief);

            assert_eq!(result, 0.0);
        }

        #[test]
        fn test_utility_for_is_zero_with_no_relief() {
            let needs = HashMap::from([(HUNGER.to_string(), 80.0)]);
            let relief = HashMap::new();

            let result = utility_for(&needs, &relief);

            assert_eq!(result, 0.0);
        }

        #[test]
        fn test_novelty_for_is_full_when_never_visited() {
            let result = novelty_for(None, 1000);

            assert_eq!(result, 1.0);
        }

        #[test]
        fn test_novelty_for_is_at_the_floor_right_after_a_visit() {
            let result = novelty_for(Some(1000), 1000);

            assert_eq!(result, NOVELTY_FLOOR);
        }

        #[test]
        fn test_novelty_for_recovers_linearly_toward_one() {
            let halfway = novelty_for(Some(0), 100); // half of NOVELTY_RECOVERY_TICKS (200)

            let expected = NOVELTY_FLOOR + (1.0 - NOVELTY_FLOOR) * 0.5;
            assert!((halfway - expected).abs() < 1e-5);
        }

        #[test]
        fn test_novelty_for_caps_at_one_past_the_recovery_window() {
            let result = novelty_for(Some(0), 10_000);

            assert_eq!(result, 1.0);
        }

        #[test]
        fn test_score_for_combines_the_four_terms() {
            let result = score_for(10.0, 2.0, 0.5, 5.0);

            assert_eq!(result, (10.0 * 2.0 * 0.5) / 5.0);
        }

        #[test]
        fn test_score_for_is_zero_for_a_non_positive_cost() {
            assert_eq!(score_for(10.0, 1.0, 1.0, 0.0), 0.0);
            assert_eq!(score_for(10.0, 1.0, 1.0, -1.0), 0.0);
        }
    }

    mod visitor_profiles {
        use super::*;

        #[test]
        fn test_returns_exactly_the_three_jalon_2a_profiles() {
            let profiles = visitor_profiles();

            let names: Vec<&str> = profiles.iter().map(|p| p.name).collect();
            assert_eq!(names, vec!["Familles", "Ados", "Seniors"]);
        }

        #[test]
        fn test_affinity_for_is_neutral_with_no_tags() {
            let profile = &visitor_profiles()[0];

            let result = affinity_for(profile, &[]);

            assert_eq!(result, AFFINITY_DEFAULT);
        }

        #[test]
        fn test_affinity_for_is_neutral_for_a_tag_the_profile_has_no_opinion_on() {
            let profile = VisitorProfile {
                name: "test",
                tag_affinities: HashMap::new(),
            };

            let result = affinity_for(&profile, &["unknown_tag".to_string()]);

            assert_eq!(result, AFFINITY_DEFAULT);
        }

        #[test]
        fn test_affinity_for_averages_over_several_tags() {
            let profile = VisitorProfile {
                name: "test",
                tag_affinities: HashMap::from([("a", 0.4), ("b", -0.2)]),
            };

            let result = affinity_for(&profile, &["a".to_string(), "b".to_string()]);

            let expected = AFFINITY_DEFAULT + (0.4 + -0.2) / 2.0;
            assert!((result - expected).abs() < 1e-5);
        }

        #[test]
        fn test_familles_prefer_family_tagged_attractions_over_thrill() {
            let profile = &visitor_profiles()[0]; // Familles

            let family_score = affinity_for(profile, &["family".to_string()]);
            let thrill_score = affinity_for(profile, &["thrill".to_string()]);

            assert!(family_score > AFFINITY_DEFAULT);
            assert!(thrill_score < AFFINITY_DEFAULT);
        }

        #[test]
        fn test_ados_prefer_thrill_tagged_attractions_over_family() {
            let profile = &visitor_profiles()[1]; // Ados

            let thrill_score = affinity_for(profile, &["thrill".to_string()]);
            let family_score = affinity_for(profile, &["family".to_string()]);

            assert!(thrill_score > AFFINITY_DEFAULT);
            assert!(family_score < AFFINITY_DEFAULT);
        }

        #[test]
        fn test_seniors_prefer_show_tagged_attractions_over_thrill() {
            let profile = &visitor_profiles()[2]; // Seniors

            let show_score = affinity_for(profile, &["show".to_string()]);
            let thrill_score = affinity_for(profile, &["thrill".to_string()]);

            assert!(show_score > AFFINITY_DEFAULT);
            assert!(thrill_score < AFFINITY_DEFAULT);
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

            visitor.advance(1.0, 1.0, (0.0, 0.0, 0.0), LATERAL_REPULSION_FACTOR);

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

            visitor.advance(1.0, 0.3, (0.0, 0.0, 0.0), LATERAL_REPULSION_FACTOR); // step = 0.3, distance = 1.0

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

            visitor.advance(1.0, 1.0, (0.0, 0.0, 0.0), LATERAL_REPULSION_FACTOR); // step = 1.0 == distance

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

            visitor.advance(2.0, 1.0, (0.0, 0.0, 0.0), LATERAL_REPULSION_FACTOR); // step = 2.0, distance = 1.0

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

            visitor.advance(1.0, 1.0, (0.0, 0.0, 0.0), LATERAL_REPULSION_FACTOR);

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

            visitor.advance(0.0, 1.0, (0.0, 0.0, 0.0), LATERAL_REPULSION_FACTOR);
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

            visitor.advance(0.0, 1.0, (0.0, 0.0, 0.0), LATERAL_REPULSION_FACTOR);

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

            visitor.advance(0.0, 1.0, (0.0, 0.0, 0.0), LATERAL_REPULSION_FACTOR);

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

            visitor.advance(1.0, 0.5, (0.0, 1.0, 0.0), LATERAL_REPULSION_FACTOR); // répulsion latérale forte

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
            visitor.advance(speed, dt, (5.0, 5.0, 0.0), LATERAL_REPULSION_FACTOR);

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
            visitor.advance(1.0, 0.6, strong_lateral_repulsion, LATERAL_REPULSION_FACTOR);

            assert!(
                visitor.position.1.abs() < 0.5,
                "visitor left the single-wide corridor: y = {}",
                visitor.position.1
            );
        }
    }
}
