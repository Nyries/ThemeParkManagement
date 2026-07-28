use std::collections::{HashMap, HashSet};
use serde::{Serialize, Deserialize};

use crate::simulation::{ErrorCode, Rotation};

pub(crate) fn movement_cost_for(material_id: &str) -> u32 {
    match material_id {
        "path" => 1,
        "stairs" | "ramp" => 2,
        "grass" => 5,
        _ => 10,
    }
}

pub(crate) fn vertical_movement_cost_for(shape: &InfrastructureShape) -> u32 {
    match shape {
        InfrastructureShape::Ramp { .. } => 1,
        InfrastructureShape::Stairs { .. } => 2,
        InfrastructureShape::Path => 0,
    }
}

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

pub(crate) fn footprint_for(template_id: &str) -> Vec<(i32, i32)> {
    //TODO: create the JSON sparser from catalog
    match template_id {
        _ => [(0,0), (0,1), (1,1)].to_vec()
    }
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

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub enum InfrastructureShape {
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
    pub terrain: HashMap<(i32, i32, i32), String /*Material */ >,
    pub infrastructure: HashMap<(i32, i32, i32), InfrastructureShape>,
    pub building: HashMap<(i32, i32, i32), BuildingId>,
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
            building: HashMap::new(),
            parcels: Vec::new(),
            unlocked_levels
        }
    }

    pub fn set_terrain(&mut self, x: i32, y: i32, z: i32, material: String) {
        self.terrain.insert((x, y, z), material);
    }

    pub fn get_terrain(&self, x: i32, y: i32, z: i32) -> Option<&String> {
        self.terrain.get(&(x, y, z))
    }

    pub fn set_infrastructure(&mut self, x: i32, y: i32, z: i32, infrastructure_kind: InfrastructureShape) {
        self.infrastructure.insert((x, y, z), infrastructure_kind);
    }

    pub fn remove_infrasture(&mut self, x: i32, y: i32, z: i32) {
        self.infrastructure.remove(&(x, y, z));
    }

    pub fn get_infrastructure(&self, x: i32, y: i32, z: i32) -> Option<&InfrastructureShape> {
        self.infrastructure.get(&(x, y, z))
    }

    pub fn set_building(&mut self, x: i32, y: i32, z: i32, building_id: BuildingId) {
        self.building.insert((x, y, z), building_id);
    }

    pub fn get_building(&self, x: i32, y: i32, z: i32) -> Option<&BuildingId> {
        self.building.get(&(x, y, z))
    }

    pub fn get_building_coords_by_building_id(&self, building_id: &str) -> Vec<(i32, i32, i32)>{
        self.building.iter()
            .filter(|(_,b)| b.building_id == building_id)
            .map(|(&coord, _)| coord)
            .collect()
    }

    pub fn place_building(&mut self, origin: (i32, i32, i32), footprint: &[(i32, i32)], rotation: Rotation, building_id: BuildingId) { 
        let (ox, oy, oz) = origin;
        let rotated = rotate_footprint(footprint, rotation);

        for (dx, dy) in rotated {
            self.set_building(ox + dx, oy + dy, oz, building_id.clone());
        }
    }
    
    pub fn remove_building(&mut self, x: i32, y: i32, z: i32) {
        if let Some(building) = self.get_building(x, y, z) {
            let building_id = building.building_id.clone();
            for coord in self.get_building_coords_by_building_id(&building_id) {
                self.building.remove(&coord);
            }
        }
    }

    pub fn parcel_at(&self, x: i32, y: i32) -> Option<&Parcel> {
        self.parcels.iter().find(|p| p.cells.contains(&(x, y)))
    }

    pub fn is_unlocked(&self, x: i32, y: i32) -> bool {
        self.parcel_at(x, y).is_some_and(|p| p.unlocked)
    }

    pub(crate) fn is_within_bounds(&self, x: i32, y: i32, z: i32) -> bool {
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
        if self.get_building(x, y, z).is_some() {
            return Err(ErrorCode::ErrorCollision);
        }
        Ok(())
    }

    pub fn can_place_infrastructure(&self, kind: InfrastructureShape, to_z: i32, coordinates: &[(i32, i32, i32)]) -> Result<(), ErrorCode> {
        for &(x, y,z) in coordinates {
            if !self.is_within_bounds(x, y, z) || !self.is_level_available(z) {
                return Err(ErrorCode::ErrorOutOfBounds);
            }
            if !self.is_unlocked(x, y) {
                return Err(ErrorCode::ErrorOutOfBounds);
            }
            if self.get_building(x, y, z).is_some() {
                return Err(ErrorCode::ErrorCollision);
            }
            if z==0 {
                if let Some(material) = self.get_terrain(x, y, z) {
                    if material == "water" {
                        // Add other material_id maybe with an inside field
                        return Err(ErrorCode::ErrorCrossingNotAllowed);
                    }
                }
            }
            if z == 1 || z == -1 {
                if let Some(building) = self.get_building(x, y, 0) {
                    let (allows_above, allows_below) = crossing_flags_for(&building.template_id);
                    let allowed = if z == 1 { allows_above } else { allows_below };
                    if !allowed {
                        return Err(ErrorCode::ErrorCrossingNotAllowed);
                    }
                }
            }
            if !matches!(kind, InfrastructureShape::Path) {
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

    pub fn can_remove_infrastructure(&self, coordinates: &[(i32, i32, i32)]) -> Result<(), ErrorCode> {
        for &(x, y, z) in coordinates {
            if !self.is_within_bounds(x, y, z) || !self.is_level_available(z) {
                return Err(ErrorCode::ErrorOutOfBounds);
            }
            if self.get_infrastructure(x, y, z).is_none() {
                return Err(ErrorCode::ErrorCollision);
            }
        }
        Ok(())
    }

    pub fn can_place_building(&self, origin: (i32, i32, i32), footprint: &[(i32, i32)], rotation: Rotation) -> Result<(), ErrorCode> {
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
            if self.get_building(x, y, z).is_some() {
                return Err(ErrorCode::ErrorCollision);
            }
            if let Some(material) = self.get_terrain(x, y, z) {
                if material == "water" {
                    //Careful: "water" hardly coded to be redefined in function of the property of the material {block_paths: bool}
                    return Err(ErrorCode::ErrorCollision);
                }
            }
        }
        Ok(())
    }

    pub fn can_remove_building(&self, x: i32, y: i32, z: i32) -> Result<(), ErrorCode> {
        if !self.is_within_bounds(x, y, z) {
            return Err(ErrorCode::ErrorOutOfBounds);
        }
        if self.get_building(x, y, z).is_none() {
            return Err(ErrorCode::ErrorCollision);
        }

        Ok(())
    }


}

#[cfg(test)]
mod tests;