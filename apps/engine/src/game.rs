use std::collections::{HashMap, HashSet};

use rand::Rng;

use crate::{
    balance::{
        AFFINITY_DEFAULT, COMFORT_THRESHOLD_DEFAULT, DENSITY_CAP, DETOUR_BIAS_STRENGTH,
        DETOUR_DENSITY_THRESHOLD, DETOUR_LOOKAHEAD_CELLS, ENTERTAINMENT_RELIEF,
        SPAWN_INTERVAL_TICKS, STALL_DISTANCE_EPSILON, STALL_TICKS_THRESHOLD,
        UNSTALL_IMPULSE_MAGNITUDE,
    },
    building_template::{BuildingCatalog, BuildingCategory, CatalogSource},
    map::{Bounds3d, BuildingId, ParkMap, base_speed_for},
    queue::QueueState,
    visitor::{
        ENTERTAINMENT, Visitor, VisitorId, affinity_for, gain_for, grow_needs, lane_bias_strength,
        lateral_repulsion_factor_for, novelty_for, penalty_for, perpendicular_of, relieve_need,
        repulsion_force, score_for, speed_at, update_satisfaction, utility_for, weighted_lane_bias,
    },
};

#[derive(Debug, Default)]
pub struct ParkMetricsAccumulator {
    pub visitors_in_park: usize,
    pub visitors_exited: u64,
}

pub struct GameWorld {
    pub park_map: ParkMap,
    pub building_catalog: BuildingCatalog,
    pub balance: f64,
    pub tick_count: u64,
    pub visitors: Vec<Visitor>,
    pub density: HashMap<(i32, i32, i32), Vec<VisitorId>>,
    pub dirty_chunks: HashSet<(i32, i32)>,
    pub metrics: ParkMetricsAccumulator,
    pub paused: bool,
    /// Keyed by attraction `BuildingId.building_id`. See `queue::QueueState`.
    pub queues: HashMap<String, QueueState>,
}

impl Default for GameWorld {
    fn default() -> Self {
        Self::new()
    }
}

impl GameWorld {
    pub fn new() -> Self {
        Self {
            park_map: ParkMap::new(
                "default".into(), //To replace with a parkmap preloaded
                Bounds3d::new(0, 50, 0, 30, -1, 1),
            ),
            building_catalog: BuildingCatalog::load(CatalogSource::Embedded(include_str!(
                "../assets/catalog/buildings.json"
            )))
            .expect("embedded buildings.json should always be valid"),
            balance: 1000.0,
            tick_count: 0,
            visitors: vec![],
            density: HashMap::new(),
            dirty_chunks: HashSet::new(),
            metrics: ParkMetricsAccumulator::default(),
            paused: false,
            queues: HashMap::new(),
        }
    }

    fn despawn_visitors_who_reached_exit(&mut self) {
        let mut exited_count = 0u64;
        self.visitors.retain(|v| {
            let should_exist = v.is_leaving && v.path.is_empty();
            if should_exist {
                let cell = (
                    v.position.0.round() as i32,
                    v.position.1.round() as i32,
                    v.position.2.round() as i32,
                );
                if let Some(bucket) = self.density.get_mut(&cell) {
                    bucket.retain(|id| id != &v.id);
                    if bucket.is_empty() {
                        self.density.remove(&cell);
                    }
                }
                exited_count += 1;
            }
            !should_exist
        });

        self.metrics.visitors_exited += exited_count;
    }

    pub fn tick(&mut self, dt: f32) {
        // Real game loop of the core game
        if self.paused {
            return;
        }
        self.dirty_chunks.clear();
        let positions: HashMap<VisitorId, (f32, f32, f32)> = self
            .visitors
            .iter()
            .map(|v| (v.id.clone(), v.position))
            .collect();
        let exit = self.park_map.entrance;

        for v in self.visitors.iter_mut() {
            v.ticks_since_spawn += 1;
            let old_cell = cell_of(v.position);
            let position_before_advance = v.position;

            redirect_if_expired(v, &self.park_map, old_cell, exit);
            redirect_if_leaving_early(v, &self.park_map, old_cell, exit);
            assign_new_target_if_arrived(
                v,
                &self.park_map,
                &self.building_catalog,
                old_cell,
                self.tick_count,
            );
            recompute_path_if_blocked(v, &self.park_map, old_cell);

            let local_density = local_density_at(&self.density, old_cell);
            let speed = compute_speed(&self.park_map, local_density, old_cell);
            let mut repulsion = compute_repulsion(v, &self.density, &positions, old_cell);
            repulsion = add_vec(
                repulsion,
                compute_lane_bias(v, &self.park_map, &self.density, old_cell),
            );
            repulsion = add_vec(
                repulsion,
                compute_detour_bias(v, &self.park_map, &self.density),
            );
            let lateral_factor = lateral_repulsion_factor_for(local_density);
            v.advance(speed, dt, repulsion, lateral_factor);
            clamp_to_walkable_ground(v, &self.park_map, position_before_advance);

            let moved = distance_moved(position_before_advance, v.position);
            update_stall_tracking(v, &self.park_map, speed, moved);

            update_needs_and_satisfaction(v, moved);

            let new_cell = cell_of(v.position);
            update_density_and_dirty_chunks(
                &mut self.density,
                &mut self.dirty_chunks,
                &v.id,
                old_cell,
                new_cell,
            );
        }
        self.despawn_visitors_who_reached_exit();

        self.metrics.visitors_in_park = self.visitors.len();
        self.tick_count += 1;

        if self.visitors.len() < 20 && self.tick_count.is_multiple_of(SPAWN_INTERVAL_TICKS) {
            self.spawn_visitor();
        }
    }

    pub fn spawn_visitor(&mut self) {
        let Some(entrance) = self.park_map.entrance else {
            return;
        };

        let target = self
            .park_map
            .random_walkable_cell(entrance)
            .unwrap_or(entrance);
        let path = self.park_map.path_excluding_start(entrance, target);

        let id = uuid::Uuid::new_v4().to_string();

        let mut visitor = Visitor::new(
            id.clone(),
            (entrance.0 as f32, entrance.1 as f32, entrance.2 as f32),
        );
        visitor.path = path;
        visitor.target = target;
        self.visitors.push(visitor);

        self.density.entry(entrance).or_default().push(id);
    }

    pub fn reset_visitors(&mut self) {
        self.visitors.clear();
        self.density.clear();
    }

    /// Recomputes `attraction_building_id`'s cached chain from the current map
    /// geometry, preserving its occupants — call whenever queue infrastructure changes.
    pub fn sync_queue_chain(&mut self, attraction_building_id: &str) {
        let chain = crate::queue::derive_queue_chain(&self.park_map, attraction_building_id);
        self.queues
            .entry(attraction_building_id.to_string())
            .or_default()
            .chain = chain;
    }
}

fn cell_of(position: (f32, f32, f32)) -> (i32, i32, i32) {
    (
        position.0.round() as i32,
        position.1.round() as i32,
        position.2.round() as i32,
    )
}

fn distance_moved(before: (f32, f32, f32), after: (f32, f32, f32)) -> f32 {
    let dx = after.0 - before.0;
    let dy = after.1 - before.1;
    let dz = after.2 - before.2;
    (dx * dx + dy * dy + dz * dz).sqrt()
}

fn is_walkable_at(park_map: &ParkMap, position: (f32, f32, f32)) -> bool {
    let cell = cell_of(position);
    park_map.is_walkable(cell.0, cell.1, cell.2)
}

/// Slides along a single axis if the full move left walkable ground, only fully
/// reverting to `before` if neither axis alone works either.
fn clamp_to_walkable_ground(v: &mut Visitor, park_map: &ParkMap, before: (f32, f32, f32)) {
    let after = v.position;
    if is_walkable_at(park_map, after) {
        return;
    }

    let x_only = (after.0, before.1, before.2);
    if is_walkable_at(park_map, x_only) {
        v.position = x_only;
        return;
    }

    let y_only = (before.0, after.1, before.2);
    if is_walkable_at(park_map, y_only) {
        v.position = y_only;
        return;
    }

    v.position = before;
}

/// Tracks head-on repulsion standoffs: a visitor with a nonzero speed that still
/// barely moved this tick is one tick closer to being considered stalled. Any real
/// progress (or having no path to walk in the first place) resets the counter. Once
/// stalled long enough, nudges the visitor sideways to break the deadlock.
fn update_stall_tracking(v: &mut Visitor, park_map: &ParkMap, speed: f32, moved: f32) {
    if speed > 0.0 && moved < STALL_DISTANCE_EPSILON && !v.path.is_empty() {
        v.stall_ticks += 1;
    } else {
        v.stall_ticks = 0;
    }

    if v.stall_ticks >= STALL_TICKS_THRESHOLD {
        apply_unstall_impulse(v, park_map);
        v.stall_ticks = 0;
    }
}

/// Nudges a stalled visitor by a small random offset, only committing it if the
/// resulting cell is still walkable — breaks a symmetric head-on repulsion deadlock
/// (see `update_stall_tracking`) without a real yielding rule (none exists yet).
fn apply_unstall_impulse(v: &mut Visitor, park_map: &ParkMap) {
    let angle: f32 = rand::thread_rng().gen_range(0.0..std::f32::consts::TAU);
    let candidate = (
        v.position.0 + angle.cos() * UNSTALL_IMPULSE_MAGNITUDE,
        v.position.1 + angle.sin() * UNSTALL_IMPULSE_MAGNITUDE,
        v.position.2,
    );
    let candidate_cell = cell_of(candidate);
    if park_map.is_walkable(candidate_cell.0, candidate_cell.1, candidate_cell.2) {
        v.position = candidate;
    }
}

fn redirect_if_expired(
    v: &mut Visitor,
    park_map: &ParkMap,
    old_cell: (i32, i32, i32),
    exit: Option<(i32, i32, i32)>,
) {
    let Some(exit) = exit else { return };
    if v.has_expired() && !v.is_leaving {
        v.is_leaving = true;
        v.target = exit;
        v.path = park_map.path_excluding_start(old_cell, exit);
    }
}

/// Stricter departure trigger than plain visit expiry: cumulative satisfaction collapsed.
fn redirect_if_leaving_early(
    v: &mut Visitor,
    park_map: &ParkMap,
    old_cell: (i32, i32, i32),
    exit: Option<(i32, i32, i32)>,
) {
    let Some(exit) = exit else { return };
    if v.should_leave_early() && !v.is_leaving {
        v.is_leaving = true;
        v.target = exit;
        v.path = park_map.path_excluding_start(old_cell, exit);
    }
}

