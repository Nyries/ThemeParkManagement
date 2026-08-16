// PLACEHOLDER
pub const K_REPULSION: f32 = 0.2;

pub const VISITOR_RADIUS: f32 = 0.10;
pub const AVOIDING_RADIUS: f32 = 0.2;

pub const DENSITY_CAP: usize = 5;

/// 🔶 PLACEHOLDER — `speed_at` used to hit exactly 0 once density reached
/// `DENSITY_CAP`, with no floor. A visitor at speed 0 never leaves its cell, so
/// density there can only ever grow (more visitors arriving/spawning) and never
/// recovers — a permanent gridlock, confirmed empirically (`bin/repulsion_diag.rs`).
/// This floor guarantees some crawl even at/above the density cap, so congestion can
/// always disperse instead of freezing solid.
pub const SPEED_FLOOR_AT_MAX_DENSITY: f32 = 0.1;

pub const VISIT_DURATION_TICKS: u64 = 1000;

pub const TICK_INTERVAL: f32 = 0.05; // seconds
pub const SPAWN_INTERVAL_TICKS: u64 = 20; // ticks

// Closer to 1 makes a turn almost instantaneous; 0 is a very progressive turn
pub const STEERING_FACTOR: f32 = 0.3;
pub const LATERAL_REPULSION_FACTOR: f32 = 0.1;

/// 🔶 PLACEHOLDER — lateral repulsion is suppressed to `LATERAL_REPULSION_FACTOR` on
/// an ordinary, uncrowded path (keeps movement looking straight/organic on a 1-wide
/// corridor), but ramps up toward this value as local density approaches/exceeds
/// `DENSITY_CAP` — otherwise visitors never spread out to use a wide path's full
/// width, or step aside to pass each other, even when there's room to.
pub const LATERAL_REPULSION_FACTOR_AT_MAX_DENSITY: f32 = 1.0;

/// 🔶 PLACEHOLDER — the physical repulsion force only reacts within `AVOIDING_RADIUS`
/// (0.2, less than a quarter of a cell), so on a path 2-3 cells wide, visitors walking
/// single-file down the centre lane can stay just far enough apart to never trigger it
/// at all — never discovering the open space on either side. This lane bias looks one
/// cell to either side of the direction of travel (reusing the existing per-cell
/// `density` map, not a new distance-based sensing radius) and steers gently toward
/// whichever neighbouring cell is less crowded, so visitors spread across a wide path
/// proactively instead of only reacting once already shoulder-to-shoulder.
pub const LANE_BIAS_STRENGTH: f32 = 0.4;

/// 🔶 PLACEHOLDER — TPM-182 option A (lightweight detour patch). `compute_detour_bias`
/// scans up to `DETOUR_LOOKAHEAD_CELLS` cells ahead along a visitor's already-computed
/// path for a jam building up further down a wide corridor — `LANE_BIAS_STRENGTH`
/// alone only reacts to the cell immediately beside the visitor's current position, too
/// late to look like a deliberate detour. `DETOUR_DENSITY_THRESHOLD` is deliberately
/// lower than `DENSITY_CAP` so the detour steer kicks in *before* a visitor is already
/// crawling from the density/lateral-repulsion falloff.
pub const DETOUR_LOOKAHEAD_CELLS: usize = 5;
pub const DETOUR_DENSITY_THRESHOLD: usize = 3;
pub const DETOUR_BIAS_STRENGTH: f32 = 0.5;

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

/// 🔶 PLACEHOLDER — head-on repulsion between several visitors converging in a
/// single-wide corridor has no damping (only the lateral component is attenuated,
/// see `LATERAL_REPULSION_FACTOR`) and no yielding rule, so it can cancel out into a
/// stable symmetric deadlock. A visitor stalled (near-zero movement despite a nonzero
/// speed) for `STALL_TICKS_THRESHOLD` consecutive ticks gets a small random lateral
/// impulse of `UNSTALL_IMPULSE_MAGNITUDE` to break the symmetry.
pub const STALL_DISTANCE_EPSILON: f32 = 0.01;
pub const STALL_TICKS_THRESHOLD: u64 = 10;
pub const UNSTALL_IMPULSE_MAGNITUDE: f32 = 0.1;

// Calibrated
