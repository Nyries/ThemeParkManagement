use std::collections::HashMap;

use crate::{map::{Bounds3d, ParkMap}, visitor::{Visitor, VisitorId}};

pub struct GameWorld {
    pub park_map: ParkMap,
    pub tick_count: u64,
    pub visitors: Vec<Visitor>,
    pub density: HashMap<(i32, i32, i32), Vec<VisitorId>>
}

impl Default for GameWorld {
    fn default() -> Self {
        Self::new()
    }
}

impl GameWorld {
    pub fn new() -> Self {
        Self { 
            park_map:ParkMap::new(
                "default".into(), //To replace with a parkmap preloaded
                Bounds3d::new(0, 50, 0, 30, -1, 1)
            ), 
            tick_count: 0, 
            visitors: vec![],
            density: HashMap::new()
        }
    }

    pub fn update(&mut self) {
        self.tick_count += 1;
    }

    pub fn tick(&mut self) {
        self.tick_count += 1;
    }

    pub fn spawn_visitor(&mut self) {
        let Some((ex, ey, ez)) = self.park_map.entrance else {
            return;
        };

        let id = uuid::Uuid::new_v4().to_string();

        self.visitors.push(Visitor { 
            id: id.clone(), 
            position: (ex as f32, ey as f32, ez as f32), 
            path: vec![], 
            target: (ex, ey, ez) 
        });

        self.density
            .entry((ex, ey, ez))
            .or_insert_with(Vec::new)
            .push(id);
    } 
}

#[cfg(test)]
mod tests {
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
        world.tick();
        assert_eq!(world.tick_count, 1);
        
        world.tick();
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
    fn test_spawn_visitor_adds_visitor_at_entrance() {
        let mut world = GameWorld::new();
        world.park_map.entrance = Some((5, 3, 0));

        world.spawn_visitor();

        assert_eq!(world.visitors.len(), 1);
        let visitor = &world.visitors[0];
        assert_eq!(visitor.position, (5.0, 3.0, 0.0));
        assert_eq!(visitor.target, (5, 3, 0));
        assert!(visitor.path.is_empty());
    }

    #[test]
    fn test_spawn_visitor_updates_density_at_entrance() {
        let mut world = GameWorld::new();
        world.park_map.entrance = Some((5, 3, 0));

        world.spawn_visitor();

        let visitor_id = world.visitors[0].id.clone();
        let bucket = world.density.get(&(5, 3, 0)).expect("density bucket should exist");
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