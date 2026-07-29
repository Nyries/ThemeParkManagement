use serde::Deserialize;

use crate::map::ParkMap;

#[derive(Debug)]
pub enum MapLoadError {
    InvalidJson(String),
    OutOfBounds { x: i32, y: i32, z: i32},
    UnknownInfrastructureKind(String),
    UnknownMaterial(String),
    InvalidEntrance,
}

#[derive(Deserialize)] 
pub struct Dimensions {
    pub width: i32,
    pub height: i32,
    pub levels: Vec<i32>,
}

#[derive(Deserialize)] 
pub struct TerrainEntry {
    pub x: i32, pub y: i32, pub z: i32,
    pub material: String,
} 

#[derive(Deserialize)] 
pub struct InfrastructureEntry {
    pub x: i32, pub y: i32, pub z: i32,
    pub kind: String,
    pub to_z: Option<i32>,
}

#[derive(Deserialize)] 
pub struct BuildingEntry {
    pub x: i32, pub y: i32, pub z: i32,
    pub building_id: String,
    pub template_id: String,
}

#[derive(Deserialize)] 
pub struct Coord { pub x: i32, pub y: i32, pub z: i32 }

#[derive(Deserialize)] 
pub struct MapTemplate {
    pub archetype: String,
    pub name: String,
    pub dimensions: Dimensions,
    pub default_terrain: String,
    pub terrain : Vec<TerrainEntry>,
    pub infrastructure: Vec<InfrastructureEntry>,
    pub buildings: Vec<BuildingEntry>,
    pub entrance: Coord,
}

impl MapTemplate {
    pub fn load(path: &std::path::Path) -> Result<MapTemplate, MapLoadError> {
        let raw = std::fs::read_to_string(path)
            .map_err(|e| MapLoadError::InvalidJson(e.to_string()))?;
        serde_json::from_str(&raw) 
            .map_err(|e| MapLoadError::InvalidJson(e.to_string()))
    }


}