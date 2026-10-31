pub mod movement;

use std::collections::HashMap;

use crate::balance::VISIT_DURATION_TICKS;

pub type VisitorId = String;

pub const HUNGER: &str = "hunger";
pub const THIRST: &str = "thirst";
pub const FATIGUE: &str = "fatigue";
pub const TOILETS: &str = "toilets";
pub const ENTERTAINMENT: &str = "entertainment";

pub const CORE_NEEDS: [&str; 5] = [HUNGER, THIRST, FATIGUE, TOILETS, ENTERTAINMENT];

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

#[derive(Default)]
pub struct Visitor {
    pub id: VisitorId,
    pub position: (f32, f32, f32),
    pub path: Vec<(i32, i32, i32)>,
    pub destination: (i32, i32, i32),
    pub ticks_since_spawn: u64,
    pub heading: (f32, f32, f32),
    pub is_leaving: bool,
    pub last_visited: HashMap<(i32, i32, i32), u64>,
    pub queue_attraction: Option<String>,
}

impl Visitor {
    pub fn new(id: VisitorId, position: (f32, f32, f32)) -> Self {
        Self {
            id,
            position,
            ..Default::default()
        }
    }

    pub fn has_expired(&self) -> bool {
        self.ticks_since_spawn >= VISIT_DURATION_TICKS
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // fn assert_close(actual: (f32, f32, f32), expected: (f32, f32, f32)) {
    //     const EPSILON: f32 = 1e-5;
    //     assert!(
    //         (actual.0 - expected.0).abs() < EPSILON,
    //         "x: {} != {}",
    //         actual.0,
    //         expected.0
    //     );
    //     assert!(
    //         (actual.1 - expected.1).abs() < EPSILON,
    //         "y: {} != {}",
    //         actual.1,
    //         expected.1
    //     );
    //     assert!(
    //         (actual.2 - expected.2).abs() < EPSILON,
    //         "z: {} != {}",
    //         actual.2,
    //         expected.2
    //     );
    // }

    mod has_expired {
        use super::*;

        #[test]
        fn test_has_expired_is_false_before_visit_duration() {
            let visitor = Visitor {
                id: "v1".into(),
                position: (0.0, 0.0, 0.0),
                path: vec![],
                destination: (0, 0, 0),
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
                destination: (0, 0, 0),
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
                destination: (0, 0, 0),
                ticks_since_spawn: VISIT_DURATION_TICKS + 500,
                heading: (0.0, 0.0, 0.0),
                is_leaving: false,
                ..Default::default()
            };

            assert!(visitor.has_expired());
        }
    }
}
