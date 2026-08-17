mod movement;
mod satisfaction;

use movement::{atternuate_lateral_repulsion, distance, lerp_direction, normalize};

pub(crate) use movement::{direction, weighted_lane_bias};
pub use movement::{
    lane_bias_strength, lateral_repulsion_factor_for, perpendicular_of, repulsion_force, speed_at,
};
pub use satisfaction::{
    gain_for, grow_needs, novelty_for, penalty_for, relieve_need, score_for, update_satisfaction,
    utility_for,
};

use rand::Rng;
use std::collections::HashMap;

use crate::balance::{
    AFFINITY_DEFAULT, COMFORT_THRESHOLD_DEFAULT, COMFORT_THRESHOLD_VARIANCE, STEERING_FACTOR,
    VISIT_DURATION_TICKS,
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
#[derive(Debug, Clone, Default)]
pub struct VisitorProfile {
    pub name: &'static str,
    /// Positive = preference, negative = aversion, absent = neutral.
    pub tag_affinities: HashMap<&'static str, f32>,
    /// Mean comfort threshold per need; absent falls back to `COMFORT_THRESHOLD_DEFAULT`.
    pub need_thresholds: HashMap<&'static str, f32>,
}

// TODO: fixed placeholder profiles, calibrated against the catalog's current tags.
pub fn visitor_profiles() -> Vec<VisitorProfile> {
    vec![
        VisitorProfile {
            name: "Familles",
            tag_affinities: HashMap::from([("family", 0.8), ("show", 0.3), ("thrill", -0.4)]),
            need_thresholds: HashMap::from([(FATIGUE, 60.0)]),
        },
        VisitorProfile {
            name: "Ados",
            tag_affinities: HashMap::from([("thrill", 0.9), ("social", 0.6), ("family", -0.2)]),
            need_thresholds: HashMap::from([(FATIGUE, 85.0), (HUNGER, 60.0), (THIRST, 60.0)]),
        },
        VisitorProfile {
            name: "Seniors",
            tag_affinities: HashMap::from([("show", 0.7), ("family", 0.2), ("thrill", -0.6)]),
            need_thresholds: HashMap::from([(FATIGUE, 55.0), (TOILETS, 55.0)]),
        },
    ]
}

/// Uniformly random profile — the jalon 2a stub distribution.
pub fn pick_profile(rng: &mut impl Rng) -> VisitorProfile {
    let profiles = visitor_profiles();
    let idx = rng.gen_range(0..profiles.len());
    profiles[idx].clone()
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
    /// Drawn at spawn, see `Visitor::new`.
    pub profile: VisitorProfile,
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
    /// `Some(attraction_building_id)` while waiting in that attraction's queue — governed
    /// by continuous FIFO advancement (`queue::QueueState`) instead of `path`/A*.
    pub queue_attraction: Option<String>,
}

impl Visitor {
    /// Fresh visitor at `position`; caller still sets `path`/`target`.
    pub fn new(id: VisitorId, position: (f32, f32, f32)) -> Self {
        let mut rng = rand::thread_rng();
        let profile = pick_profile(&mut rng);
        let comfort_thresholds = CORE_NEEDS
            .iter()
            .map(|&need| {
                let mean = profile
                    .need_thresholds
                    .get(need)
                    .copied()
                    .unwrap_or(COMFORT_THRESHOLD_DEFAULT);
                let jitter =
                    rng.gen_range(-COMFORT_THRESHOLD_VARIANCE..=COMFORT_THRESHOLD_VARIANCE);
                (need.to_string(), mean + jitter)
            })
            .collect();
        let needs = CORE_NEEDS
            .iter()
            .map(|&need| (need.to_string(), 0.0))
            .collect();

        Self {
            id,
            position,
            profile,
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

    mod new {
        use super::*;

        #[test]
        fn test_new_sets_all_core_needs_to_zero() {
            let visitor = Visitor::new("v1".into(), (0.0, 0.0, 0.0));

            for need in CORE_NEEDS {
                assert_eq!(visitor.needs.get(need), Some(&0.0));
            }
        }

        #[test]
        fn test_new_draws_comfort_thresholds_within_the_expected_range_of_its_profile() {
            let visitor = Visitor::new("v1".into(), (0.0, 0.0, 0.0));

            for need in CORE_NEEDS {
                let mean = visitor
                    .profile
                    .need_thresholds
                    .get(need)
                    .copied()
                    .unwrap_or(COMFORT_THRESHOLD_DEFAULT);
                let threshold = *visitor.comfort_thresholds.get(need).unwrap();
                assert!(
                    (mean - COMFORT_THRESHOLD_VARIANCE..=mean + COMFORT_THRESHOLD_VARIANCE)
                        .contains(&threshold),
                    "{need} threshold {threshold} out of range around profile mean {mean}"
                );
            }
        }

        #[test]
        fn test_new_assigns_one_of_the_known_profiles() {
            let visitor = Visitor::new("v1".into(), (0.0, 0.0, 0.0));

            let known_names: Vec<&str> = visitor_profiles().iter().map(|p| p.name).collect();
            assert!(known_names.contains(&visitor.profile.name));
        }

        #[test]
        fn test_new_starts_with_neutral_satisfaction_and_not_leaving() {
            let visitor = Visitor::new("v1".into(), (0.0, 0.0, 0.0));

            assert_eq!(visitor.satisfaction, 0.0);
            assert!(!visitor.is_leaving);
        }
    }

    mod should_leave_early {
        use super::*;

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

    mod visitor_profiles {
        use super::*;

        #[test]
        fn test_returns_exactly_the_three_jalon_2a_profiles() {
            let profiles = visitor_profiles();

            let names: Vec<&str> = profiles.iter().map(|p| p.name).collect();
            assert_eq!(names, vec!["Familles", "Ados", "Seniors"]);
        }

        #[test]
        fn test_pick_profile_can_return_every_known_profile() {
            let known_names: Vec<&str> = visitor_profiles().iter().map(|p| p.name).collect();
            let mut rng = rand::thread_rng();

            let mut seen: Vec<&str> = Vec::new();
            for _ in 0..200 {
                let picked = pick_profile(&mut rng);
                assert!(known_names.contains(&picked.name));
                if !seen.contains(&picked.name) {
                    seen.push(picked.name);
                }
            }
            assert_eq!(seen.len(), known_names.len());
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
                ..Default::default()
            };

            let result = affinity_for(&profile, &["unknown_tag".to_string()]);

            assert_eq!(result, AFFINITY_DEFAULT);
        }

        #[test]
        fn test_affinity_for_averages_over_several_tags() {
            let profile = VisitorProfile {
                name: "test",
                tag_affinities: HashMap::from([("a", 0.4), ("b", -0.2)]),
                ..Default::default()
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
        use crate::balance::LATERAL_REPULSION_FACTOR;

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
