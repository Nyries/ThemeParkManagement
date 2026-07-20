mod map;

use map::{ParkMap, TileType};
use std::time::{Duration, Instant};
use tokio::time::sleep;

struct GameWorld {
    park_map: ParkMap,
    tick_count: u64,
}

impl GameWorld {
    fn new() -> Self {
        Self { 
            park_map:ParkMap::new(), 
            tick_count: 0, 
        }
    }

    fn update(&mut self) {
        self.tick_count += 1;
    }

    fn tick(&mut self) {
        self.tick_count += 1;
    }
}
#[tokio::main]
async fn main() {
    println!("Démarrage du moteur de simulation Rust");

    let mut world = GameWorld::new();

    world.park_map.set_tile(0, 0, TileType::Path, 0);
    world.park_map.set_tile(1, 0, TileType::Empty, 0);

    let tick_interval = Duration::from_millis(50);

    loop {
        let start = Instant::now();

        world.update();

        if world.tick_count % 100 == 0 {
            println!("Tick de simulation en cours... Actuel: {}", world.tick_count);
        }

        let elapsed = start.elapsed();
        if elapsed < tick_interval {
            sleep(tick_interval-elapsed).await;
        }
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
