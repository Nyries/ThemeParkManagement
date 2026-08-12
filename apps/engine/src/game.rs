use std::{collections::{HashMap, HashSet}};

use crate::{balance::{DENSITY_CAP, SPAWN_INTERVAL_TICKS}, building_template::{BuildingCatalog, CatalogSource}, map::{Bounds3d, ParkMap, base_speed_for}, visitor::{Visitor, VisitorId, repulsion_force, speed_at}};



#[derive(Debug, Default)]
pub struct ParkMetricsAccumulator {
    pub visitors_in_park: usize,
    pub visitors_exited: u64,
}

pub struct GameWorld {
    pub park_map: ParkMap,
    pub building_catalog: BuildingCatalog,
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
            park_map:ParkMap::new(
                "default".into(), //To replace with a parkmap preloaded
                Bounds3d::new(0, 50, 0, 30, -1, 1)
            ), 
            building_catalog: BuildingCatalog::load(
                CatalogSource::Embedded(include_str!("../assets/catalog/buildings.json"))
            ).expect("embedded buildings.json should always be valid"),
            tick_count: 0, 
            visitors: vec![],
            density: HashMap::new(),
            dirty_chunks: HashSet::new(),
            metrics: ParkMetricsAccumulator::default(),
            paused: false,
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
        let positions: HashMap<VisitorId, (f32, f32, f32)> = self.visitors.iter().map(|v| (v.id.clone(), v.position)).collect();
        let exit = self.park_map.entrance;

        for v in self.visitors.iter_mut() {
            v.ticks_since_spawn += 1; 
            let old_cell = cell_of(v.position);

            redirect_if_expired(v, &self.park_map, old_cell, exit);
            assign_new_target_if_arrived(v, &self.park_map, old_cell);
            recompute_path_if_blocked(v, &self.park_map, old_cell);

            let speed = compute_speed(&self.park_map, &self.density, old_cell);
            let repulsion = compute_repulsion(v, &self.density, &positions, old_cell);
            v.advance(speed, dt, repulsion); 

            let new_cell = cell_of(v.position);
            update_density_and_dirty_chunks(&mut self.density, &mut self.dirty_chunks, &v.id, old_cell, new_cell);
        }
        self.despawn_visitors_who_reached_exit();

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
        let path = self.park_map.path_excluding_start(entrance, target);

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

    pub fn reset_visitors(&mut self) {
        self.visitors.clear();
        self.density.clear();
    }
}

fn cell_of(position: (f32, f32, f32)) -> (i32, i32, i32) {
    (position.0.round() as i32, position.1.round() as i32, position.2.round() as i32)
}

fn redirect_if_expired(v: &mut Visitor, park_map: &ParkMap, old_cell: (i32, i32, i32), exit: Option<(i32, i32, i32)>) {
    let Some(exit) = exit else { return };
    if v.has_expired() && !v.is_leaving {
        v.is_leaving = true;
        v.target = exit;
        v.path = park_map.path_excluding_start(old_cell, exit);
    }
}