/// Grows needs by distance moved, rolls resulting penalties into satisfaction (no gain
/// side here — that's `relieve_needs_at_arrival`).
fn update_needs_and_satisfaction(v: &mut Visitor, distance_moved: f32) {
    grow_needs(&mut v.needs, distance_moved);

    let total_penalty: f32 = v
        .needs
        .iter()
        .map(|(need, &level)| {
            let threshold = v
                .comfort_thresholds
                .get(need)
                .copied()
                .unwrap_or(COMFORT_THRESHOLD_DEFAULT);
            penalty_for(level, threshold)
        })
        .sum();

    v.satisfaction = update_satisfaction(v.satisfaction, 0.0, total_penalty);
}

/// A visitor never stands on a building cell (buildings carry no infrastructure, cf.
/// `is_walkable`), so "using" one means standing on an adjacent walkable cell.
fn adjacent_building(park_map: &ParkMap, cell: (i32, i32, i32)) -> Option<&BuildingId> {
    let (x, y, z) = cell;
    [(1, 0), (-1, 0), (0, 1), (0, -1)]
        .into_iter()
        .find_map(|(dx, dy)| park_map.get_building(x + dx, y + dy, z))
}

/// Applies needs relief for the building adjacent to a just-reached cell (if any) and
/// records the visit for the novelty factor. No-op if nothing is adjacent.
fn relieve_needs_at_arrival(
    v: &mut Visitor,
    park_map: &ParkMap,
    catalog: &BuildingCatalog,
    cell: (i32, i32, i32),
    current_tick: u64,
) {
    let Some(building) = adjacent_building(park_map, cell) else {
        return;
    };
    let Some(template) = catalog.get(&building.template_id) else {
        return;
    };

    let mut total_gain = 0.0;
    for (need, &relief) in &template.needs_relief {
        let level_before = v.needs.get(need).copied().unwrap_or(0.0);
        let threshold = v
            .comfort_thresholds
            .get(need)
            .copied()
            .unwrap_or(COMFORT_THRESHOLD_DEFAULT);
        total_gain += gain_for(relief as f32, level_before, threshold);
        relieve_need(&mut v.needs, need, relief as f32);
    }

    // Entertainment isn't declared per-template like the other needs — any Attraction
    // relieves it generically (Wiki des Formules §Besoins et satisfaction).
    if template.category == BuildingCategory::Attraction {
        let level_before = v.needs.get(ENTERTAINMENT).copied().unwrap_or(0.0);
        let threshold = v
            .comfort_thresholds
            .get(ENTERTAINMENT)
            .copied()
            .unwrap_or(COMFORT_THRESHOLD_DEFAULT);
        total_gain += gain_for(ENTERTAINMENT_RELIEF, level_before, threshold);
        relieve_need(&mut v.needs, ENTERTAINMENT, ENTERTAINMENT_RELIEF);
    }

    v.satisfaction = update_satisfaction(v.satisfaction, total_gain, 0.0);
    v.last_visited.insert(cell, current_tick);
}

/// Utility of a template for `v`: `needs_relief` dot product, plus the same generic
/// entertainment term `relieve_needs_at_arrival` grants on arrival for any Attraction
/// (catalog templates leave `needs_relief` empty for entertainment) — without this,
/// no Attraction could ever outscore plain wandering in `best_destination`.
fn template_utility(v: &Visitor, template: &crate::building_template::BuildingTemplate) -> f32 {
    let mut utility = utility_for(&v.needs, &template.needs_relief);
    if template.category == BuildingCategory::Attraction {
        utility += v.needs.get(ENTERTAINMENT).copied().unwrap_or(0.0) * ENTERTAINMENT_RELIEF;
    }
    utility
}

/// Picks the reachable cell with the best destination score. Only strictly positive
/// scores count — a cell with no relevant building never outscores wandering.
fn best_destination(
    v: &Visitor,
    park_map: &ParkMap,
    catalog: &BuildingCatalog,
    old_cell: (i32, i32, i32),
    current_tick: u64,
) -> Option<(i32, i32, i32)> {
    park_map
        .infrastructure
        .keys()
        .filter(|&&cell| cell != old_cell)
        .filter_map(|&cell| {
            let (_, cost) = park_map.find_path(old_cell, cell)?;
            let template = adjacent_building(park_map, cell)
                .and_then(|building| catalog.get(&building.template_id));
            let utility = template.map(|t| template_utility(v, t)).unwrap_or(0.0);
            let affinity = template
                .map(|template| affinity_for(&v.profile, &template.tags))
                .unwrap_or(AFFINITY_DEFAULT);
            let novelty = novelty_for(v.last_visited.get(&cell).copied(), current_tick);
            let score = score_for(utility, affinity, novelty, cost as f32);
            Some((cell, score))
        })
        .filter(|&(_, score)| score > 0.0)
        .max_by(|a, b| a.1.total_cmp(&b.1))
        .map(|(cell, _)| cell)
}

fn assign_new_target_if_arrived(
    v: &mut Visitor,
    park_map: &ParkMap,
    catalog: &BuildingCatalog,
    old_cell: (i32, i32, i32),
    current_tick: u64,
) {
    if v.path.is_empty() && !v.is_leaving {
        relieve_needs_at_arrival(v, park_map, catalog, old_cell, current_tick);

        let new_target = best_destination(v, park_map, catalog, old_cell, current_tick)
            .or_else(|| park_map.random_walkable_cell(old_cell))
            .unwrap_or(old_cell);
        v.target = new_target;
        v.path = park_map.path_excluding_start(old_cell, new_target);
    }
}

fn recompute_path_if_blocked(v: &mut Visitor, park_map: &ParkMap, old_cell: (i32, i32, i32)) {
    if let Some(&next) = v.path.first()
        && !park_map.is_walkable(next.0, next.1, next.2)
    {
        v.path = park_map.path_excluding_start(old_cell, v.target);
    }
}

fn local_density_at(
    density: &HashMap<(i32, i32, i32), Vec<VisitorId>>,
    cell: (i32, i32, i32),
) -> usize {
    density.get(&cell).map(|bucket| bucket.len()).unwrap_or(0)
}

fn compute_speed(park_map: &ParkMap, local_density: usize, cell: (i32, i32, i32)) -> f32 {
    let base_speed = park_map
        .get_infrastructure(cell.0, cell.1, cell.2)
        .map(base_speed_for)
        .unwrap_or(0.0);
    speed_at(base_speed, local_density)
}

fn compute_repulsion(
    v: &Visitor,
    density: &HashMap<(i32, i32, i32), Vec<VisitorId>>,
    positions: &HashMap<VisitorId, (f32, f32, f32)>,
    cell: (i32, i32, i32),
) -> (f32, f32, f32) {
    let mut repulsion = (0.0, 0.0, 0.0);
    for dx in -1..=1 {
        for dy in -1..=1 {
            let neighbor_cell = (cell.0 + dx, cell.1 + dy, cell.2);
            if let Some(bucket) = density.get(&neighbor_cell) {
                for other_id in bucket.iter().take(DENSITY_CAP) {
                    if *other_id == v.id {
                        continue;
                    }
                    if let Some(&other_pos) = positions.get(other_id) {
                        let force = repulsion_force(v.position, other_pos);
                        repulsion.0 += force.0;
                        repulsion.1 += force.1;
                        repulsion.2 += force.2;
                    }
                }
            }
        }
    }
    repulsion
}

fn add_vec(a: (f32, f32, f32), b: (f32, f32, f32)) -> (f32, f32, f32) {
    (a.0 + b.0, a.1 + b.1, a.2 + b.2)
}

/// Steers a visitor toward whichever of the two cells to either side of its current
/// direction of travel is less crowded — proactive spreading across a wide path,
/// unlike `compute_repulsion` which only reacts within `AVOIDING_RADIUS` (a visitor
/// walking single-file down the centre of a 2-3-wide path can stay just far enough
/// from anyone else to never trigger that at all). Reuses the same per-cell `density`
/// map rather than a new distance-based sensing radius.
fn compute_lane_bias(
    v: &Visitor,
    park_map: &ParkMap,
    density: &HashMap<(i32, i32, i32), Vec<VisitorId>>,
    cell: (i32, i32, i32),
) -> (f32, f32, f32) {
    let Some(&next) = v.path.first() else {
        return (0.0, 0.0, 0.0);
    };
    let next_f = (next.0 as f32, next.1 as f32, next.2 as f32);
    let forward = crate::visitor::direction(v.position, next_f);
    let Some(perp) = perpendicular_of(forward) else {
        return (0.0, 0.0, 0.0);
    };

    let left_cell = (
        cell.0 + perp.0.round() as i32,
        cell.1 + perp.1.round() as i32,
        cell.2,
    );
    let right_cell = (
        cell.0 - perp.0.round() as i32,
        cell.1 - perp.1.round() as i32,
        cell.2,
    );

    let density_if_walkable = |c: (i32, i32, i32)| -> Option<usize> {
        park_map
            .is_walkable(c.0, c.1, c.2)
            .then(|| local_density_at(density, c))
    };

    let strength = lane_bias_strength(
        density_if_walkable(left_cell),
        density_if_walkable(right_cell),
    );
    (perp.0 * strength, perp.1 * strength, 0.0)
}

/// Anticipatory counterpart to `compute_lane_bias`: scans ahead on `path` for a jam and
/// steers toward the less-congested parallel cell, without touching `path` itself.
fn compute_detour_bias(
    v: &Visitor,
    park_map: &ParkMap,
    density: &HashMap<(i32, i32, i32), Vec<VisitorId>>,
) -> (f32, f32, f32) {
    let Some(&next) = v.path.first() else {
        return (0.0, 0.0, 0.0);
    };
    let next_f = (next.0 as f32, next.1 as f32, next.2 as f32);
    let forward = crate::visitor::direction(v.position, next_f);
    let Some(perp) = perpendicular_of(forward) else {
        return (0.0, 0.0, 0.0);
    };
    let perp_step = (perp.0.round() as i32, perp.1.round() as i32);

    let density_if_walkable = |c: (i32, i32, i32)| -> Option<usize> {
        park_map
            .is_walkable(c.0, c.1, c.2)
            .then(|| local_density_at(density, c))
    };

    let mut bias = 0.0;
    for (i, &ahead) in v.path.iter().take(DETOUR_LOOKAHEAD_CELLS).enumerate() {
        if local_density_at(density, ahead) < DETOUR_DENSITY_THRESHOLD {
            continue;
        }
        let left = (ahead.0 + perp_step.0, ahead.1 + perp_step.1, ahead.2);
        let right = (ahead.0 - perp_step.0, ahead.1 - perp_step.1, ahead.2);
        let weight = 1.0 / (i + 1) as f32; // closer congestion steers harder
        bias += weighted_lane_bias(
            density_if_walkable(left),
            density_if_walkable(right),
            DETOUR_BIAS_STRENGTH,
        ) * weight;
    }

    (perp.0 * bias, perp.1 * bias, 0.0)
}

