mod destinations;
mod steering;

use destinations::{
    assign_new_target_if_arrived, recompute_path_if_blocked, redirect_if_expired,
    redirect_if_leaving_early, update_needs_and_satisfaction,
};
use steering::{
    clamp_to_walkable_ground, compute_detour_bias, compute_lane_bias, compute_repulsion,
    compute_speed, local_density_at, update_density_and_dirty_chunks, update_stall_tracking,
};

use std::collections::{HashMap, HashSet};

use crate::{
    balance::{SPAWN_INTERVAL_TICKS, VISITOR_RADIUS},
    building_template::{BuildingCatalog, CatalogSource},
    map::{Bounds3d, ParkMap, base_speed_for},
    queue::{QueueState, advance_toward},
    visitor::{Visitor, VisitorId, lateral_repulsion_factor_for},
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
                "../../assets/catalog/buildings.json"
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

            if let Some(attraction_id) = v.queue_attraction.clone() {
                let moved = advance_visitor_in_queue(v, &self.queues, &attraction_id, dt);
                update_needs_and_satisfaction(v, moved);

                let new_cell = cell_of(v.position);
                update_density_and_dirty_chunks(
                    &mut self.density,
                    &mut self.dirty_chunks,
                    &v.id,
                    old_cell,
                    new_cell,
                );
                continue;
            }

            redirect_if_expired(v, &self.park_map, old_cell, exit);
            redirect_if_leaving_early(v, &self.park_map, old_cell, exit);
            assign_new_target_if_arrived(
                v,
                &self.park_map,
                &self.building_catalog,
                &mut self.queues,
                &mut self.balance,
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

    /// Moves a visitor from normal pathfinding-driven movement into an attraction's
    /// queue — false (no-op) if the queue is already full. The caller is expected to
    /// have already checked the visitor is at the chain's tail (queue entrance); the
    /// trigger for that lives with target resolution (TPM-156), not here.
    pub fn try_join_queue(&mut self, visitor_id: &str, attraction_building_id: &str) -> bool {
        let Some(v) = self.visitors.iter_mut().find(|v| v.id == visitor_id) else {
            return false;
        };
        join_queue(v, &mut self.queues, attraction_building_id)
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

fn visitor_diameter() -> f32 {
    2.0 * VISITOR_RADIUS
}

/// Moves a queued visitor toward its FIFO slot along the cached chain — continuous,
/// capped by the same speed as walking a plain path, never A*. Returns distance moved.
fn advance_visitor_in_queue(
    v: &mut Visitor,
    queues: &HashMap<String, QueueState>,
    attraction_id: &str,
    dt: f32,
) -> f32 {
    let Some(queue) = queues.get(attraction_id) else {
        return 0.0;
    };
    let Some(index) = queue.occupants.iter().position(|id| id == &v.id) else {
        return 0.0;
    };
    let Some(target) = queue.target_position(index, visitor_diameter()) else {
        return 0.0;
    };

    let before = v.position;
    // Same base speed as a plain path (Queue's own base_speed_for arm matches it).
    let max_step = base_speed_for(&crate::map::InfrastructureShape::Path) * dt;
    v.position = advance_toward(v.position, target, max_step);
    distance_moved(before, v.position)
}

/// Moves `v` from normal pathfinding-driven movement into `attraction_id`'s queue —
/// false (no-op) if it's already full.
fn join_queue(
    v: &mut Visitor,
    queues: &mut HashMap<String, QueueState>,
    attraction_id: &str,
) -> bool {
    let diameter = visitor_diameter();
    let queue = queues.entry(attraction_id.to_string()).or_default();
    if queue.is_full(diameter) {
        return false;
    }
    queue.occupants.push_back(v.id.clone());
    v.queue_attraction = Some(attraction_id.to_string());
    v.path.clear();
    true
}

/// The attraction whose queue entrance (chain tail) is `cell`, if any — TPM-156: this
/// is how `assign_new_target_if_arrived` recognizes "just arrived at a queue" versus
/// an ordinary destination.
fn queue_at_entrance(queues: &HashMap<String, QueueState>, cell: (i32, i32, i32)) -> Option<&str> {
    queues
        .iter()
        .find(|(_, queue)| queue.chain.last() == Some(&cell))
        .map(|(attraction_id, _)| attraction_id.as_str())
}

fn add_vec(a: (f32, f32, f32), b: (f32, f32, f32)) -> (f32, f32, f32) {
    (a.0 + b.0, a.1 + b.1, a.2 + b.2)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::map::{BuildingId, InfrastructureShape};

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

    mod try_join_queue {
        use super::*;

        fn visitor_at(id: &str, position: (f32, f32, f32)) -> Visitor {
            Visitor {
                id: id.into(),
                position,
                ..Default::default()
            }
        }

        #[test]
        fn test_adds_the_visitor_to_occupants_and_marks_them_as_queued() {
            let mut world = GameWorld::new();
            world.queues.insert(
                "coaster".to_string(),
                crate::queue::QueueState {
                    chain: vec![(0, 0, 0), (10, 0, 0)],
                    occupants: Default::default(),
                },
            );
            world.visitors.push(visitor_at("a", (10.0, 0.0, 0.0)));
            world.visitors[0].path = vec![(5, 5, 0)];

            let joined = world.try_join_queue("a", "coaster");

            assert!(joined);
            assert_eq!(world.queues["coaster"].occupants, vec!["a".to_string()]);
            assert_eq!(
                world.visitors[0].queue_attraction,
                Some("coaster".to_string())
            );
            assert!(world.visitors[0].path.is_empty());
        }

        #[test]
        fn test_refuses_when_the_queue_is_already_full() {
            let mut world = GameWorld::new();
            world.queues.insert(
                "coaster".to_string(),
                crate::queue::QueueState {
                    chain: vec![(0, 0, 0)], // capacity 1 at any diameter
                    occupants: vec!["existing".to_string()].into(),
                },
            );
            world.visitors.push(visitor_at("a", (0.0, 0.0, 0.0)));

            let joined = world.try_join_queue("a", "coaster");

            assert!(!joined);
            assert_eq!(world.queues["coaster"].occupants.len(), 1);
            assert_eq!(world.visitors[0].queue_attraction, None);
        }
    }

    mod tick_queue_advancement {
        use super::*;

        #[test]
        fn test_a_queued_visitor_advances_toward_its_slot_instead_of_following_a_path() {
            let mut world = GameWorld::new();
            world
                .park_map
                .set_infrastructure(0, 0, 0, InfrastructureShape::Path);
            world
                .park_map
                .set_infrastructure(1, 0, 0, InfrastructureShape::Path);
            world.queues.insert(
                "coaster".to_string(),
                crate::queue::QueueState {
                    chain: vec![(0, 0, 0), (1, 0, 0)],
                    occupants: vec!["a".to_string()].into(),
                },
            );
            world.visitors.push(Visitor {
                id: "a".into(),
                position: (1.0, 0.0, 0.0),
                path: vec![(5, 5, 0)], // stale — must be ignored while queued
                target: (9, 9, 0),     // stale — must stay untouched while queued
                queue_attraction: Some("coaster".to_string()),
                ..Default::default()
            });

            world.tick(1.0); // large dt: should reach the front-of-queue slot (index 0)

            let visitor = &world.visitors[0];
            assert_eq!(visitor.position, (0.0, 0.0, 0.0));
            assert_eq!(visitor.target, (9, 9, 0)); // never went through assign_new_target_if_arrived
        }
    }
}
