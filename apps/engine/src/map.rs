use std::collections::HashMap;
use serde::{Serialize, Deserialize};

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
    pub building_id: String
}

#[derive(Serialize, Deserialize, Debug, Default)]
pub struct ParkMap {
    pub map_id: String,
    pub bounds: Bounds3d,
    pub terrain: HashMap<(i32, i32, i32), Material>,
    pub infrastructure: HashMap<(i32, i32, i32), InfrastructureKind>,
    pub buildings: HashMap<(i32, i32, i32), BuildingId>,
    pub parcels: Vec<Parcel>,
}

impl ParkMap {

    pub fn new(map_id: String, bounds: Bounds3d) -> Self {
        Self {
            map_id,
            bounds,
            terrain: HashMap::new(),
            infrastructure: HashMap::new(),
            buildings: HashMap::new(),
            parcels: Vec::new()
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

}

#[cfg(test)]
mod tests {
    use super::*;

    fn build_test_map() -> ParkMap {
        let map_id = "map-1".to_string();
        let bounds_3d= Bounds3d::new(0,0,0,0,0,0);
        ParkMap::new(map_id, bounds_3d)
    }

    #[test]
    fn test_set_and_get_terrain() {
        let mut park_map = build_test_map();

        assert!(park_map.get_terrain(0, 0, 0).is_none());

        park_map.set_terrain(0, 0, 0, Material { material_id: "grass".into() });

        let material = park_map.get_terrain(0,0,0).expect("The terrain should exist");
        assert_eq!(material.material_id, "grass");
    }

    #[test]
    fn test_set_and_get_infrastructure() {
        let mut park_map = build_test_map();

        assert!(park_map.get_infrastructure(0, 0, 0).is_none());

        park_map.set_infrastructure(0, 0, 0, InfrastructureKind::Path);

        let infrastructure_kind = park_map.get_infrastructure(0,0,0).expect("The infrastructure should exist");
        assert_eq!(infrastructure_kind, &InfrastructureKind::Path);
    }

    #[test]
    fn test_set_and_get_buildings() {
        let mut park_map = build_test_map();

        assert!(park_map.get_buildings(0, 0, 0).is_none());

        park_map.set_buildings(0, 0, 0, BuildingId { building_id: "coaster-1".into()});

        let building_id = park_map.get_buildings(0,0,0).expect("The terrain should exist");
        assert_eq!(building_id.building_id, "coaster-1");
    }

    #[test]
    fn test_parcel_at_and_is_unlocked() {
        let mut park_map = build_test_map();

        assert!(park_map.parcel_at(0, 0).is_none());
        assert!(!park_map.is_unlocked(0, 0));

        park_map.parcels.push(Parcel {
            id: "start".into(),
            cells: vec![(0, 0)],
            unlocked: true,
            price: 0,
        });

        assert!(park_map.is_unlocked(0, 0));
        assert!(!park_map.is_unlocked(5, 5)); // hors de toute parcelle
    }
}