fn update_density_and_dirty_chunks(
    density: &mut HashMap<(i32, i32, i32), Vec<VisitorId>>,
    dirty_chunks: &mut HashSet<(i32, i32)>,
    visitor_id: &VisitorId,
    old_cell: (i32, i32, i32),
    new_cell: (i32, i32, i32),
) {
    if new_cell == old_cell {
        return;
    }
    if let Some(bucket) = density.get_mut(&old_cell) {
        bucket.retain(|id| id != visitor_id);
        if bucket.is_empty() {
            density.remove(&old_cell);
        }
    }
    density
        .entry(new_cell)
        .or_default()
        .push(visitor_id.clone());
    dirty_chunks.insert((new_cell.0, new_cell.1));
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::map::InfrastructureShape;

    #[test]
    fn test_game_world_starts_with_empty_metrics() {
        let world = GameWorld::new();
        assert_eq!(world.metrics.visitors_in_park, 0);
        assert_eq!(world.metrics.visitors_exited, 0);
    }

    #[test]
    fn test_game_world_starts_with_default_balance() {
        let world = GameWorld::new();
        assert_eq!(world.balance, 1000.0);
    }

    mod spawn_visitor {
        use super::*;
        #[test]
        fn test_game_world_initialization() {
            let world = GameWorld::new();
            assert_eq!(world.tick_count, 0);
            assert!(world.park_map.terrain.is_empty());
        }

        #[test]
        fn test_game_world_single_tick() {
            let mut world = GameWorld::new();

            // On déclenche manuellement un tick sans lancer la boucle infinie
            world.tick(0.05);
            assert_eq!(world.tick_count, 1);

            world.tick(0.05);
            assert_eq!(world.tick_count, 2);
        }

        #[test]
        fn test_spawn_visitor_does_nothing_without_entrance() {
            let mut world = GameWorld::new();
            // park_map.entrance is None by default (ParkMap::new)

            world.spawn_visitor();

            assert!(world.visitors.is_empty());
            assert!(world.density.is_empty());
        }

        #[test]
        fn test_spawn_visitor_falls_back_to_entrance_when_no_other_cell_is_walkable() {
            let mut world = GameWorld::new();
            world.park_map.entrance = Some((5, 3, 0));
            // aucune infrastructure posée : random_walkable_cell ne trouve rien d'autre

            world.spawn_visitor();

            let visitor = &world.visitors[0];
            assert_eq!(visitor.position, (5.0, 3.0, 0.0));
            assert_eq!(visitor.target, (5, 3, 0));
            assert_eq!(visitor.path, vec![]);
        }

        #[test]
        fn test_spawn_visitor_computes_target_and_path_when_another_cell_is_walkable() {
            let mut world = GameWorld::new();
            world.park_map.entrance = Some((0, 0, 0));
            world
                .park_map
                .set_infrastructure(0, 0, 0, InfrastructureShape::Path);
            world
                .park_map
                .set_infrastructure(1, 0, 0, InfrastructureShape::Path);

            world.spawn_visitor();

            let visitor = &world.visitors[0];
            assert_eq!(visitor.position, (0.0, 0.0, 0.0));
            assert_eq!(visitor.target, (1, 0, 0)); // seul autre candidat possible, déterministe
            assert_eq!(visitor.path, vec![(1, 0, 0)]);
        }

        #[test]
        fn test_spawn_visitor_updates_density_at_entrance() {
            let mut world = GameWorld::new();
            world.park_map.entrance = Some((5, 3, 0));

            world.spawn_visitor();

            let visitor_id = world.visitors[0].id.clone();
            let bucket = world
                .density
                .get(&(5, 3, 0))
                .expect("density bucket should exist");
            assert_eq!(bucket, &vec![visitor_id]);
        }

        #[test]
        fn test_spawn_visitor_twice_accumulates_density_on_same_cell() {
            let mut world = GameWorld::new();
            world.park_map.entrance = Some((0, 0, 0));

            world.spawn_visitor();
            world.spawn_visitor();

            assert_eq!(world.visitors.len(), 2);
            let bucket = world.density.get(&(0, 0, 0)).unwrap();
            assert_eq!(bucket.len(), 2);
        }
    }

    mod tick {
        use crate::balance::VISIT_DURATION_TICKS;

        use super::*;

        #[test]
        fn test_tick_moves_visitor_toward_target() {
            let mut world = GameWorld::new();
            world.park_map.entrance = Some((0, 0, 0));
            world
                .park_map
                .set_infrastructure(0, 0, 0, InfrastructureShape::Path);
            world
                .park_map
                .set_infrastructure(1, 0, 0, InfrastructureShape::Path);

            world.spawn_visitor(); // target = (1,0,0), path = [(1,0,0)]

            world.tick(0.1);

            let visitor = &world.visitors[0];
            // base_speed(Path)=1.0, density=1 (le visiteur lui-même) -> speed_at = 0.8
            // step = 0.8 * 0.1 = 0.08, distance restante = 1.0 -> pas encore arrivé
            assert!(visitor.position.0 > 0.0 && visitor.position.0 < 1.0);
            assert_eq!(visitor.path, vec![(1, 0, 0)]); // pas encore atteint
        }

        #[test]
        fn test_tick_moves_visitor_density_bucket_when_crossing_a_cell() {
            let mut world = GameWorld::new();
            world.park_map.entrance = Some((0, 0, 0));
            world
                .park_map
                .set_infrastructure(0, 0, 0, InfrastructureShape::Path);
            world
                .park_map
                .set_infrastructure(1, 0, 0, InfrastructureShape::Path);

            world.spawn_visitor();

            world.tick(2.0); // dt large : garantit l'arrivée exacte sur (1,0,0) en un seul tick

            let visitor_id = world.visitors[0].id.clone();
            assert_eq!(world.visitors[0].position, (1.0, 0.0, 0.0));
            assert!(
                !world.density.contains_key(&(0, 0, 0)),
                "old cell bucket should be removed once empty"
            );
            assert_eq!(world.density.get(&(1, 0, 0)), Some(&vec![visitor_id]));
        }

        #[test]
        fn test_tick_speed_decreases_with_density_on_current_cell() {
            let mut lone_world = GameWorld::new();
            lone_world
                .park_map
                .set_infrastructure(0, 0, 0, InfrastructureShape::Path);
            lone_world
                .park_map
                .set_infrastructure(5, 0, 0, InfrastructureShape::Path);
            lone_world.visitors.push(Visitor {
                id: "lone".into(),
                position: (0.0, 0.0, 0.0),
                path: vec![(5, 0, 0)],
                target: (5, 0, 0),
                ticks_since_spawn: 0,
                heading: (0.0, 0.0, 0.0),
                is_leaving: false,
                ..Default::default()
            });
            lone_world.density.insert((0, 0, 0), vec!["lone".into()]);

            let mut crowded_world = GameWorld::new();
            crowded_world
                .park_map
                .set_infrastructure(0, 0, 0, InfrastructureShape::Path);
            crowded_world.visitors.push(Visitor {
                id: "v0".into(),
                position: (0.0, 0.0, 0.0),
                path: vec![(5, 0, 0)],
                target: (5, 0, 0),
                ticks_since_spawn: 0,
                heading: (0.0, 0.0, 0.0),
                is_leaving: false,
                ..Default::default()
            });
            // v1/v2/v3 n'existent que dans le seau de densité, pas dans self.visitors :
            // ça isole l'effet de densité sur la vitesse sans bruit de répulsion (positions inconnues -> ignorées).
            crowded_world.density.insert(
                (0, 0, 0),
                vec!["v0".into(), "v1".into(), "v2".into(), "v3".into()],
            );

            lone_world.tick(0.1);
            crowded_world.tick(0.1);

            assert!(
                crowded_world.visitors[0].position.0 < lone_world.visitors[0].position.0,
                "a visitor on a crowded cell should move less than one alone"
            );
        }

        #[test]
        fn test_tick_applies_repulsion_between_visitors_sharing_a_cell() {
            let mut world = GameWorld::new();
            world
                .park_map
                .set_infrastructure(0, 0, 0, InfrastructureShape::Path);
            world
                .park_map
                .set_infrastructure(5, 0, 0, InfrastructureShape::Path);

            world.visitors.push(Visitor {
                id: "a".into(),
                position: (0.0, 0.0, 0.0),
                path: vec![(5, 0, 0)],
                target: (5, 0, 0),
                ticks_since_spawn: 0,
                heading: (0.0, 0.0, 0.0),
                is_leaving: false,
                ..Default::default()
            });
            world.visitors.push(Visitor {
                id: "b".into(),
                position: (0.0, 0.15, 0.0), // within AVOIDING_RADIUS of "a"
                path: vec![],               // stays put, isolates "a"'s reaction to the repulsion
                target: (0, 0, 0),
                ticks_since_spawn: 0,
                heading: (0.0, 0.0, 0.0),
                is_leaving: false,
                ..Default::default()
            });
            world
                .density
                .insert((0, 0, 0), vec!["a".into(), "b".into()]);

            world.tick(0.05);

            let a = world.visitors.iter().find(|v| v.id == "a").unwrap();
            assert!(
                a.position.1 < 0.0,
                "a should be pushed away from b (at +y), got y = {}",
                a.position.1
            );
        }

        #[test]
        fn test_tick_never_leaves_a_visitor_on_an_unwalkable_cell_even_under_strong_repulsion() {
            // Packing neighbors on top of "a" with a large dt maximizes repulsion drift;
            // the tick loop must still leave "a" on walkable ground afterward.
            let mut world = GameWorld::new();
            world
                .park_map
                .set_infrastructure(0, 0, 0, InfrastructureShape::Path);
            world
                .park_map
                .set_infrastructure(1, 0, 0, InfrastructureShape::Path);
            // (0,1,0)/(0,-1,0) are deliberately left unwalkable: a single-wide corridor.

            world.visitors.push(Visitor {
                id: "a".into(),
                position: (0.0, 0.0, 0.0),
                path: vec![(1, 0, 0)],
                target: (1, 0, 0),
                ..Default::default()
            });
            let mut bucket = vec!["a".to_string()];
            for i in 0..4 {
                let id = format!("n{i}");
                world.visitors.push(Visitor {
                    id: id.clone(),
                    position: (0.0, 0.01, 0.0), // packed right next to "a"
                    path: vec![],
                    target: (0, 0, 0),
                    ..Default::default()
                });
                bucket.push(id);
            }
            world.density.insert((0, 0, 0), bucket);

            world.tick(1.0); // large dt to amplify any drift

            let a = world.visitors.iter().find(|v| v.id == "a").unwrap();
            let cell = (
                a.position.0.round() as i32,
                a.position.1.round() as i32,
                a.position.2.round() as i32,
            );
            assert!(
                world.park_map.is_walkable(cell.0, cell.1, cell.2),
                "visitor ended up off the path at {:?}",
                a.position
            );
        }

        #[test]
        fn test_tick_does_not_move_visitor_when_no_infrastructure_at_current_cell() {
            let mut world = GameWorld::new();
            // no infrastructure placed anywhere

            world.visitors.push(Visitor {
                id: "a".into(),
                position: (0.0, 0.0, 0.0),
                path: vec![(5, 0, 0)],
                target: (5, 0, 0),
                ticks_since_spawn: 0,
                heading: (0.0, 0.0, 0.0),
                is_leaving: false,
                ..Default::default()
            });

            world.tick(1.0);

            assert_eq!(world.visitors[0].position, (0.0, 0.0, 0.0));
        }

        #[test]
        fn test_tick_marks_crossed_chunk_as_dirty() {
            let mut world = GameWorld::new();
            world.park_map.entrance = Some((0, 0, 0));
            world
                .park_map
                .set_infrastructure(0, 0, 0, InfrastructureShape::Path);
            world
                .park_map
                .set_infrastructure(1, 0, 0, InfrastructureShape::Path);

            world.spawn_visitor();

            world.tick(2.0); // dt large : traverse bien jusqu'à (1,0,0)

            assert!(world.dirty_chunks.contains(&(1, 0)));
        }

        #[test]
        fn test_tick_clears_dirty_chunks_when_nothing_moves() {
            let mut world = GameWorld::new();
            // pas de visiteurs, rien ne bouge

            world.tick(1.0);

            assert!(world.dirty_chunks.is_empty());
        }

        #[test]
        fn test_tick_recalculates_path_when_next_cell_becomes_impraticable() {
            let mut world = GameWorld::new();
            world
                .park_map
                .set_infrastructure(0, 0, 0, InfrastructureShape::Path);
            world
                .park_map
                .set_infrastructure(1, 0, 0, InfrastructureShape::Path);
            world
                .park_map
                .set_infrastructure(0, 1, 0, InfrastructureShape::Path);
            world
                .park_map
                .set_infrastructure(1, 1, 0, InfrastructureShape::Path);

            world.visitors.push(Visitor {
                id: "a".into(),
                position: (0.0, 0.0, 0.0),
                path: vec![(1, 0, 0)], // stale : cette case va être bloquée juste après
                target: (1, 1, 0),
                ticks_since_spawn: 0,
                heading: (0.0, 0.0, 0.0),
                is_leaving: false,
                ..Default::default()
            });

            world.park_map.remove_infrasture(1, 0, 0); // simule une modification de carte

            world.tick(0.01); // dt petit : on veut juste voir le chemin recalculé, pas l'arrivée

            let visitor = &world.visitors[0];
            assert_ne!(
                visitor.path.first(),
                Some(&(1, 0, 0)),
                "should not still point at the blocked cell"
            );
            assert!(
                !visitor.path.is_empty(),
                "an alternate route exists via (0,1,0)"
            );
        }

        #[test]
        fn test_tick_clears_path_when_target_becomes_unreachable_after_recalculation() {
            let mut world = GameWorld::new();
            world
                .park_map
                .set_infrastructure(0, 0, 0, InfrastructureShape::Path);
            world
                .park_map
                .set_infrastructure(1, 0, 0, InfrastructureShape::Path);

            world.visitors.push(Visitor {
                id: "a".into(),
                position: (0.0, 0.0, 0.0),
                path: vec![(1, 0, 0)],
                target: (1, 0, 0), // la cible elle-même va devenir impraticable, aucune autre route
                ticks_since_spawn: 0,
                heading: (0.0, 0.0, 0.0),
                is_leaving: false,
                ..Default::default()
            });

            world.park_map.remove_infrasture(1, 0, 0);

            world.tick(0.01);

            assert!(world.visitors[0].path.is_empty());
        }

        #[test]
        fn test_tick_syncs_visitors_in_park_metric() {
            let mut world = GameWorld::new();
            world.park_map.entrance = Some((0, 0, 0));
            world
                .park_map
                .set_infrastructure(0, 0, 0, InfrastructureShape::Path);

            world.spawn_visitor();
            world.spawn_visitor();

            world.tick(0.05);

            assert_eq!(world.metrics.visitors_in_park, 2);
        }

        #[test]
        fn test_tick_redirects_expired_visitor_toward_exit() {
            let mut world = GameWorld::new();
            world.park_map.entrance = Some((0, 0, 0));
            world
                .park_map
                .set_infrastructure(0, 0, 0, InfrastructureShape::Path);
            world
                .park_map
                .set_infrastructure(1, 0, 0, InfrastructureShape::Path);

            world.visitors.push(Visitor {
                id: "a".into(),
                position: (1.0, 0.0, 0.0),
                path: vec![],
                target: (1, 0, 0),
                ticks_since_spawn: VISIT_DURATION_TICKS,
                heading: (0.0, 0.0, 0.0),
                is_leaving: false,
                ..Default::default()
            });

            world.tick(0.01);

            let visitor = &world.visitors[0];
            assert!(visitor.is_leaving);
            assert_eq!(visitor.target, (0, 0, 0));
        }

        #[test]
        fn test_tick_grows_needs_and_updates_satisfaction() {
            let mut world = GameWorld::new();
            world
                .park_map
                .set_infrastructure(0, 0, 0, InfrastructureShape::Path);
            world
                .visitors
                .push(Visitor::new("a".into(), (0.0, 0.0, 0.0)));

            world.tick(0.05);

            let visitor = &world.visitors[0];
            for need in crate::visitor::CORE_NEEDS {
                assert!(visitor.needs[need] > 0.0, "{need} should have grown");
            }
        }

        #[test]
        fn test_tick_redirects_dissatisfied_visitor_toward_exit() {
            let mut world = GameWorld::new();
            world.park_map.entrance = Some((0, 0, 0));
            world
                .park_map
                .set_infrastructure(0, 0, 0, InfrastructureShape::Path);
            world
                .park_map
                .set_infrastructure(1, 0, 0, InfrastructureShape::Path);

            let mut visitor = Visitor::new("a".into(), (1.0, 0.0, 0.0));
            visitor.target = (1, 0, 0);
            visitor.satisfaction = crate::balance::EARLY_DEPARTURE_SATISFACTION_THRESHOLD - 1.0;
            world.visitors.push(visitor);

            world.tick(0.01);

            let visitor = &world.visitors[0];
            assert!(visitor.is_leaving);
            assert_eq!(visitor.target, (0, 0, 0));
        }

        #[test]
        fn test_tick_removes_visitor_who_reached_the_exit() {
            let mut world = GameWorld::new();
            world
                .park_map
                .set_infrastructure(0, 0, 0, InfrastructureShape::Path);

            world.visitors.push(Visitor {
                id: "a".into(),
                position: (0.0, 0.0, 0.0),
                path: vec![], // déjà arrivé
                target: (0, 0, 0),
                ticks_since_spawn: 0,
                heading: (0.0, 0.0, 0.0),
                is_leaving: true, // was already leaving
                ..Default::default()
            });
            world.density.insert((0, 0, 0), vec!["a".into()]);

            world.tick(0.01);

            assert!(world.visitors.is_empty());
            assert!(!world.density.contains_key(&(0, 0, 0)));
            assert_eq!(world.metrics.visitors_exited, 1);
        }

        #[test]
        fn test_tick_spawns_a_visitor_every_spawn_interval() {
            let mut world = GameWorld::new();
            world.park_map.entrance = Some((0, 0, 0));
            world
                .park_map
                .set_infrastructure(0, 0, 0, InfrastructureShape::Path);

            for _ in 0..SPAWN_INTERVAL_TICKS - 1 {
                world.tick(0.01);
            }
            assert!(
                world.visitors.is_empty(),
                "no spawn before the interval is reached"
            );

            world.tick(0.01); // atteint exactement SPAWN_INTERVAL_TICKS
            assert_eq!(world.visitors.len(), 1);
        }

        #[test]
        fn test_tick_assigns_a_new_target_when_visitor_arrives_and_is_not_leaving() {
            let mut world = GameWorld::new();
            world
                .park_map
                .set_infrastructure(0, 0, 0, InfrastructureShape::Path);
            world
                .park_map
                .set_infrastructure(1, 0, 0, InfrastructureShape::Path);

            world.visitors.push(Visitor {
                id: "a".into(),
                position: (0.0, 0.0, 0.0),
                path: vec![], // déjà arrivé
                target: (0, 0, 0),
                ticks_since_spawn: 0,
                heading: (0.0, 0.0, 0.0),
                is_leaving: false,
                ..Default::default()
            });

            world.tick(0.01);

            let visitor = &world.visitors[0];
            assert_eq!(visitor.target, (1, 0, 0)); // seul autre candidat, déterministe
            assert_eq!(visitor.path, vec![(1, 0, 0)]);
        }

        #[test]
        fn test_tick_does_not_assign_new_target_when_visitor_is_leaving() {
            let mut world = GameWorld::new();
            world
                .park_map
                .set_infrastructure(0, 0, 0, InfrastructureShape::Path);
            world
                .park_map
                .set_infrastructure(1, 0, 0, InfrastructureShape::Path);

            world.visitors.push(Visitor {
                id: "a".into(),
                position: (0.0, 0.0, 0.0),
                path: vec![],
                target: (0, 0, 0),
                ticks_since_spawn: 0,
                heading: (0.0, 0.0, 0.0),
                is_leaving: true,
                ..Default::default()
            });
            world.density.insert((0, 0, 0), vec!["a".into()]);

            world.tick(0.01);

            // Doit être despawné (path vide + is_leaving), pas recevoir une nouvelle cible.
            assert!(world.visitors.is_empty());
        }
    }

    mod redirect_if_expired {
        use super::*;
        use crate::balance::VISIT_DURATION_TICKS;

        fn expired_visitor_at(position: (i32, i32, i32)) -> Visitor {
            Visitor {
                id: "a".into(),
                position: (position.0 as f32, position.1 as f32, position.2 as f32),
                path: vec![],
                target: position,
                ticks_since_spawn: VISIT_DURATION_TICKS,
                heading: (0.0, 0.0, 0.0),
                is_leaving: false,
                ..Default::default()
            }
        }

        #[test]
        fn test_does_nothing_when_there_is_no_exit() {
            let park_map = ParkMap::new("m".into(), Bounds3d::new(0, 5, 0, 5, -1, 1));
            let mut v = expired_visitor_at((0, 0, 0));

            redirect_if_expired(&mut v, &park_map, (0, 0, 0), None);

            assert!(!v.is_leaving);
        }

        #[test]
        fn test_does_nothing_when_visitor_has_not_expired() {
            let park_map = ParkMap::new("m".into(), Bounds3d::new(0, 5, 0, 5, -1, 1));
            let mut v = expired_visitor_at((0, 0, 0));
            v.ticks_since_spawn = 0;

            redirect_if_expired(&mut v, &park_map, (0, 0, 0), Some((1, 0, 0)));

            assert!(!v.is_leaving);
        }

        #[test]
        fn test_does_not_overwrite_target_when_visitor_is_already_leaving() {
            let park_map = ParkMap::new("m".into(), Bounds3d::new(0, 5, 0, 5, -1, 1));
            let mut v = expired_visitor_at((0, 0, 0));
            v.is_leaving = true;
            v.target = (3, 3, 0);

            redirect_if_expired(&mut v, &park_map, (0, 0, 0), Some((1, 0, 0)));

            assert_eq!(v.target, (3, 3, 0));
        }

        #[test]
        fn test_sets_is_leaving_and_retargets_exit_when_expired() {
            let park_map = ParkMap::new("m".into(), Bounds3d::new(0, 5, 0, 5, -1, 1));
            let mut v = expired_visitor_at((0, 0, 0));

            redirect_if_expired(&mut v, &park_map, (0, 0, 0), Some((4, 4, 0)));

            assert!(v.is_leaving);
            assert_eq!(v.target, (4, 4, 0));
        }

        #[test]
        fn test_computes_path_toward_exit() {
            let mut park_map = ParkMap::new("m".into(), Bounds3d::new(0, 5, 0, 5, -1, 1));
            park_map.set_infrastructure(0, 0, 0, InfrastructureShape::Path);
            park_map.set_infrastructure(1, 0, 0, InfrastructureShape::Path);
            let mut v = expired_visitor_at((0, 0, 0));

            redirect_if_expired(&mut v, &park_map, (0, 0, 0), Some((1, 0, 0)));

            assert_eq!(v.path, vec![(1, 0, 0)]);
        }
    }

    mod redirect_if_leaving_early {
        use super::*;
        use crate::balance::EARLY_DEPARTURE_SATISFACTION_THRESHOLD;

        fn dissatisfied_visitor_at(position: (i32, i32, i32)) -> Visitor {
            Visitor {
                id: "a".into(),
                position: (position.0 as f32, position.1 as f32, position.2 as f32),
                path: vec![],
                target: position,
                satisfaction: EARLY_DEPARTURE_SATISFACTION_THRESHOLD - 1.0,
                is_leaving: false,
                ..Default::default()
            }
        }

        #[test]
        fn test_does_nothing_when_there_is_no_exit() {
            let park_map = ParkMap::new("m".into(), Bounds3d::new(0, 5, 0, 5, -1, 1));
            let mut v = dissatisfied_visitor_at((0, 0, 0));

            redirect_if_leaving_early(&mut v, &park_map, (0, 0, 0), None);

            assert!(!v.is_leaving);
        }

        #[test]
        fn test_does_nothing_when_satisfaction_is_above_the_threshold() {
            let park_map = ParkMap::new("m".into(), Bounds3d::new(0, 5, 0, 5, -1, 1));
            let mut v = dissatisfied_visitor_at((0, 0, 0));
            v.satisfaction = 0.0;

            redirect_if_leaving_early(&mut v, &park_map, (0, 0, 0), Some((1, 0, 0)));

            assert!(!v.is_leaving);
        }

        #[test]
        fn test_does_not_overwrite_target_when_visitor_is_already_leaving() {
            let park_map = ParkMap::new("m".into(), Bounds3d::new(0, 5, 0, 5, -1, 1));
            let mut v = dissatisfied_visitor_at((0, 0, 0));
            v.is_leaving = true;
            v.target = (3, 3, 0);

            redirect_if_leaving_early(&mut v, &park_map, (0, 0, 0), Some((1, 0, 0)));

            assert_eq!(v.target, (3, 3, 0));
        }

        #[test]
        fn test_sets_is_leaving_and_retargets_exit_when_dissatisfied() {
            let park_map = ParkMap::new("m".into(), Bounds3d::new(0, 5, 0, 5, -1, 1));
            let mut v = dissatisfied_visitor_at((0, 0, 0));

            redirect_if_leaving_early(&mut v, &park_map, (0, 0, 0), Some((4, 4, 0)));

            assert!(v.is_leaving);
            assert_eq!(v.target, (4, 4, 0));
        }

        #[test]
        fn test_computes_path_toward_exit() {
            let mut park_map = ParkMap::new("m".into(), Bounds3d::new(0, 5, 0, 5, -1, 1));
            park_map.set_infrastructure(0, 0, 0, InfrastructureShape::Path);
            park_map.set_infrastructure(1, 0, 0, InfrastructureShape::Path);
            let mut v = dissatisfied_visitor_at((0, 0, 0));

            redirect_if_leaving_early(&mut v, &park_map, (0, 0, 0), Some((1, 0, 0)));

            assert_eq!(v.path, vec![(1, 0, 0)]);
        }
    }

    mod distance_moved {
        use super::*;

        #[test]
        fn test_distance_moved_is_zero_when_position_is_unchanged() {
            let result = distance_moved((1.0, 2.0, 0.0), (1.0, 2.0, 0.0));

            assert_eq!(result, 0.0);
        }

        #[test]
        fn test_distance_moved_computes_euclidean_distance() {
            let result = distance_moved((0.0, 0.0, 0.0), (3.0, 4.0, 0.0));

            assert_eq!(result, 5.0);
        }
    }

    mod clamp_to_walkable_ground {
        use super::*;

        fn visitor_at(position: (f32, f32, f32)) -> Visitor {
            Visitor {
                id: "a".into(),
                position,
                ..Default::default()
            }
        }

        #[test]
        fn test_does_nothing_when_the_full_move_is_still_walkable() {
            let mut park_map = ParkMap::new("m".into(), Bounds3d::new(0, 5, 0, 5, -1, 1));
            park_map.set_infrastructure(0, 0, 0, InfrastructureShape::Path);
            let mut v = visitor_at((0.2, 0.1, 0.0)); // rounds to (0,0,0), still on the path

            clamp_to_walkable_ground(&mut v, &park_map, (0.0, 0.0, 0.0));

            assert_eq!(v.position, (0.2, 0.1, 0.0));
        }

        #[test]
        fn test_slides_along_x_when_the_y_progress_alone_would_leave_the_path() {
            // Corridor along x (0,0,0)-(1,0,0): the tick's x progress is legitimate
            // forward movement, the y drift is lateral repulsion pushing off the edge.
            // The full move rounds to (1,1,0), never set walkable — but x alone
            // (1,0,0) is, so the visitor should keep that forward progress.
            let mut park_map = ParkMap::new("m".into(), Bounds3d::new(0, 5, 0, 5, -1, 1));
            park_map.set_infrastructure(0, 0, 0, InfrastructureShape::Path);
            park_map.set_infrastructure(1, 0, 0, InfrastructureShape::Path);
            let mut v = visitor_at((0.8, 0.6, 0.0));

            clamp_to_walkable_ground(&mut v, &park_map, (0.0, 0.0, 0.0));

            assert_eq!(v.position, (0.8, 0.0, 0.0)); // kept the x progress, y clamped back
        }

        #[test]
        fn test_slides_along_y_when_the_x_progress_alone_would_leave_the_path() {
            // Same idea, corridor along y this time: full move rounds to (1,1,0),
            // unwalkable; y alone (0,1,0) is walkable, x alone (1,0,0) isn't.
            let mut park_map = ParkMap::new("m".into(), Bounds3d::new(0, 5, 0, 5, -1, 1));
            park_map.set_infrastructure(0, 0, 0, InfrastructureShape::Path);
            park_map.set_infrastructure(0, 1, 0, InfrastructureShape::Path);
            let mut v = visitor_at((0.6, 0.8, 0.0));

            clamp_to_walkable_ground(&mut v, &park_map, (0.0, 0.0, 0.0));

            assert_eq!(v.position, (0.0, 0.8, 0.0)); // kept the y progress, x clamped back
        }

        #[test]
        fn test_reverts_fully_when_neither_axis_alone_is_walkable() {
            // Only (0,0,0) is walkable: both (1,0,0) and (0,1,0) are off-path.
            let mut park_map = ParkMap::new("m".into(), Bounds3d::new(0, 5, 0, 5, -1, 1));
            park_map.set_infrastructure(0, 0, 0, InfrastructureShape::Path);
            let mut v = visitor_at((0.6, 0.6, 0.0));

            clamp_to_walkable_ground(&mut v, &park_map, (0.1, 0.1, 0.0));

            assert_eq!(v.position, (0.1, 0.1, 0.0));
        }
    }

    mod update_stall_tracking {
        use super::*;

        fn visitor_with_path() -> Visitor {
            Visitor {
                id: "a".into(),
                position: (0.0, 0.0, 0.0),
                path: vec![(1, 0, 0)],
                ..Default::default()
            }
        }

        #[test]
        fn test_increments_stall_ticks_when_speed_is_positive_but_movement_is_negligible() {
            let park_map = ParkMap::new("m".into(), Bounds3d::new(0, 5, 0, 5, -1, 1));
            let mut v = visitor_with_path();

            update_stall_tracking(&mut v, &park_map, 1.0, 0.0);

            assert_eq!(v.stall_ticks, 1);
        }

        #[test]
        fn test_resets_stall_ticks_on_real_progress() {
            let park_map = ParkMap::new("m".into(), Bounds3d::new(0, 5, 0, 5, -1, 1));
            let mut v = visitor_with_path();
            v.stall_ticks = 5;

            update_stall_tracking(&mut v, &park_map, 1.0, 1.0);

            assert_eq!(v.stall_ticks, 0);
        }

        #[test]
        fn test_resets_stall_ticks_when_speed_is_zero() {
            // A visitor with no infrastructure under them (speed 0) isn't "stalled by
            // another visitor" — that's a different, already-handled case.
            let park_map = ParkMap::new("m".into(), Bounds3d::new(0, 5, 0, 5, -1, 1));
            let mut v = visitor_with_path();
            v.stall_ticks = 5;

            update_stall_tracking(&mut v, &park_map, 0.0, 0.0);

            assert_eq!(v.stall_ticks, 0);
        }

        #[test]
        fn test_does_not_count_a_visitor_with_no_path_as_stalled() {
            let park_map = ParkMap::new("m".into(), Bounds3d::new(0, 5, 0, 5, -1, 1));
            let mut v = visitor_with_path();
            v.path = vec![];
            v.stall_ticks = 5;

            update_stall_tracking(&mut v, &park_map, 1.0, 0.0);

            assert_eq!(v.stall_ticks, 0);
        }

        #[test]
        fn test_applies_an_impulse_and_resets_the_counter_once_the_threshold_is_reached() {
            let mut park_map = ParkMap::new("m".into(), Bounds3d::new(0, 5, 0, 5, -1, 1));
            park_map.set_infrastructure(0, 0, 0, InfrastructureShape::Path);
            let mut v = visitor_with_path();
            v.stall_ticks = STALL_TICKS_THRESHOLD - 1;

            update_stall_tracking(&mut v, &park_map, 1.0, 0.0);

            assert_eq!(v.stall_ticks, 0);
        }
    }

    mod apply_unstall_impulse {
        use super::*;

        #[test]
        fn test_moves_within_the_impulse_magnitude_when_the_nudge_stays_walkable() {
            // A wide-open walkable area: any angle keeps the candidate cell walkable.
            let mut park_map = ParkMap::new("m".into(), Bounds3d::new(-5, 5, -5, 5, -1, 1));
            for x in -2..=2 {
                for y in -2..=2 {
                    park_map.set_infrastructure(x, y, 0, InfrastructureShape::Path);
                }
            }
            let mut v = Visitor {
                id: "a".into(),
                position: (0.0, 0.0, 0.0),
                ..Default::default()
            };

            apply_unstall_impulse(&mut v, &park_map);

            let moved = distance_moved((0.0, 0.0, 0.0), v.position);
            assert!(moved > 0.0, "the visitor should have been nudged");
            assert!(moved <= UNSTALL_IMPULSE_MAGNITUDE + 1e-5);
        }

        #[test]
        fn test_never_commits_a_move_that_leaves_walkable_ground_even_near_an_edge() {
            // Only (0,0,0) is walkable — (1,0,0) deliberately isn't. Starting close to
            // that edge means some (but not all) random angles would cross it; run
            // enough trials to exercise both the commit and the rejection branch, and
            // check the invariant holds every time rather than asserting one outcome.
            let mut park_map = ParkMap::new("m".into(), Bounds3d::new(0, 5, 0, 5, -1, 1));
            park_map.set_infrastructure(0, 0, 0, InfrastructureShape::Path);

            for _ in 0..200 {
                let mut v = Visitor {
                    id: "a".into(),
                    position: (0.42, 0.0, 0.0),
                    ..Default::default()
                };

                apply_unstall_impulse(&mut v, &park_map);

                let cell = (
                    v.position.0.round() as i32,
                    v.position.1.round() as i32,
                    v.position.2.round() as i32,
                );
                assert!(
                    park_map.is_walkable(cell.0, cell.1, cell.2),
                    "landed at {:?}",
                    v.position
                );
            }
        }
    }

    mod update_needs_and_satisfaction {
        use super::*;

        #[test]
        fn test_grows_every_core_need() {
            let mut v = Visitor::new("a".into(), (0.0, 0.0, 0.0));

            update_needs_and_satisfaction(&mut v, 0.0);

            for need in crate::visitor::CORE_NEEDS {
                assert!(v.needs[need] > 0.0, "{need} should have grown");
            }
        }

        #[test]
        fn test_satisfaction_stays_neutral_while_all_needs_are_comfortable() {
            let mut v = Visitor::new("a".into(), (0.0, 0.0, 0.0));

            update_needs_and_satisfaction(&mut v, 0.0);

            // A single tick of growth from 0 stays far under any comfort threshold,
            // so no penalty should apply yet.
            assert_eq!(v.satisfaction, 0.0);
        }

        #[test]
        fn test_satisfaction_drops_once_a_need_crosses_its_comfort_threshold() {
            let mut v = Visitor::new("a".into(), (0.0, 0.0, 0.0));
            for need in crate::visitor::CORE_NEEDS {
                v.needs.insert(need.to_string(), 100.0);
                v.comfort_thresholds.insert(need.to_string(), 70.0);
            }

            update_needs_and_satisfaction(&mut v, 0.0);

            assert!(v.satisfaction < 0.0);
        }

        #[test]
        fn test_missing_comfort_threshold_falls_back_to_the_default() {
            let mut v = Visitor::new("a".into(), (0.0, 0.0, 0.0));
            v.needs.insert(crate::visitor::HUNGER.to_string(), 100.0);
            v.comfort_thresholds.remove(crate::visitor::HUNGER);

            // Should not panic looking up a missing threshold, and should still
            // penalize the over-threshold need using COMFORT_THRESHOLD_DEFAULT.
            update_needs_and_satisfaction(&mut v, 0.0);

            assert!(v.satisfaction < 0.0);
        }
    }

    mod assign_new_target_if_arrived {
        use super::*;

        fn arrived_visitor() -> Visitor {
            Visitor {
                id: "a".into(),
                position: (0.0, 0.0, 0.0),
                path: vec![],
                target: (0, 0, 0),
                ticks_since_spawn: 0,
                heading: (0.0, 0.0, 0.0),
                is_leaving: false,
                ..Default::default()
            }
        }

        fn empty_catalog() -> BuildingCatalog {
            BuildingCatalog::default()
        }

        fn catalog_with_snack_stand() -> BuildingCatalog {
            BuildingCatalog::load(CatalogSource::Embedded(
                r#"{
                    "templates": [
                        {
                            "template_id": "snack_stand",
                            "name": "Snack Stand",
                            "category": "ShopUtility",
                            "footprint": [[0,0]],
                            "cost": 100,
                            "visitor_behavior": "short_stay",
                            "crossing_flags": { "bridge_above_allowed": false, "tunnel_below_allowed": false },
                            "needs_relief": { "hunger": 40 },
                            "tags": []
                        }
                    ]
                }"#,
            ))
            .unwrap()
        }

        fn catalog_with_thrill_and_family_rides() -> BuildingCatalog {
            BuildingCatalog::load(CatalogSource::Embedded(
                r#"{
                    "templates": [
                        {
                            "template_id": "thrill_ride",
                            "name": "Thrill Ride",
                            "category": "Attraction",
                            "footprint": [[0,0]],
                            "cost": 100,
                            "visitor_behavior": "short_stay",
                            "crossing_flags": { "bridge_above_allowed": false, "tunnel_below_allowed": false },
                            "needs_relief": { "hunger": 10 },
                            "tags": ["thrill"]
                        },
                        {
                            "template_id": "family_ride",
                            "name": "Family Ride",
                            "category": "Attraction",
                            "footprint": [[0,0]],
                            "cost": 100,
                            "visitor_behavior": "short_stay",
                            "crossing_flags": { "bridge_above_allowed": false, "tunnel_below_allowed": false },
                            "needs_relief": { "hunger": 10 },
                            "tags": ["family"]
                        }
                    ]
                }"#,
            ))
            .unwrap()
        }

        fn catalog_with_attraction_with_no_declared_needs_relief() -> BuildingCatalog {
            // Mirrors the real catalog: every Attraction template has an empty
            // `needs_relief` (entertainment relief is only granted on arrival).
            BuildingCatalog::load(CatalogSource::Embedded(
                r#"{
                    "templates": [
                        {
                            "template_id": "coaster",
                            "name": "Coaster",
                            "category": "Attraction",
                            "footprint": [[0,0]],
                            "cost": 100,
                            "visitor_behavior": "long_stay",
                            "crossing_flags": { "bridge_above_allowed": false, "tunnel_below_allowed": false },
                            "needs_relief": {},
                            "tags": ["thrill"]
                        }
                    ]
                }"#,
            ))
            .unwrap()
        }

        #[test]
        fn test_prefers_an_attraction_with_no_declared_needs_relief_over_wandering_when_entertainment_is_urgent() {
            let mut park_map = ParkMap::new("m".into(), Bounds3d::new(0, 5, 0, 5, -1, 1));
            park_map.set_infrastructure(0, 0, 0, InfrastructureShape::Path);
            park_map.set_infrastructure(1, 0, 0, InfrastructureShape::Path); // plain candidate
            park_map.set_infrastructure(2, 0, 0, InfrastructureShape::Path); // adjacent to the coaster
            park_map.set_building(
                3,
                0,
                0,
                BuildingId {
                    building_id: "b1".into(),
                    template_id: "coaster".into(),
                },
            );
            let mut v = arrived_visitor();
            v.needs
                .insert(crate::visitor::ENTERTAINMENT.to_string(), 90.0);

            assign_new_target_if_arrived(
                &mut v,
                &park_map,
                &catalog_with_attraction_with_no_declared_needs_relief(),
                (0, 0, 0),
                0,
            );

            assert_eq!(v.target, (2, 0, 0));
        }

        #[test]
        fn test_prefers_the_building_matching_the_visitors_profile_affinity_when_utility_and_cost_are_equal() {
            let mut park_map = ParkMap::new("m".into(), Bounds3d::new(-5, 5, -5, 5, -1, 1));
            park_map.set_infrastructure(0, 0, 0, InfrastructureShape::Path);
            park_map.set_infrastructure(1, 0, 0, InfrastructureShape::Path); // adjacent to family_ride
            park_map.set_infrastructure(-1, 0, 0, InfrastructureShape::Path); // adjacent to thrill_ride
            park_map.set_building(
                2,
                0,
                0,
                BuildingId {
                    building_id: "family".into(),
                    template_id: "family_ride".into(),
                },
            );
            park_map.set_building(
                -2,
                0,
                0,
                BuildingId {
                    building_id: "thrill".into(),
                    template_id: "thrill_ride".into(),
                },
            );
            let mut v = arrived_visitor();
            v.needs.insert(crate::visitor::HUNGER.to_string(), 90.0);
            v.profile = crate::visitor::visitor_profiles()
                .into_iter()
                .find(|p| p.name == "Ados")
                .unwrap();

            assign_new_target_if_arrived(
                &mut v,
                &park_map,
                &catalog_with_thrill_and_family_rides(),
                (0, 0, 0),
                0,
            );

            assert_eq!(v.target, (-1, 0, 0)); // Ados: thrill affinity beats family affinity
        }

        #[test]
        fn test_does_nothing_when_path_is_not_empty() {
            let park_map = ParkMap::new("m".into(), Bounds3d::new(0, 5, 0, 5, -1, 1));
            let mut v = arrived_visitor();
            v.path = vec![(2, 2, 0)];

            assign_new_target_if_arrived(&mut v, &park_map, &empty_catalog(), (0, 0, 0), 0);

            assert_eq!(v.path, vec![(2, 2, 0)]);
        }

        #[test]
        fn test_does_nothing_when_visitor_is_leaving() {
            let park_map = ParkMap::new("m".into(), Bounds3d::new(0, 5, 0, 5, -1, 1));
            let mut v = arrived_visitor();
            v.is_leaving = true;

            assign_new_target_if_arrived(&mut v, &park_map, &empty_catalog(), (0, 0, 0), 0);

            assert!(v.path.is_empty());
            assert_eq!(v.target, (0, 0, 0));
        }

        #[test]
        fn test_falls_back_to_random_walk_when_no_building_scores_positively() {
            let mut park_map = ParkMap::new("m".into(), Bounds3d::new(0, 5, 0, 5, -1, 1));
            park_map.set_infrastructure(0, 0, 0, InfrastructureShape::Path);
            park_map.set_infrastructure(1, 0, 0, InfrastructureShape::Path);
            let mut v = arrived_visitor();

            assign_new_target_if_arrived(&mut v, &park_map, &empty_catalog(), (0, 0, 0), 0);

            assert_eq!(v.target, (1, 0, 0)); // only other candidate, deterministic
            assert_eq!(v.path, vec![(1, 0, 0)]);
        }

        #[test]
        fn test_stays_immobile_with_no_panic_when_no_other_cell_is_reachable() {
            // Only walkable cell is the visitor's own; must not re-freeze by retargeting itself.
            let mut park_map = ParkMap::new("m".into(), Bounds3d::new(0, 5, 0, 5, -1, 1));
            park_map.set_infrastructure(0, 0, 0, InfrastructureShape::Path);
            let mut v = arrived_visitor();

            assign_new_target_if_arrived(&mut v, &park_map, &empty_catalog(), (0, 0, 0), 0);

            assert!(v.path.is_empty());
        }

        #[test]
        fn test_prefers_a_cell_adjacent_to_a_relevant_building_over_plain_wandering() {
            let mut park_map = ParkMap::new("m".into(), Bounds3d::new(0, 5, 0, 5, -1, 1));
            park_map.set_infrastructure(0, 0, 0, InfrastructureShape::Path);
            park_map.set_infrastructure(1, 0, 0, InfrastructureShape::Path); // plain candidate
            park_map.set_infrastructure(2, 0, 0, InfrastructureShape::Path); // link to keep the path contiguous
            park_map.set_infrastructure(3, 0, 0, InfrastructureShape::Path); // adjacent to the stand
            park_map.set_building(
                4,
                0,
                0,
                BuildingId {
                    building_id: "b1".into(),
                    template_id: "snack_stand".into(),
                },
            );
            let mut v = arrived_visitor();
            v.needs.insert(crate::visitor::HUNGER.to_string(), 90.0); // starving

            assign_new_target_if_arrived(
                &mut v,
                &park_map,
                &catalog_with_snack_stand(),
                (0, 0, 0),
                0,
            );

            assert_eq!(v.target, (3, 0, 0));
        }

        #[test]
        fn test_relieves_needs_and_records_the_visit_on_arrival_next_to_a_building() {
            let mut park_map = ParkMap::new("m".into(), Bounds3d::new(0, 5, 0, 5, -1, 1));
            park_map.set_infrastructure(0, 0, 0, InfrastructureShape::Path);
            park_map.set_building(
                1,
                0,
                0,
                BuildingId {
                    building_id: "b1".into(),
                    template_id: "snack_stand".into(),
                },
            );
            let mut v = arrived_visitor();
            v.needs.insert(crate::visitor::HUNGER.to_string(), 90.0);
            v.comfort_thresholds
                .insert(crate::visitor::HUNGER.to_string(), 70.0);

            assign_new_target_if_arrived(
                &mut v,
                &park_map,
                &catalog_with_snack_stand(),
                (0, 0, 0),
                42,
            );

            assert_eq!(v.needs[crate::visitor::HUNGER], 50.0); // 90 - 40 relief
            assert!(v.satisfaction > 0.0, "relief should have granted a gain");
            assert_eq!(v.last_visited.get(&(0, 0, 0)), Some(&42));
        }
    }

    mod recompute_path_if_blocked {
        use super::*;

        fn visitor_with_path(path: Vec<(i32, i32, i32)>, target: (i32, i32, i32)) -> Visitor {
            Visitor {
                id: "a".into(),
                position: (0.0, 0.0, 0.0),
                path,
                target,
                ticks_since_spawn: 0,
                heading: (0.0, 0.0, 0.0),
                is_leaving: false,
                ..Default::default()
            }
        }

        #[test]
        fn test_does_nothing_when_path_is_empty() {
            let park_map = ParkMap::new("m".into(), Bounds3d::new(0, 5, 0, 5, -1, 1));
            let mut v = visitor_with_path(vec![], (0, 0, 0));

            recompute_path_if_blocked(&mut v, &park_map, (0, 0, 0));

            assert!(v.path.is_empty());
        }

        #[test]
        fn test_does_nothing_when_next_cell_is_still_walkable() {
            let mut park_map = ParkMap::new("m".into(), Bounds3d::new(0, 5, 0, 5, -1, 1));
            park_map.set_infrastructure(0, 0, 0, InfrastructureShape::Path);
            park_map.set_infrastructure(1, 0, 0, InfrastructureShape::Path);
            let mut v = visitor_with_path(vec![(1, 0, 0)], (1, 0, 0));

            recompute_path_if_blocked(&mut v, &park_map, (0, 0, 0));

            assert_eq!(v.path, vec![(1, 0, 0)]);
        }

        #[test]
        fn test_recomputes_path_when_next_cell_becomes_unwalkable() {
            let mut park_map = ParkMap::new("m".into(), Bounds3d::new(0, 5, 0, 5, -1, 1));
            park_map.set_infrastructure(0, 0, 0, InfrastructureShape::Path);
            park_map.set_infrastructure(0, 1, 0, InfrastructureShape::Path);
            park_map.set_infrastructure(1, 1, 0, InfrastructureShape::Path);
            // (1,0,0) is deliberately left without infrastructure: simulates the blocked cell
            let mut v = visitor_with_path(vec![(1, 0, 0)], (1, 1, 0));

            recompute_path_if_blocked(&mut v, &park_map, (0, 0, 0));

            assert_ne!(v.path.first(), Some(&(1, 0, 0)));
            assert!(!v.path.is_empty(), "an alternate route exists via (0,1,0)");
        }

        #[test]
        fn test_clears_path_when_no_alternate_route_exists() {
            let mut park_map = ParkMap::new("m".into(), Bounds3d::new(0, 5, 0, 5, -1, 1));
            park_map.set_infrastructure(0, 0, 0, InfrastructureShape::Path);
            // (1,0,0) is both the target and now unwalkable, no other route exists
            let mut v = visitor_with_path(vec![(1, 0, 0)], (1, 0, 0));

            recompute_path_if_blocked(&mut v, &park_map, (0, 0, 0));

            assert!(v.path.is_empty());
        }
    }

    mod compute_repulsion {
        use super::*;

        fn visitor_at(id: &str, position: (f32, f32, f32)) -> Visitor {
            Visitor {
                id: id.into(),
                position,
                path: vec![],
                target: (0, 0, 0),
                ticks_since_spawn: 0,
                heading: (0.0, 0.0, 0.0),
                is_leaving: false,
                ..Default::default()
            }
        }

        #[test]
        fn test_ignores_neighbors_beyond_density_cap_in_the_same_bucket() {
            let v = visitor_at("a", (0.0, 0.0, 0.0));

            let mut positions: HashMap<VisitorId, (f32, f32, f32)> = HashMap::new();
            let mut bucket_at_cap: Vec<VisitorId> = Vec::new();
            for i in 0..DENSITY_CAP {
                let id = format!("n{i}");
                positions.insert(id.clone(), (0.1, 0.0, 0.0));
                bucket_at_cap.push(id);
            }

            // Same first DENSITY_CAP neighbors, plus extra ones packed into the same cell.
            let mut bucket_over_cap = bucket_at_cap.clone();
            for i in DENSITY_CAP..(DENSITY_CAP + 10) {
                let id = format!("n{i}");
                positions.insert(id.clone(), (0.1, 0.0, 0.0));
                bucket_over_cap.push(id);
            }

            let mut density_at_cap: HashMap<(i32, i32, i32), Vec<VisitorId>> = HashMap::new();
            density_at_cap.insert((0, 0, 0), bucket_at_cap);

            let mut density_over_cap: HashMap<(i32, i32, i32), Vec<VisitorId>> = HashMap::new();
            density_over_cap.insert((0, 0, 0), bucket_over_cap);

            let repulsion_at_cap = compute_repulsion(&v, &density_at_cap, &positions, (0, 0, 0));
            let repulsion_over_cap =
                compute_repulsion(&v, &density_over_cap, &positions, (0, 0, 0));

            assert_eq!(
                repulsion_at_cap, repulsion_over_cap,
                "neighbors beyond DENSITY_CAP in the same bucket should not affect the result"
            );
        }
    }

    mod compute_detour_bias {
        use super::*;

        fn visitor_with_path(path: Vec<(i32, i32, i32)>) -> Visitor {
            Visitor {
                id: "a".into(),
                position: (0.0, 0.0, 0.0),
                path,
                target: (0, 0, 0),
                ..Default::default()
            }
        }

        #[test]
        fn test_zero_bias_when_the_visitor_has_no_path() {
            let park_map = ParkMap::new("m".into(), Bounds3d::new(0, 5, 0, 5, -1, 1));
            let v = visitor_with_path(vec![]);
            let density: HashMap<(i32, i32, i32), Vec<VisitorId>> = HashMap::new();

            let bias = compute_detour_bias(&v, &park_map, &density);

            assert_eq!(bias, (0.0, 0.0, 0.0));
        }

        #[test]
        fn test_zero_bias_when_nothing_ahead_reaches_the_congestion_threshold() {
            let park_map = ParkMap::new("m".into(), Bounds3d::new(0, 5, 0, 5, -1, 1));
            let v = visitor_with_path(vec![(1, 0, 0), (2, 0, 0), (3, 0, 0)]);
            let mut density: HashMap<(i32, i32, i32), Vec<VisitorId>> = HashMap::new();
            density.insert((2, 0, 0), vec!["n0".into(), "n1".into()]); // below DETOUR_DENSITY_THRESHOLD (3)

            let bias = compute_detour_bias(&v, &park_map, &density);

            assert_eq!(bias, (0.0, 0.0, 0.0));
        }

        #[test]
        fn test_steers_toward_the_walkable_side_when_a_cell_ahead_is_congested_and_the_other_side_is_blocked()
         {
            // Straight corridor along x. A jam forms two cells ahead at (2,0,0): the
            // parallel cell on the left, (2,1,0), is open ground; the right, (2,-1,0),
            // was never set walkable at all.
            let mut park_map = ParkMap::new("m".into(), Bounds3d::new(0, 5, -5, 5, -1, 1));
            park_map.set_infrastructure(1, 0, 0, InfrastructureShape::Path);
            park_map.set_infrastructure(2, 0, 0, InfrastructureShape::Path);
            park_map.set_infrastructure(3, 0, 0, InfrastructureShape::Path);
            park_map.set_infrastructure(2, 1, 0, InfrastructureShape::Path);

            let v = visitor_with_path(vec![(1, 0, 0), (2, 0, 0), (3, 0, 0)]);
            let mut density: HashMap<(i32, i32, i32), Vec<VisitorId>> = HashMap::new();
            density.insert(
                (2, 0, 0),
                vec!["n0".into(), "n1".into(), "n2".into()], // at DETOUR_DENSITY_THRESHOLD
            );

            let bias = compute_detour_bias(&v, &park_map, &density);

            assert!(
                bias.1 > 0.0,
                "should steer toward +y (left/open side), got {bias:?}"
            );
        }

        #[test]
        fn test_zero_bias_when_the_only_open_parallel_cell_is_just_as_congested() {
            let mut park_map = ParkMap::new("m".into(), Bounds3d::new(0, 5, -5, 5, -1, 1));
            park_map.set_infrastructure(1, 0, 0, InfrastructureShape::Path);
            park_map.set_infrastructure(2, 0, 0, InfrastructureShape::Path);
            park_map.set_infrastructure(3, 0, 0, InfrastructureShape::Path);
            park_map.set_infrastructure(2, 1, 0, InfrastructureShape::Path);
            park_map.set_infrastructure(2, -1, 0, InfrastructureShape::Path);

            let v = visitor_with_path(vec![(1, 0, 0), (2, 0, 0), (3, 0, 0)]);
            let mut density: HashMap<(i32, i32, i32), Vec<VisitorId>> = HashMap::new();
            let jam = vec!["n0".into(), "n1".into(), "n2".into()];
            density.insert((2, 0, 0), jam.clone());
            density.insert((2, 1, 0), jam.clone());
            density.insert((2, -1, 0), jam);

            let bias = compute_detour_bias(&v, &park_map, &density);

            assert_eq!(bias, (0.0, 0.0, 0.0));
        }

        #[test]
        fn test_closer_congestion_steers_harder_than_farther_congestion() {
            fn corridor() -> ParkMap {
                let mut park_map = ParkMap::new("m".into(), Bounds3d::new(0, 5, -5, 5, -1, 1));
                for x in 1..=5 {
                    park_map.set_infrastructure(x, 0, 0, InfrastructureShape::Path);
                    park_map.set_infrastructure(x, 1, 0, InfrastructureShape::Path);
                }
                park_map
            }
            let near = corridor();
            let far = corridor();

            let path = vec![(1, 0, 0), (2, 0, 0), (3, 0, 0), (4, 0, 0), (5, 0, 0)];
            let v_near = visitor_with_path(path.clone());
            let v_far = visitor_with_path(path);

            let jam = vec!["n0".into(), "n1".into(), "n2".into()];
            let mut density_near: HashMap<(i32, i32, i32), Vec<VisitorId>> = HashMap::new();
            density_near.insert((1, 0, 0), jam.clone()); // 1st cell ahead
            let mut density_far: HashMap<(i32, i32, i32), Vec<VisitorId>> = HashMap::new();
            density_far.insert((5, 0, 0), jam); // last cell in the lookahead window

            let bias_near = compute_detour_bias(&v_near, &near, &density_near);
            let bias_far = compute_detour_bias(&v_far, &far, &density_far);

            assert!(
                bias_near.1 > bias_far.1,
                "congestion right ahead ({bias_near:?}) should steer harder than the same congestion \
                 further out ({bias_far:?})"
            );
        }
    }

    mod update_density_and_dirty_chunks {
        use super::*;

        #[test]
        fn test_does_nothing_when_cell_is_unchanged() {
            let mut density: HashMap<(i32, i32, i32), Vec<VisitorId>> = HashMap::new();
            density.insert((0, 0, 0), vec!["a".into()]);
            let mut dirty_chunks = HashSet::new();

            update_density_and_dirty_chunks(
                &mut density,
                &mut dirty_chunks,
                &"a".to_string(),
                (0, 0, 0),
                (0, 0, 0),
            );

            assert_eq!(density.get(&(0, 0, 0)), Some(&vec!["a".to_string()]));
            assert!(dirty_chunks.is_empty());
        }

        #[test]
        fn test_moves_visitor_to_new_bucket_when_cell_changes() {
            let mut density: HashMap<(i32, i32, i32), Vec<VisitorId>> = HashMap::new();
            density.insert((0, 0, 0), vec!["a".into()]);
            let mut dirty_chunks = HashSet::new();

            update_density_and_dirty_chunks(
                &mut density,
                &mut dirty_chunks,
                &"a".to_string(),
                (0, 0, 0),
                (1, 0, 0),
            );

            assert_eq!(density.get(&(1, 0, 0)), Some(&vec!["a".to_string()]));
        }

        #[test]
        fn test_removes_old_bucket_when_it_becomes_empty() {
            let mut density: HashMap<(i32, i32, i32), Vec<VisitorId>> = HashMap::new();
            density.insert((0, 0, 0), vec!["a".into()]);
            let mut dirty_chunks = HashSet::new();

            update_density_and_dirty_chunks(
                &mut density,
                &mut dirty_chunks,
                &"a".to_string(),
                (0, 0, 0),
                (1, 0, 0),
            );

            assert!(!density.contains_key(&(0, 0, 0)));
        }

        #[test]
        fn test_keeps_old_bucket_when_other_visitors_remain() {
            let mut density: HashMap<(i32, i32, i32), Vec<VisitorId>> = HashMap::new();
            density.insert((0, 0, 0), vec!["a".into(), "b".into()]);
            let mut dirty_chunks = HashSet::new();

            update_density_and_dirty_chunks(
                &mut density,
                &mut dirty_chunks,
                &"a".to_string(),
                (0, 0, 0),
                (1, 0, 0),
            );

            assert_eq!(density.get(&(0, 0, 0)), Some(&vec!["b".to_string()]));
        }

        #[test]
        fn test_marks_new_chunk_as_dirty_when_cell_changes() {
            let mut density: HashMap<(i32, i32, i32), Vec<VisitorId>> = HashMap::new();
            density.insert((0, 0, 0), vec!["a".into()]);
            let mut dirty_chunks = HashSet::new();

            update_density_and_dirty_chunks(
                &mut density,
                &mut dirty_chunks,
                &"a".to_string(),
                (0, 0, 0),
                (1, 0, 0),
            );

            assert!(dirty_chunks.contains(&(1, 0)));
        }
    }
    mod pause_resume {
        use super::*;

        #[test]
        fn test_tick_does_nothing_when_paused() {
            let mut world = GameWorld::new();
            world.park_map.entrance = Some((0, 0, 0));
            world
                .park_map
                .set_infrastructure(0, 0, 0, InfrastructureShape::Path);
            world
                .park_map
                .set_infrastructure(1, 0, 0, InfrastructureShape::Path);
            world.spawn_visitor();
            let position_before = world.visitors[0].position;

            world.paused = true;
            world.tick(0.1);
            world.tick(0.1);

            assert_eq!(world.tick_count, 0);
            assert_eq!(world.visitors[0].position, position_before);
        }

        #[test]
        fn test_tick_resumes_normally_after_being_unpaused() {
            let mut world = GameWorld::new();
            world.park_map.entrance = Some((0, 0, 0));
            world
                .park_map
                .set_infrastructure(0, 0, 0, InfrastructureShape::Path);
            world
                .park_map
                .set_infrastructure(1, 0, 0, InfrastructureShape::Path);
            world.spawn_visitor();

            world.paused = true;
            world.tick(0.1);
            world.paused = false;
            world.tick(0.1);

            assert_eq!(world.tick_count, 1);
        }
    }

    mod reset_visitors {
        use super::*;

        #[test]
        fn test_reset_visitors_clears_visitors_and_density() {
            let mut world = GameWorld::new();
            world
                .park_map
                .set_infrastructure(0, 0, 0, InfrastructureShape::Path);
            world.visitors.push(Visitor {
                id: "a".into(),
                position: (0.0, 0.0, 0.0),
                path: vec![],
                target: (0, 0, 0),
                ticks_since_spawn: 0,
                heading: (0.0, 0.0, 0.0),
                is_leaving: false,
                ..Default::default()
            });
            world.density.insert((0, 0, 0), vec!["a".into()]);

            world.reset_visitors();

            assert!(world.visitors.is_empty());
            assert!(world.density.is_empty());
        }

        #[test]
        fn test_reset_visitors_does_not_touch_map_or_tick_count() {
            let mut world = GameWorld::new();
            world
                .park_map
                .set_infrastructure(0, 0, 0, InfrastructureShape::Path);
            world.tick_count = 42;
            world.visitors.push(Visitor {
                id: "a".into(),
                position: (0.0, 0.0, 0.0),
                path: vec![],
                target: (0, 0, 0),
                ticks_since_spawn: 0,
                heading: (0.0, 0.0, 0.0),
                is_leaving: false,
                ..Default::default()
            });

            world.reset_visitors();

            assert_eq!(world.tick_count, 42);
            assert!(world.park_map.get_infrastructure(0, 0, 0).is_some());
        }
    }

    mod sync_queue_chain {
        use super::*;

        #[test]
        fn test_populates_the_chain_from_the_map_geometry() {
            let mut world = GameWorld::new();
            world.park_map.set_building(
                0,
                0,
                0,
                BuildingId {
                    building_id: "coaster".into(),
                    template_id: "roller_coaster".into(),
                },
            );
            world.park_map.set_infrastructure(
                1,
                0,
                0,
                InfrastructureShape::Queue {
                    attraction_id: BuildingId {
                        building_id: "coaster".into(),
                        template_id: "roller_coaster".into(),
                    },
                },
            );

            world.sync_queue_chain("coaster");

            assert_eq!(world.queues["coaster"].chain, vec![(1, 0, 0)]);
        }

        #[test]
        fn test_preserves_occupants_across_a_resync() {
            let mut world = GameWorld::new();
            world.queues.insert(
                "coaster".to_string(),
                crate::queue::QueueState {
                    chain: vec![],
                    occupants: vec!["v1".to_string()].into(),
                },
            );

            world.sync_queue_chain("coaster");

            assert_eq!(world.queues["coaster"].occupants, vec!["v1".to_string()]);
        }
    }
}
