pub const VISITOR_RADIUS: f32 = 0.10;
pub const AVOIDING_RADIUS: f32 = 0.2;

pub const DENSITY_CAP: usize = 5;

pub const SPEED_FLOOR_AT_MAX_DENSITY: f32 = 0.1;
pub const STEERING_FACTOR: f32 = 0.5;

pub const VISIT_DURATION_TICKS: u64 = 500;

pub const TICK_INTERVAL: f32 = 0.05; // seconds
pub const SPAWN_INTERVAL_TICKS: u64 = 20; // ticks

/// 🔶 esquissé — coût de déplacement horizontal par case, selon le type
/// d'infrastructure (pas le terrain). Voir Wiki des Formules §Déplacement des visiteurs.
pub const MOVEMENT_COST_PATH: u32 = 1;
pub const MOVEMENT_COST_RAMP: u32 = 2;
pub const MOVEMENT_COST_STAIRS: u32 = 3;