fn assign_new_target_if_arrived(v: &mut Visitor, park_map: &ParkMap, old_cell: (i32, i32, i32)) {
    if v.path.is_empty() && !v.is_leaving {
        let new_target = park_map.random_walkable_cell(old_cell).unwrap_or(old_cell);
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

fn compute_speed(park_map: &ParkMap, density: &HashMap<(i32, i32, i32), Vec<VisitorId>>, cell: (i32, i32, i32)) -> f32 {
    let base_speed = park_map
        .get_infrastructure(cell.0, cell.1, cell.2)
        .map(base_speed_for)
        .unwrap_or(0.0);
    let local_density = density.get(&cell).map(|bucket| bucket.len()).unwrap_or(0);
    speed_at(base_speed, local_density)
}

fn compute_repulsion(
    v: &Visitor,
    density: &HashMap<(i32, i32, i32), Vec<VisitorId>>,
    positions: &HashMap<VisitorId, (f32, f32, f32)>,
    cell: (i32, i32, i32)
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
                        let force =repulsion_force(v.position, other_pos);
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

fn update_density_and_dirty_chunks(
    density: &mut HashMap<(i32, i32, i32), Vec<VisitorId>>,
    dirty_chunks: &mut HashSet<(i32, i32)>,
    visitor_id: &VisitorId,
    old_cell: (i32, i32, i32),
    new_cell: (i32, i32, i32)
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
    density.entry(new_cell).or_default().push(visitor_id.clone());
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

    mod redirect_if_expired {
        use crate::balance::VISIT_DURATION_TICKS;
        use super::*;

        fn expired_visitor_at(position: (i32, i32, i32)) -> Visitor {
            Visitor {
                id: "a".into(),
                position: (position.0 as f32, position.1 as f32, position.2 as f32),
                path: vec![],
                target: position,
                ticks_since_spawn: VISIT_DURATION_TICKS,
                heading: (0.0, 0.0, 0.0),
                is_leaving: false,
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
            }
        }

        #[test]
        fn test_does_nothing_when_path_is_not_empty() {
            let park_map = ParkMap::new("m".into(), Bounds3d::new(0, 5, 0, 5, -1, 1));
            let mut v = arrived_visitor();
            v.path = vec![(2, 2, 0)];

            assign_new_target_if_arrived(&mut v, &park_map, (0, 0, 0));

            assert_eq!(v.path, vec![(2, 2, 0)]);
        }

        #[test]
        fn test_does_nothing_when_visitor_is_leaving() {
            let park_map = ParkMap::new("m".into(), Bounds3d::new(0, 5, 0, 5, -1, 1));
            let mut v = arrived_visitor();
            v.is_leaving = true;

            assign_new_target_if_arrived(&mut v, &park_map, (0, 0, 0));

            assert!(v.path.is_empty());
            assert_eq!(v.target, (0, 0, 0));
        }

        #[test]
        fn test_assigns_new_target_and_path_when_arrived() {
            let mut park_map = ParkMap::new("m".into(), Bounds3d::new(0, 5, 0, 5, -1, 1));
            park_map.set_infrastructure(0, 0, 0, InfrastructureShape::Path);
            park_map.set_infrastructure(1, 0, 0, InfrastructureShape::Path);
            let mut v = arrived_visitor();

            assign_new_target_if_arrived(&mut v, &park_map, (0, 0, 0));

            assert_eq!(v.target, (1, 0, 0)); // only other candidate, deterministic
            assert_eq!(v.path, vec![(1, 0, 0)]);
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
            let repulsion_over_cap = compute_repulsion(&v, &density_over_cap, &positions, (0, 0, 0));

            assert_eq!(
                repulsion_at_cap, repulsion_over_cap,
                "neighbors beyond DENSITY_CAP in the same bucket should not affect the result"
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

            update_density_and_dirty_chunks(&mut density, &mut dirty_chunks, &"a".to_string(), (0, 0, 0), (0, 0, 0));

            assert_eq!(density.get(&(0, 0, 0)), Some(&vec!["a".to_string()]));
            assert!(dirty_chunks.is_empty());
        }

        #[test]
        fn test_moves_visitor_to_new_bucket_when_cell_changes() {
            let mut density: HashMap<(i32, i32, i32), Vec<VisitorId>> = HashMap::new();
            density.insert((0, 0, 0), vec!["a".into()]);
            let mut dirty_chunks = HashSet::new();

            update_density_and_dirty_chunks(&mut density, &mut dirty_chunks, &"a".to_string(), (0, 0, 0), (1, 0, 0));

            assert_eq!(density.get(&(1, 0, 0)), Some(&vec!["a".to_string()]));
        }

        #[test]
        fn test_removes_old_bucket_when_it_becomes_empty() {
            let mut density: HashMap<(i32, i32, i32), Vec<VisitorId>> = HashMap::new();
            density.insert((0, 0, 0), vec!["a".into()]);
            let mut dirty_chunks = HashSet::new();

            update_density_and_dirty_chunks(&mut density, &mut dirty_chunks, &"a".to_string(), (0, 0, 0), (1, 0, 0));

            assert!(!density.contains_key(&(0, 0, 0)));
        }

        #[test]
        fn test_keeps_old_bucket_when_other_visitors_remain() {
            let mut density: HashMap<(i32, i32, i32), Vec<VisitorId>> = HashMap::new();
            density.insert((0, 0, 0), vec!["a".into(), "b".into()]);
            let mut dirty_chunks = HashSet::new();

            update_density_and_dirty_chunks(&mut density, &mut dirty_chunks, &"a".to_string(), (0, 0, 0), (1, 0, 0));

            assert_eq!(density.get(&(0, 0, 0)), Some(&vec!["b".to_string()]));
        }

        #[test]
        fn test_marks_new_chunk_as_dirty_when_cell_changes() {
            let mut density: HashMap<(i32, i32, i32), Vec<VisitorId>> = HashMap::new();
            density.insert((0, 0, 0), vec!["a".into()]);
            let mut dirty_chunks = HashSet::new();

            update_density_and_dirty_chunks(&mut density, &mut dirty_chunks, &"a".to_string(), (0, 0, 0), (1, 0, 0));

            assert!(dirty_chunks.contains(&(1, 0)));
        }
    }
        mod pause_resume {
        use super::*;

        #[test]
        fn test_tick_does_nothing_when_paused() {
            let mut world = GameWorld::new();
            world.park_map.entrance = Some((0, 0, 0));
            world.park_map.set_infrastructure(0, 0, 0, InfrastructureShape::Path);
            world.park_map.set_infrastructure(1, 0, 0, InfrastructureShape::Path);
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
            world.park_map.set_infrastructure(0, 0, 0, InfrastructureShape::Path);
            world.park_map.set_infrastructure(1, 0, 0, InfrastructureShape::Path);
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
            world.park_map.set_infrastructure(0, 0, 0, InfrastructureShape::Path);
            world.visitors.push(Visitor {
                id: "a".into(),
                position: (0.0, 0.0, 0.0),
                path: vec![],
                target: (0, 0, 0),
                ticks_since_spawn: 0,
                heading: (0.0, 0.0, 0.0),
                is_leaving: false,
            });
            world.density.insert((0, 0, 0), vec!["a".into()]);

            world.reset_visitors();

            assert!(world.visitors.is_empty());
            assert!(world.density.is_empty());
        }

        #[test]
        fn test_reset_visitors_does_not_touch_map_or_tick_count() {
            let mut world = GameWorld::new();
            world.park_map.set_infrastructure(0, 0, 0, InfrastructureShape::Path);
            world.tick_count = 42;
            world.visitors.push(Visitor {
                id: "a".into(),
                position: (0.0, 0.0, 0.0),
                path: vec![],
                target: (0, 0, 0),
                ticks_since_spawn: 0,
                heading: (0.0, 0.0, 0.0),
                is_leaving: false,
            });

            world.reset_visitors();

            assert_eq!(world.tick_count, 42);
            assert!(world.park_map.get_infrastructure(0, 0, 0).is_some());
        }
    }

}