use serde::Deserialize;

use crate::map::{Bounds3d, BuildingId, InfrastructureShape, Parcel, ParkMap};

pub enum MapSource {
    Embedded(&'static str),
    File(std::path::PathBuf),
}

#[derive(Debug)]
pub enum MapLoadError {
    InvalidJson(String),
    OutOfBounds { x: i32, y: i32, z: i32},
    UnknownInfrastructureKind(String),
    UnknownMaterial(String),
    InvalidEntrance,
}

impl std::fmt::Display for MapLoadError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}", self)
    }
}

impl std::error::Error for MapLoadError {}

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
pub struct ParcelEntry {
    pub id: String,
    pub cells: Vec<(i32, i32)>,
    pub unlocked: bool,
    pub price: u32,
}

#[derive(Deserialize)] 
pub struct Coord { pub x: i32, pub y: i32, pub z: i32 }

fn parse_infrastructure_kind(kind: &str, to_z: Option<i32>) -> Result<InfrastructureShape, MapLoadError> {
    match kind {
        "path" => Ok(InfrastructureShape::Path),
        "ramp" => {
            let to_z = to_z.ok_or_else(|| {
                MapLoadError::InvalidJson("infrastructure ramp without to_z".into())
            })?;
            Ok(InfrastructureShape::Ramp { to_z })
        },
        "stairs" => {
            let to_z = to_z.ok_or_else(|| {
                MapLoadError::InvalidJson("infrastructure stairs without to_z".into())
            })?;
            Ok(InfrastructureShape::Stairs { to_z })
        },
        other => Err(MapLoadError::UnknownInfrastructureKind(other.to_string())),
    }
}

#[derive(Deserialize)] 
pub struct MapTemplate {
    pub archetype: String,
    pub name: String,
    pub dimensions: Dimensions,
    pub default_terrain: String,
    pub terrain : Vec<TerrainEntry>,
    pub infrastructure: Vec<InfrastructureEntry>,
    pub buildings: Vec<BuildingEntry>,
    pub parcels: Vec<ParcelEntry>,
    pub entrance: Coord,
}

impl MapTemplate {
    
    pub fn load(source: MapSource) -> Result<MapTemplate, MapLoadError> {
        let raw = match source {
            MapSource::Embedded(json) => json.to_string(),
            MapSource::File(path) => std::fs::read_to_string(&path)
                .map_err(|e| MapLoadError::InvalidJson(e.to_string()))?,
        };
        serde_json::from_str(&raw) 
            .map_err(|e| MapLoadError::InvalidJson(e.to_string()))
    }

    pub fn into_park_map(&self) -> Result<ParkMap, MapLoadError> {
        let min_z = *self.dimensions.levels.iter().min()
            .ok_or_else(|| MapLoadError::InvalidJson("dimensions.levels is empty".into()))?;
        let max_z = *self.dimensions.levels.iter().max().unwrap();
        let bounds = Bounds3d::new(
            0, self.dimensions.width - 1, 
            0, self.dimensions.height -1,
            min_z, max_z
        );

        let mut park_map = ParkMap::new(uuid::Uuid::new_v4().to_string(), bounds);
        park_map.unlocked_levels = self.dimensions.levels.iter().copied().collect();
        // Filling default_terrain
        for x in 0..self.dimensions.width {
            for y in 0..self.dimensions.height {
                    park_map.set_terrain(x, y, 0, self.default_terrain.clone());
            }
        }
        // Replacing the exceptions
        for terrain_entry in &self.terrain {
            if !park_map.is_within_bounds(terrain_entry.x, terrain_entry.y, terrain_entry.z) {
                return Err(MapLoadError::OutOfBounds { x: terrain_entry.x, y: terrain_entry.y, z: terrain_entry.z });
            }
            park_map.set_terrain(terrain_entry.x, terrain_entry.y, terrain_entry.z, terrain_entry.material.clone());
        }
        // Placing infrastructure
        for infrastructure_entry in &self.infrastructure {
            if !park_map.is_within_bounds(infrastructure_entry.x, infrastructure_entry.y, infrastructure_entry.z) {
                return Err(MapLoadError::OutOfBounds { x: infrastructure_entry.x, y: infrastructure_entry.y, z: infrastructure_entry.z });
            }
            let shape = parse_infrastructure_kind(&infrastructure_entry.kind, infrastructure_entry.to_z)?;
            park_map.set_infrastructure(infrastructure_entry.x, infrastructure_entry.y, infrastructure_entry.z, shape);
        }
        // Placing the buildings
        for building_entry in &self.buildings {
            if !park_map.is_within_bounds(building_entry.x, building_entry.y, building_entry.z) {
                return Err(MapLoadError::OutOfBounds { x: building_entry.x, y: building_entry.y, z: building_entry.z });
            }
            let building_id = BuildingId {
                building_id: building_entry.building_id.clone(),
                template_id: building_entry.template_id.clone()
            };
            park_map.set_building(building_entry.x, building_entry.y, building_entry.z, building_id);
        }
        // Splitting map into parcels
        for parcel_entry in &self.parcels {
            for &(x, y) in &parcel_entry.cells {
                if !park_map.is_within_bounds(x, y, park_map.bounds.min_z) {
                    return Err(MapLoadError::OutOfBounds { x, y, z: park_map.bounds.min_z });
                }
            }
            park_map.parcels.push(Parcel {
                id: parcel_entry.id.clone(),
                cells: parcel_entry.cells.clone(),
                unlocked: parcel_entry.unlocked,
                price: parcel_entry.price,
            });
        }
        // Checking if the entrance points to a walkable infrastructure
        if park_map.get_infrastructure(self.entrance.x, self.entrance.y, self.entrance.z).is_none() {
            return Err(MapLoadError::InvalidEntrance);
        }
        park_map.entrance = Some((self.entrance.x, self.entrance.y, self.entrance.z));
        Ok(park_map)
    }
}

#[cfg(test)]
mod tests;