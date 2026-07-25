use std::collections::{HashMap, HashSet};
use serde::{Serialize, Deserialize};

use crate::simulation::{ErrorCode, Rotation};

fn crossing_flags_for(template_id: &str) -> (bool, bool) {
    // Allows above or allows below
    match template_id {
        "bridge_support" => (true, true),
        _ => (false, false),
    } // Depends on template_id catalog
}

fn rotate_footprint(footprint: &[(i32, i32)], rotation: Rotation) -> Vec<(i32, i32)> {
    footprint.iter().map(|&(dx, dy)| match rotation {
        Rotation::Deg0 => (dx, dy),
        Rotation::Deg90 => (-dy, dx),
        Rotation::Deg180 => (-dx, -dy),
        Rotation::Deg270 => (dy, -dx),
    }).collect()
}

#[derive(Serialize, Deserialize, Debug, Clone, Default)]
pub struct Parcel {
    pub id: String,
    pub cells: Vec<(i32, i32)>,
    pub unlocked: bool,
    pub price: u32,
}

#[derive(Serialize, Deserialize, Debug, Clone, Default)]
pub struct Bounds3d {
    pub min_x: i32,
    pub max_x: i32,
    pub min_y: i32,
    pub max_y: i32,
    pub min_z: i32,
    pub max_z: i32,
}

impl Bounds3d {
    pub fn new(min_x: i32, max_x: i32, min_y: i32, max_y: i32, min_z: i32, max_z: i32) -> Self {
        Self {
            min_x,
            max_x,
            min_y,
            max_y,
            min_z,
            max_z,
        }
    }
}

#[derive(Serialize, Deserialize, Debug, Clone, Default)]
pub struct Material {
    pub material_id: String
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub enum InfrastructureKind {
    Path,
    Ramp{to_z: i32},
    Stairs{to_z: i32}
}

#[derive(Serialize, Deserialize, Debug, Clone, Default)]
pub struct BuildingId {
    pub building_id: String,
    pub template_id: String
}

#[derive(Serialize, Deserialize, Debug, Default)]
pub struct ParkMap {
    pub map_id: String,
    pub bounds: Bounds3d,
    pub terrain: HashMap<(i32, i32, i32), Material>,
    pub infrastructure: HashMap<(i32, i32, i32), InfrastructureKind>,
    pub buildings: HashMap<(i32, i32, i32), BuildingId>,
    pub parcels: Vec<Parcel>,
    pub unlocked_levels: HashSet<i32>,
}

impl ParkMap {

    pub fn new(map_id: String, bounds: Bounds3d) -> Self {
        let mut unlocked_levels = HashSet::new();
        unlocked_levels.insert(0);

        Self {
            map_id,
            bounds,
            terrain: HashMap::new(),
            infrastructure: HashMap::new(),
            buildings: HashMap::new(),
            parcels: Vec::new(),
            unlocked_levels
        }
    }

    pub fn set_terrain(&mut self, x: i32, y: i32, z: i32, material: Material) {
        self.terrain.insert((x, y, z), material);
    }

    pub fn get_terrain(&self, x: i32, y: i32, z: i32) -> Option<&Material> {
        self.terrain.get(&(x, y, z))
    }

    pub fn set_infrastructure(&mut self, x: i32, y: i32, z: i32, infrastructure_kind: InfrastructureKind) {
        self.infrastructure.insert((x, y, z), infrastructure_kind);
    }

    pub fn get_infrastructure(&self, x: i32, y: i32, z: i32) -> Option<&InfrastructureKind> {
        self.infrastructure.get(&(x, y, z))
    }

    pub fn set_buildings(&mut self, x: i32, y: i32, z: i32, building_id: BuildingId) {
        self.buildings.insert((x, y, z), building_id);
    }

    pub fn get_buildings(&self, x: i32, y: i32, z: i32) -> Option<&BuildingId> {
        self.buildings.get(&(x, y, z))
    }

    pub fn parcel_at(&self, x: i32, y: i32) -> Option<&Parcel> {
        self.parcels.iter().find(|p| p.cells.contains(&(x, y)))
    }

    pub fn is_unlocked(&self, x: i32, y: i32) -> bool {
        self.parcel_at(x, y).is_some_and(|p| p.unlocked)
    }

