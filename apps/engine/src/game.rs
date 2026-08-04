use std::collections::HashMap;

use crate::{map::{Bounds3d, ParkMap, base_speed_for}, visitor::{Visitor, VisitorId, repulsion_force, speed_at}};

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

    pub fn tick(&mut self, dt: f32) {
        // Real game loop of the core game
        let positions: HashMap<VisitorId, (f32, f32, f32)> = self.visitors.iter().map(|v| (v.id.clone(), v.position)).collect();
        
        for v in self.visitors.iter_mut() {
            let cell = (
                v.position.0.round() as i32,
                v.position.1.round() as i32,
                v.position.2.round() as i32,
            );
            let base_speed = self.park_map
                .get_infrastructure(cell.0, cell.1, cell.2)
                .map(base_speed_for)
                .unwrap_or(0.0);
            let density = self.density.get(&cell).map(|bucket| bucket.len()).unwrap_or(0);
            let speed = speed_at(base_speed, density);

            let mut repulsion = (0.0, 0.0, 0.0);
            for dx in -1..=1 {
                for dy in -1..=1 {
                    let neighbor_cell = (cell.0 + dx, cell.1 + dy, cell.2);
                    if let Some(bucket) = self.density.get(&neighbor_cell) {
                        for other_id in bucket {
                            if *other_id == v.id {
                                continue;
                            }
                            if let Some(&other_pos) = positions.get(other_id) {
                                let force =repulsion_force(v.position, other_pos);
                                repulsion.0 += force.0;
                                repulsion.1 += force.1;
                                repulsion.2 += force.2;
                            }
                        }
                    }
                }
            }

            v.advance(speed, dt, repulsion); 
        }
        
        self.tick_count += 1;
    }

    pub fn spawn_visitor(&mut self) {
        let Some(entrance) = self.park_map.entrance else {
            return;
        };

        let target = self.park_map.random_walkable_cell(entrance).unwrap_or(entrance);
        let mut path = self.park_map
            .find_path(entrance, target)
            .map(|(path, _cost)| path)
            .unwrap_or_default();
        if !path.is_empty() {
            path.remove(0);
        }

        let id = uuid::Uuid::new_v4().to_string();

        self.visitors.push(Visitor { 
            id: id.clone(), 
            position: (entrance.0 as f32, entrance.1 as f32, entrance.2 as f32), 
            path,
            target,
            ticks_since_spawn: 0,
            heading: (0.0, 0.0, 0.0),
        });

        self.density
            .entry(entrance)
            .or_default()
            .push(id);
    } 
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::map::InfrastructureShape;

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
        world.park_map.set_infrastructure(0, 0, 0, InfrastructureShape::Path);
        world.park_map.set_infrastructure(1, 0, 0, InfrastructureShape::Path);

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