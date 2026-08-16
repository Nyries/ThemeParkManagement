// PLACEHOLDER
pub const K_REPULSION: f32 = 0.2;

pub const VISITOR_RADIUS: f32 = 0.2;
pub const AVOIDING_RADIUS: f32 = 0.2;

pub const DENSITY_CAP: usize = 5;

pub const VISIT_DURATION_TICKS: u64 = 1000;

pub const TICK_INTERVAL: f32 = 0.05; // seconds
pub const SPAWN_INTERVAL_TICKS: u64 = 20; // ticks

// Closer to 1 makes a turn almost instantaneous; 0 is a very progressive turn
pub const STEERING_FACTOR: f32 = 0.3;
pub const LATERAL_REPULSION_FACTOR: f32 = 0.1;

/// 🔶 esquissé — coût de déplacement horizontal par case, selon le type
/// d'infrastructure (pas le terrain). Voir Wiki des Formules §Déplacement des visiteurs.
pub const MOVEMENT_COST_PATH: u32 = 1;
pub const MOVEMENT_COST_RAMP: u32 = 2;
pub const MOVEMENT_COST_STAIRS: u32 = 3;

/// 🔶 PLACEHOLDER — needs calibration at playtest. See Wiki des Formules §Besoins
/// et satisfaction des visiteurs (TPM-44). Growth per tick, on a 0-100 scale.
pub const NEED_GROWTH_HUNGER: f32 = 0.05;
pub const NEED_GROWTH_THIRST: f32 = 0.07;
pub const NEED_GROWTH_TOILETS_TIME: f32 = 0.03;
pub const NEED_GROWTH_TOILETS_DISTANCE: f32 = 0.005;
pub const NEED_GROWTH_FATIGUE_TIME: f32 = 0.02;
pub const NEED_GROWTH_FATIGUE_DISTANCE: f32 = 0.01;
pub const NEED_GROWTH_FATIGUE_PER_INTENSITY: f32 = 2.0;
pub const NEED_GROWTH_ENTERTAINMENT: f32 = 0.02;

/// 🔶 PLACEHOLDER — generic relief granted by riding any Attraction (no per-template
/// `needs_relief` entry exists for entertainment in the catalog, unlike hunger/thirst/
/// fatigue/toilets which are declared per building).
pub const ENTERTAINMENT_RELIEF: f32 = 40.0;

/// 🔶 PLACEHOLDER — individual comfort threshold per need, drawn at spawn as
/// `COMFORT_THRESHOLD_DEFAULT ± COMFORT_THRESHOLD_VARIANCE` (uniform).
pub const COMFORT_THRESHOLD_DEFAULT: f32 = 70.0;
pub const COMFORT_THRESHOLD_VARIANCE: f32 = 10.0;

/// 🔶 PLACEHOLDER — exponent `p` in the convex satisfaction penalty formula, and the
/// recency weight `λ` of the exponential moving average of cumulative satisfaction.
pub const SATISFACTION_PENALTY_EXPONENT: f32 = 2.0;
pub const SATISFACTION_RECENCY_WEIGHT: f32 = 0.1;

/// 🔶 PLACEHOLDER — cumulative satisfaction below this triggers an early departure
/// (stricter than the ordinary per-need penalty, sunk cost of the paid ticket).
pub const EARLY_DEPARTURE_SATISFACTION_THRESHOLD: f32 = -50.0;

/// 🔶 PLACEHOLDER — affinity multiplier is fixed at neutral (no bonus/malus) until
/// TPM-48 provides real per-profile affinity vectors. See Wiki des Formules §Choix de
/// destination.
pub const AFFINITY_DEFAULT: f32 = 1.0;

/// 🔶 PLACEHOLDER — novelty multiplier right after visiting a target, and the number
/// of ticks it takes to linearly recover back to 1.0.
pub const NOVELTY_FLOOR: f32 = 0.3;
pub const NOVELTY_RECOVERY_TICKS: f32 = 200.0;

// Calibrated
