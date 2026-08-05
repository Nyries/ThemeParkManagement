use std::{collections::{HashMap, HashSet}};

use crate::{balance::SPAWN_INTERVAL_TICKS, map::{Bounds3d, ParkMap, base_speed_for}, visitor::{Visitor, VisitorId, repulsion_force, speed_at}};

#[derive(Debug, Default)]
pub struct ParkMetricsAccumulator {
    pub visitors_in_park: usize,
    pub visitors_exited: u64,
}

pub struct GameWorld {
    pub park_map: ParkMap,
    pub tick_count: u64,
    pub visitors: Vec<Visitor>,
    pub density: HashMap<(i32, i32, i32), Vec<VisitorId>>,
    pub dirty_chunks: HashSet<(i32, i32)>,
    pub metrics: ParkMetricsAccumulator
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
            density: HashMap::new(),
            dirty_chunks: HashSet::new(),
            metrics: ParkMetricsAccumulator::default(),
        }
    }

    pub fn tick(&mut self, dt: f32) {
        // Real game loop of the core game
        self.dirty_chunks.clear();
        let positions: HashMap<VisitorId, (f32, f32, f32)> = self.visitors.iter().map(|v| (v.id.clone(), v.position)).collect();
        let exit = self.park_map.entrance;

        for v in self.visitors.iter_mut() {
            v.ticks_since_spawn += 1; 

            let old_cell = (
                v.position.0.round() as i32,
                v.position.1.round() as i32,
                v.position.2.round() as i32,
            );

            if let Some(exit) = exit
                && v.has_expired() && !v.is_leaving {
                    v.is_leaving = true;
                    v.target = exit;
                    let mut new_path = self.park_map
                        .find_path(old_cell, exit)
                        .map(|(p, _)| p)
                        .unwrap_or_default();
                    if !new_path.is_empty() {
                        new_path.remove(0);
                    }
                    v.path = new_path;
            }

            if v.path.is_empty() && !v.is_leaving {
                let new_target = self.park_map.random_walkable_cell(old_cell).unwrap_or(old_cell);
                v.target = new_target;
                let mut new_path = self.park_map
                    .find_path(old_cell, new_target)
                    .map(|(p, _)| p)
                    .unwrap_or_default();
                if !new_path.is_empty() {
                    new_path.remove(0);
                }
                v.path = new_path;
            }

            if let Some(&next) = v.path.first() 
                && !self.park_map.is_walkable(next.0, next.1, next.2) {
                    let mut new_path = self.park_map
                        .find_path(old_cell, v.target)
                        .map(|(path, _cost)| path)
                        .unwrap_or_default();
                    if !new_path.is_empty() {
                        new_path.remove(0);
                    }
                    v.path = new_path;
                
            }
            let base_speed = self.park_map
                .get_infrastructure(old_cell.0, old_cell.1, old_cell.2)
                .map(base_speed_for)
                .unwrap_or(0.0);
            let density = self.density.get(&old_cell).map(|bucket| bucket.len()).unwrap_or(0);
            let speed = speed_at(base_speed, density);

            let mut repulsion = (0.0, 0.0, 0.0);
            for dx in -1..=1 {
                for dy in -1..=1 {
                    let neighbor_cell = (old_cell.0 + dx, old_cell.1 + dy, old_cell.2);
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

            let new_cell = (
                v.position.0.round() as i32,
                v.position.1.round() as i32,
                v.position.2.round() as i32,
            );
            if new_cell != old_cell {
                if let Some(bucket) = self.density.get_mut(&old_cell) {
                    bucket.retain(|id| id != &v.id);
                    if bucket.is_empty() {
                        self.density.remove(&old_cell);
                    }
                }
                self.density.entry(new_cell).or_default().push(v.id.clone());
                self.dirty_chunks.insert((new_cell.0, new_cell.1));
            }
        }

        // Despawn
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
        self.metrics.visitors_in_park = self.visitors.len();

        self.tick_count += 1;

        if self.tick_count.is_multiple_of(SPAWN_INTERVAL_TICKS) {
            self.spawn_visitor();
        }
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
            is_leaving: false
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
    fn test_game_world_starts_with_empty_metrics() {
        let world = GameWorld::new();
        assert_eq!(world.metrics.visitors_in_park, 0);
        assert_eq!(world.metrics.visitors_exited, 0);
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

    mod tick {
        use crate::balance::VISIT_DURATION_TICKS;

use super::*;

        #[test]
        fn test_tick_moves_visitor_toward_target() {
            let mut world = GameWorld::new();
            world.park_map.entrance = Some((0, 0, 0));
            world.park_map.set_infrastructure(0, 0, 0, InfrastructureShape::Path);
            world.park_map.set_infrastructure(1, 0, 0, InfrastructureShape::Path);

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
            world.park_map.set_infrastructure(0, 0, 0, InfrastructureShape::Path);
            world.park_map.set_infrastructure(1, 0, 0, InfrastructureShape::Path);

            world.spawn_visitor();

            world.tick(2.0); // dt large : garantit l'arrivée exacte sur (1,0,0) en un seul tick

            let visitor_id = world.visitors[0].id.clone();
            assert_eq!(world.visitors[0].position, (1.0, 0.0, 0.0));
            assert!(!world.density.contains_key(&(0, 0, 0)), "old cell bucket should be removed once empty");
            assert_eq!(world.density.get(&(1, 0, 0)), Some(&vec![visitor_id]));
        }

        #[test]
        fn test_tick_speed_decreases_with_density_on_current_cell() {
            let mut lone_world = GameWorld::new();
            lone_world.park_map.set_infrastructure(0, 0, 0, InfrastructureShape::Path);
            lone_world.park_map.set_infrastructure(5, 0, 0, InfrastructureShape::Path);
            lone_world.visitors.push(Visitor {
                id: "lone".into(),
                position: (0.0, 0.0, 0.0),
                path: vec![(5, 0, 0)],
                target: (5, 0, 0),
                ticks_since_spawn: 0,
                heading: (0.0, 0.0, 0.0),
                is_leaving: false,
            });
            lone_world.density.insert((0, 0, 0), vec!["lone".into()]);

            let mut crowded_world = GameWorld::new();
            crowded_world.park_map.set_infrastructure(0, 0, 0, InfrastructureShape::Path);
            crowded_world.visitors.push(Visitor {
                id: "v0".into(),
                position: (0.0, 0.0, 0.0),
                path: vec![(5, 0, 0)],
                target: (5, 0, 0),
                ticks_since_spawn: 0,
                heading: (0.0, 0.0, 0.0),
                is_leaving: false,
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
            world.park_map.set_infrastructure(0, 0, 0, InfrastructureShape::Path);
            world.park_map.set_infrastructure(5, 0, 0, InfrastructureShape::Path);

            world.visitors.push(Visitor {
                id: "a".into(),
                position: (0.0, 0.0, 0.0),
                path: vec![(5, 0, 0)],
                target: (5, 0, 0),
                ticks_since_spawn: 0,
                heading: (0.0, 0.0, 0.0),
                is_leaving: false,
            });
            world.visitors.push(Visitor {
                id: "b".into(),
                position: (0.0, 0.15, 0.0), // within AVOIDING_RADIUS of "a"
                path: vec![], // stays put, isolates "a"'s reaction to the repulsion
                target: (0, 0, 0),
                ticks_since_spawn: 0,
                heading: (0.0, 0.0, 0.0),
                is_leaving: false,
            });
            world.density.insert((0, 0, 0), vec!["a".into(), "b".into()]);

            world.tick(0.05);

            let a = world.visitors.iter().find(|v| v.id == "a").unwrap();
            assert!(a.position.1 < 0.0, "a should be pushed away from b (at +y), got y = {}", a.position.1);
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
            });

            world.tick(1.0);

            assert_eq!(world.visitors[0].position, (0.0, 0.0, 0.0));
        }

        #[test]
        fn test_tick_marks_crossed_chunk_as_dirty() {
            let mut world = GameWorld::new();
            world.park_map.entrance = Some((0, 0, 0));
            world.park_map.set_infrastructure(0, 0, 0, InfrastructureShape::Path);
            world.park_map.set_infrastructure(1, 0, 0, InfrastructureShape::Path);

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
            world.park_map.set_infrastructure(0, 0, 0, InfrastructureShape::Path);
            world.park_map.set_infrastructure(1, 0, 0, InfrastructureShape::Path);
            world.park_map.set_infrastructure(0, 1, 0, InfrastructureShape::Path);
            world.park_map.set_infrastructure(1, 1, 0, InfrastructureShape::Path);

            world.visitors.push(Visitor {
                id: "a".into(),
                position: (0.0, 0.0, 0.0),
                path: vec![(1, 0, 0)], // stale : cette case va être bloquée juste après
                target: (1, 1, 0),
                ticks_since_spawn: 0,
                heading: (0.0, 0.0, 0.0),
                is_leaving: false,
            });

            world.park_map.remove_infrasture(1, 0, 0); // simule une modification de carte

            world.tick(0.01); // dt petit : on veut juste voir le chemin recalculé, pas l'arrivée

            let visitor = &world.visitors[0];
            assert_ne!(visitor.path.first(), Some(&(1, 0, 0)), "should not still point at the blocked cell");
            assert!(!visitor.path.is_empty(), "an alternate route exists via (0,1,0)");
        }

        #[test]
        fn test_tick_clears_path_when_target_becomes_unreachable_after_recalculation() {
            let mut world = GameWorld::new();
            world.park_map.set_infrastructure(0, 0, 0, InfrastructureShape::Path);
            world.park_map.set_infrastructure(1, 0, 0, InfrastructureShape::Path);

            world.visitors.push(Visitor {
                id: "a".into(),
                position: (0.0, 0.0, 0.0),
                path: vec![(1, 0, 0)],
                target: (1, 0, 0), // la cible elle-même va devenir impraticable, aucune autre route
                ticks_since_spawn: 0,
                heading: (0.0, 0.0, 0.0),
                is_leaving: false,
            });

            world.park_map.remove_infrasture(1, 0, 0);

            world.tick(0.01);

            assert!(world.visitors[0].path.is_empty());
        }

        #[test]
        fn test_tick_syncs_visitors_in_park_metric() {
            let mut world = GameWorld::new();
            world.park_map.entrance = Some((0, 0, 0));
            world.park_map.set_infrastructure(0, 0, 0, InfrastructureShape::Path);

            world.spawn_visitor();
            world.spawn_visitor();

            world.tick(0.05);

            assert_eq!(world.metrics.visitors_in_park, 2);
        }

        #[test]
        fn test_tick_redirects_expired_visitor_toward_exit() {
            let mut world = GameWorld::new();
            world.park_map.entrance = Some((0, 0, 0));
            world.park_map.set_infrastructure(0, 0, 0, InfrastructureShape::Path);
            world.park_map.set_infrastructure(1, 0, 0, InfrastructureShape::Path);

            world.visitors.push(Visitor {
                id: "a".into(),
                position: (1.0, 0.0, 0.0),
                path: vec![],
                target: (1, 0, 0),
                ticks_since_spawn: VISIT_DURATION_TICKS,
                heading: (0.0, 0.0, 0.0),
                is_leaving: false,
            });

            world.tick(0.01);

            let visitor = &world.visitors[0];
            assert!(visitor.is_leaving);
            assert_eq!(visitor.target, (0, 0, 0));
        }

        #[test]
        fn test_tick_removes_visitor_who_reached_the_exit() {
            let mut world = GameWorld::new();
            world.park_map.set_infrastructure(0, 0, 0, InfrastructureShape::Path);

            world.visitors.push(Visitor {
                id: "a".into(),
                position: (0.0, 0.0, 0.0),
                path: vec![], // déjà arrivé
                target: (0, 0, 0),
                ticks_since_spawn: 0,
                heading: (0.0, 0.0, 0.0),
                is_leaving: true, // était déjà en train de partir
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
            world.park_map.set_infrastructure(0, 0, 0, InfrastructureShape::Path);

            for _ in 0..SPAWN_INTERVAL_TICKS - 1 {
                world.tick(0.01);
            }
            assert!(world.visitors.is_empty(), "no spawn before the interval is reached");

            world.tick(0.01); // atteint exactement SPAWN_INTERVAL_TICKS
            assert_eq!(world.visitors.len(), 1);
        }

        #[test]
        fn test_tick_assigns_a_new_target_when_visitor_arrives_and_is_not_leaving() {
            let mut world = GameWorld::new();
            world.park_map.set_infrastructure(0, 0, 0, InfrastructureShape::Path);
            world.park_map.set_infrastructure(1, 0, 0, InfrastructureShape::Path);

            world.visitors.push(Visitor {
                id: "a".into(),
                position: (0.0, 0.0, 0.0),
                path: vec![], // déjà arrivé
                target: (0, 0, 0),
                ticks_since_spawn: 0,
                heading: (0.0, 0.0, 0.0),
                is_leaving: false,
            });

            world.tick(0.01);

            let visitor = &world.visitors[0];
            assert_eq!(visitor.target, (1, 0, 0)); // seul autre candidat, déterministe
            assert_eq!(visitor.path, vec![(1, 0, 0)]);
        }

        #[test]
        fn test_tick_does_not_assign_new_target_when_visitor_is_leaving() {
            let mut world = GameWorld::new();
            world.park_map.set_infrastructure(0, 0, 0, InfrastructureShape::Path);
            world.park_map.set_infrastructure(1, 0, 0, InfrastructureShape::Path);

            world.visitors.push(Visitor {
                id: "a".into(),
                position: (0.0, 0.0, 0.0),
                path: vec![],
                target: (0, 0, 0),
                ticks_since_spawn: 0,
                heading: (0.0, 0.0, 0.0),
                is_leaving: true,
            });
            world.density.insert((0, 0, 0), vec!["a".into()]);

            world.tick(0.01);

            // Doit être despawné (path vide + is_leaving), pas recevoir une nouvelle cible.
            assert!(world.visitors.is_empty());
        }
    }
}