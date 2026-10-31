use std::collections::{HashMap, HashSet};

use crate::{
    balance::{SPAWN_INTERVAL_TICKS, VISITOR_RADIUS},
    building_template::{BuildingCatalog, CatalogSource},
    map::{Bounds3d, ParkMap},
    visitor::{Visitor, VisitorId, movement::pick_random_walking_destination},
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
        }
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

            let speed = 1.5;
            v.advance(speed, dt);

            let new_cell = cell_of(v.position);
            if new_cell == v.destination {
                v.path.clear();
                v.assign_new_destination(&self.park_map, new_cell);
            }

        }

        self.visitors.retain(|visitor| !visitor.has_expired());

        self.metrics.visitors_in_park = self.visitors.len();
        self.tick_count += 1;

        if self.visitors.len() < 1
            && self
                .tick_count
                .is_multiple_of(self.park_map.visitor_spawning_rate as u64)
        {
            self.spawn_visitor();
        }
    }

    pub fn spawn_visitor(&mut self) {
        let Some(entrance) = self.park_map.entrance else {
            return;
        };

        let destination = pick_random_walking_destination(&self.park_map).unwrap_or(entrance);
        let path = self.park_map.path_excluding_start(entrance, destination);

        let id = uuid::Uuid::new_v4().to_string();

        let mut visitor = Visitor::new(
            id.clone(),
            (entrance.0 as f32, entrance.1 as f32, entrance.2 as f32),
        );

        visitor.path = path;
        visitor.destination = destination;

        self.visitors.push(visitor);
    }

    pub fn despawn_visitors() {
        
    }

    pub fn reset_visitors(&mut self) {
        self.visitors.clear();
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

fn add_vec(a: (f32, f32, f32), b: (f32, f32, f32)) -> (f32, f32, f32) {
    (a.0 + b.0, a.1 + b.1, a.2 + b.2)
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
    }

    mod tick {

        use super::*;

        #[test]
        fn test_tick_clears_dirty_chunks_when_nothing_moves() {
            let mut world = GameWorld::new();
            // pas de visiteurs, rien ne bouge

            world.tick(1.0);

            assert!(world.dirty_chunks.is_empty());
        }

        #[test]
        fn test_tick_spawns_a_visitor_every_spawn_interval() {
            let mut world = GameWorld::new();
            let spawn_interval = 10;
            world.park_map.visitor_spawning_rate = 10;
            world.park_map.entrance = Some((0, 0, 0));
            world
                .park_map
                .set_infrastructure(0, 0, 0, InfrastructureShape::Path);

            for _ in 0..spawn_interval - 1 {
                world.tick(0.01);
            }
            assert!(
                world.visitors.is_empty(),
                "no spawn before the interval is reached"
            );

            world.tick(0.01); // atteint exactement SPAWN_INTERVAL_TICKS
            assert_eq!(world.visitors.len(), 1);
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
        fn test_reset_visitors_clears_visitors() {
            let mut world = GameWorld::new();
            world
                .park_map
                .set_infrastructure(0, 0, 0, InfrastructureShape::Path);
            world.visitors.push(Visitor {
                id: "a".into(),
                position: (0.0, 0.0, 0.0),
                path: vec![],
                destination: (0, 0, 0),
                ticks_since_spawn: 0,
                heading: (0.0, 0.0, 0.0),
                is_leaving: false,
                ..Default::default()
            });
            world.density.insert((0, 0, 0), vec!["a".into()]);

            world.reset_visitors();

            assert!(world.visitors.is_empty());
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
                destination: (0, 0, 0),
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
}
