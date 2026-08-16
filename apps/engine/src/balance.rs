// PLACEHOLDER
pub const K_REPULSION: f32 = 0.2;

pub const VISITOR_RADIUS: f32 = 0.10;
pub const AVOIDING_RADIUS: f32 = 0.2;

pub const DENSITY_CAP: usize = 5;

/// Floor so `speed_at` never hits exactly 0 at/above `DENSITY_CAP` — a stalled visitor
/// never leaves its cell, so density there could only ever grow, never recover.
pub const SPEED_FLOOR_AT_MAX_DENSITY: f32 = 0.1;

pub const VISIT_DURATION_TICKS: u64 = 1000;

pub const TICK_INTERVAL: f32 = 0.05; // seconds
pub const SPAWN_INTERVAL_TICKS: u64 = 20; // ticks

// Closer to 1 makes a turn almost instantaneous; 0 is a very progressive turn
pub const STEERING_FACTOR: f32 = 0.3;
pub const LATERAL_REPULSION_FACTOR: f32 = 0.1;

/// Lateral repulsion ramps from `LATERAL_REPULSION_FACTOR` up to this as density
/// approaches `DENSITY_CAP`, so crowded visitors actually step aside.
pub const LATERAL_REPULSION_FACTOR_AT_MAX_DENSITY: f32 = 1.0;

/// Steers a visitor toward whichever neighbouring lane is less crowded, beyond the
/// short reach of `AVOIDING_RADIUS`.
pub const LANE_BIAS_STRENGTH: f32 = 0.4;

/// How many cells ahead `compute_detour_bias` scans for a jam, and how dense a cell
/// must be to count — lower than `DENSITY_CAP` so the steer kicks in before crawling.
pub const DETOUR_LOOKAHEAD_CELLS: usize = 5;
pub const DETOUR_DENSITY_THRESHOLD: usize = 3;
pub const DETOUR_BIAS_STRENGTH: f32 = 0.5;

/// Multiplier on visitor diameter for queue spacing (1.0 = shoulder to shoulder).
pub const QUEUE_SPACING_FACTOR: f32 = 1.0;

/// 🔶 esquissé — coût de déplacement horizontal par case, selon le type
/// d'infrastructure (pas le terrain). Voir Wiki des Formules §Déplacement des visiteurs.
pub const MOVEMENT_COST_PATH: u32 = 1;
pub const MOVEMENT_COST_RAMP: u32 = 2;
pub const MOVEMENT_COST_STAIRS: u32 = 3;

/// 🔶 PLACEHOLDER — needs calibration at playtest. Growth per tick, on a 0-100 scale.
pub const NEED_GROWTH_HUNGER: f32 = 0.05;
pub const NEED_GROWTH_THIRST: f32 = 0.07;
pub const NEED_GROWTH_TOILETS_TIME: f32 = 0.03;
pub const NEED_GROWTH_TOILETS_DISTANCE: f32 = 0.005;
pub const NEED_GROWTH_FATIGUE_TIME: f32 = 0.02;
pub const NEED_GROWTH_FATIGUE_DISTANCE: f32 = 0.01;
pub const NEED_GROWTH_FATIGUE_PER_INTENSITY: f32 = 2.0;
pub const NEED_GROWTH_ENTERTAINMENT: f32 = 0.02;

/// Generic relief for riding any Attraction (no per-template entry for entertainment).
pub const ENTERTAINMENT_RELIEF: f32 = 40.0;

/// Comfort threshold per need, drawn at spawn as `DEFAULT ± VARIANCE`.
pub const COMFORT_THRESHOLD_DEFAULT: f32 = 70.0;
pub const COMFORT_THRESHOLD_VARIANCE: f32 = 10.0;

/// Exponent of the convex satisfaction penalty, and the EMA recency weight.
pub const SATISFACTION_PENALTY_EXPONENT: f32 = 2.0;
pub const SATISFACTION_RECENCY_WEIGHT: f32 = 0.1;

/// Cumulative satisfaction below this triggers an early departure.
pub const EARLY_DEPARTURE_SATISFACTION_THRESHOLD: f32 = -50.0;

/// Neutral affinity baseline (no bonus/malus).
pub const AFFINITY_DEFAULT: f32 = 1.0;

/// Novelty multiplier right after a visit, and ticks to linearly recover to 1.0.
pub const NOVELTY_FLOOR: f32 = 0.3;
pub const NOVELTY_RECOVERY_TICKS: f32 = 200.0;

/// Stall detection/recovery: symmetric head-on repulsion can deadlock with no damping,
/// so a visitor stuck for `STALL_TICKS_THRESHOLD` gets a small random lateral impulse.
pub const STALL_DISTANCE_EPSILON: f32 = 0.01;
pub const STALL_TICKS_THRESHOLD: u64 = 10;
pub const UNSTALL_IMPULSE_MAGNITUDE: f32 = 0.1;

// Calibrated
