use crate::map::ParkMap;

pub struct GameWorld {
    pub park_map: ParkMap,
    pub tick_count: u64,
}

impl GameWorld {
    pub fn new() -> Self {
        Self { 
            park_map:ParkMap::new(), 
            tick_count: 0, 
        }
    }

    pub fn update(&mut self) {
        self.tick_count += 1;
    }

    pub fn tick(&mut self) {
        self.tick_count += 1;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_game_world_initialization() {
        let world = GameWorld::new();
        assert_eq!(world.tick_count, 0);
        assert!(world.park_map.tiles.is_empty());
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
}