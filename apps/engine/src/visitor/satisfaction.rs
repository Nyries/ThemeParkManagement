use std::collections::HashMap;

use crate::balance::{
    NEED_GROWTH_ENTERTAINMENT, NEED_GROWTH_FATIGUE_DISTANCE, NEED_GROWTH_FATIGUE_TIME,
    NEED_GROWTH_HUNGER, NEED_GROWTH_THIRST, NEED_GROWTH_TOILETS_DISTANCE, NEED_GROWTH_TOILETS_TIME,
    NOVELTY_FLOOR, NOVELTY_RECOVERY_TICKS, SATISFACTION_PENALTY_EXPONENT,
    SATISFACTION_RECENCY_WEIGHT,
};

use super::{ENTERTAINMENT, FATIGUE, HUNGER, THIRST, TOILETS};

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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_grow_needs_increases_every_core_need() {
        use super::super::CORE_NEEDS;

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
}