    fn is_within_bounds(&self, x: i32, y: i32, z: i32) -> bool {
        x>= self.bounds.min_x && x <= self.bounds.max_x
        && y >= self.bounds.min_y && y <= self.bounds.max_y
        && z >= self.bounds.min_z && z <= self.bounds.max_z 
    }

    fn is_level_available(&self, z: i32) -> bool {
        self.unlocked_levels.contains(&z)
    }

    pub fn can_apply_terrain(&self, x: i32, y: i32, z: i32) -> Result<(), ErrorCode> {
        if !self.is_within_bounds(x, y, z) || !self.is_level_available(z) {
            return Err(ErrorCode::ErrorOutOfBounds);
        }
        if !self.is_unlocked(x, y) {
            return Err(ErrorCode::ErrorOutOfBounds);
        }
        if self.get_buildings(x, y, z).is_some() {
            return Err(ErrorCode::ErrorCollision);
        }
        Ok(())
    }

    pub fn can_place_infrastructure(&self, kind: InfrastructureKind, to_z: i32, coordinates: &[(i32, i32, i32)]) -> Result<(), ErrorCode> {
        for &(x, y,z) in coordinates {
            if !self.is_within_bounds(x, y, z) || !self.is_level_available(z) {
                return Err(ErrorCode::ErrorOutOfBounds);
            }
            if !self.is_unlocked(x, y) {
                return Err(ErrorCode::ErrorOutOfBounds);
            }
            if self.get_buildings(x, y, z).is_some() {
                return Err(ErrorCode::ErrorCollision);
            }
            if z==0 {
                if let Some(material) = self.get_terrain(x, y, z) {
                    if material.material_id == "water" {
                        // Add other material_id maybe with an inside field
                        return Err(ErrorCode::ErrorCrossingNotAllowed);
                    }
                }
            }
            if z == 1 || z == -1 {
                if let Some(building) = self.get_buildings(x, y, 0) {
                    let (allows_above, allows_below) = crossing_flags_for(&building.template_id);
                    let allowed = if z == 1 { allows_above } else { allows_below };
                    if !allowed {
                        return Err(ErrorCode::ErrorCrossingNotAllowed);
                    }
                }
            }
            if !matches!(kind, InfrastructureKind::Path) {
                if !self.is_level_available(to_z) {
                    return Err(ErrorCode::ErrorOutOfBounds);
                }
                if (to_z - z).abs() != 1 {
                    return Err(ErrorCode::ErrorCrossingNotAllowed);
                }
             }
        }
        Ok(())
    }

    pub fn can_place_building(&self, origin: (i32, i32, i32), footprint: &[(i32, i32)], rotation: Rotation, template_id: &str) -> Result<(), ErrorCode> {
        let (ox, oy, oz) = origin;
        let rotated = rotate_footprint(footprint, rotation);

        for (dx, dy) in rotated {
            let (x, y, z) = (ox + dx, oy + dy, oz);

            if !self.is_within_bounds(x, y, z) || !self.is_level_available(z){
                return Err(ErrorCode::ErrorOutOfBounds);
            }
            if !self.is_unlocked(x, y) {
                return Err(ErrorCode::ErrorOutOfBounds);
            }
            if self.get_buildings(x, y, z).is_some() {
                return Err(ErrorCode::ErrorCollision);
            }
            if let Some(material) = self.get_terrain(x, y, z) {
                if material.material_id == "water" {
                    //Careful: "water" hardly coded to be redefined in function of the property of the material {block_paths: bool}
                    return Err(ErrorCode::ErrorCollision);
                }
            }
        }
        Ok(())
    }

    pub fn can_remove(&self, x: i32, y: i32, z: i32) -> Result<(), ErrorCode> {
        if !self.is_within_bounds(x, y, z) {
            return Err(ErrorCode::ErrorOutOfBounds);
        }
        if self.get_buildings(x, y, z).is_none() && self.get_infrastructure(x, y, z).is_none() && self.get_terrain(x, y, z).is_none() {
            return Err(ErrorCode::ErrorCollision);
        }

        Ok(())
    }

}

#[cfg(test)]
mod tests;