use std::collections::HashMap;
use serde::{Serialize, Deserialize};

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub enum TileType {
    Empty,
    Grass,
    Water,
    Montain,
    Path,
    Building(String),
    // TODO: Add new types of tiles
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Tile {
    pub tile_type: TileType,
    pub elevation: i32
}

#[derive(Serialize, Deserialize, Debug, Default)]
pub struct ParkMap {
    pub tiles: HashMap<(i32, i32), Tile>
}

impl ParkMap {
    pub fn new() -> Self {
        Self {
            tiles: HashMap::new(),
        }
    }

    pub fn set_tile(&mut self, x: i32, y: i32, tile_type: TileType, elevation: i32) {
        self.tiles.insert((x,y), Tile {tile_type, elevation});
    }

    pub fn get_tile(&self, x: i32, y: i32) -> Option<&Tile> {
        self.tiles.get(&(x,y))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_set_and_get_tile() {
        let mut park_map = ParkMap::new();

        assert!(park_map.get_tile(10, 20).is_none());

        park_map.set_tile(10, 20, TileType::Path, 1);

        let tile = park_map.get_tile(10, 20).expect("The tile should exist");
        assert_eq!(tile.tile_type, TileType::Path);
        assert_eq!(tile.elevation, 1);
    }

    #[test]
    fn test_overwrite_tile() {
        let mut park_map = ParkMap::new();
        
        park_map.set_tile(0, 0, TileType::Water, 0);
        park_map.set_tile(0, 0, TileType::Path, 0); 

        let tile = park_map.get_tile(0, 0).unwrap();
        assert_eq!(tile.tile_type, TileType::Path, "The tile should be overwritten by a path");
    }
}