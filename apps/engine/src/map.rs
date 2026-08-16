use rand::seq::IteratorRandom;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};

use crate::{
    building_template::BuildingCatalog,
    simulation::{ErrorCode, Rotation},
};

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

pub(crate) fn base_speed_for(shape: &InfrastructureShape) -> f32 {
    match shape {
        InfrastructureShape::Path => 1.0,
        InfrastructureShape::Ramp { .. } => 0.7,
        InfrastructureShape::Stairs { .. } => 0.5,
    }
}

fn rotate_footprint(footprint: &[(i32, i32)], rotation: Rotation) -> Vec<(i32, i32)> {
    footprint
        .iter()
        .map(|&(dx, dy)| match rotation {
            Rotation::Deg0 => (dx, dy),
            Rotation::Deg90 => (-dy, dx),
            Rotation::Deg180 => (-dx, -dy),
            Rotation::Deg270 => (dy, -dx),
        })
        .collect()
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
    Ramp { to_z: i32 },
    Stairs { to_z: i32 },
}

#[derive(Serialize, Deserialize, Debug, Clone, Default)]
pub struct BuildingId {
    pub building_id: String,
    pub template_id: String,
}

#[derive(Serialize, Deserialize, Debug, Default)]
pub struct ParkMap {
    pub map_id: String,
    pub bounds: Bounds3d,
    pub terrain: HashMap<(i32, i32, i32), String /*Material */>,
    pub infrastructure: HashMap<(i32, i32, i32), InfrastructureShape>,
    pub building: HashMap<(i32, i32, i32), BuildingId>,
    pub parcels: Vec<Parcel>,
    pub unlocked_levels: HashSet<i32>,
    pub entrance: Option<(i32, i32, i32)>,
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
            unlocked_levels,
            entrance: None,
        }
    }

    pub fn set_terrain(&mut self, x: i32, y: i32, z: i32, material: String) {
        self.terrain.insert((x, y, z), material);
    }

    pub fn get_terrain(&self, x: i32, y: i32, z: i32) -> Option<&String> {
        self.terrain.get(&(x, y, z))
    }

    pub fn set_infrastructure(
        &mut self,
        x: i32,
        y: i32,
        z: i32,
        infrastructure_kind: InfrastructureShape,
    ) {
        self.infrastructure.insert((x, y, z), infrastructure_kind);
    }

    pub fn remove_infrasture(&mut self, x: i32, y: i32, z: i32) {
        self.infrastructure.remove(&(x, y, z));
    }

    pub fn get_infrastructure(&self, x: i32, y: i32, z: i32) -> Option<&InfrastructureShape> {
        self.infrastructure.get(&(x, y, z))
    }

    pub fn random_walkable_cell(&self, exclude: (i32, i32, i32)) -> Option<(i32, i32, i32)> {
        let mut rng = rand::thread_rng();
        self.infrastructure
            .keys()
            .filter(|&&cell| cell != exclude)
            .choose(&mut rng)
            .copied()
            .or(if self.infrastructure.is_empty() {
                None
            } else {
                Some(exclude)
            })
    }

    pub fn set_building(&mut self, x: i32, y: i32, z: i32, building_id: BuildingId) {
        self.building.insert((x, y, z), building_id);
    }

    pub fn get_building(&self, x: i32, y: i32, z: i32) -> Option<&BuildingId> {
        self.building.get(&(x, y, z))
    }

    pub fn get_building_coords_by_building_id(&self, building_id: &str) -> Vec<(i32, i32, i32)> {
        self.building
            .iter()
            .filter(|(_, b)| b.building_id == building_id)
            .map(|(&coord, _)| coord)
            .collect()
    }

    pub fn place_building(
        &mut self,
        origin: (i32, i32, i32),
        footprint: &[(i32, i32)],
        rotation: Rotation,
        building_id: BuildingId,
    ) {
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
        x >= self.bounds.min_x
            && x <= self.bounds.max_x
            && y >= self.bounds.min_y
            && y <= self.bounds.max_y
            && z >= self.bounds.min_z
            && z <= self.bounds.max_z
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

    pub fn can_place_infrastructure(
        &self,
        catalog: &BuildingCatalog,
        kind: InfrastructureShape,
        to_z: i32,
        coordinates: &[(i32, i32, i32)],
    ) -> Result<(), ErrorCode> {
        for &(x, y, z) in coordinates {
            if !self.is_within_bounds(x, y, z) || !self.is_level_available(z) {
                return Err(ErrorCode::ErrorOutOfBounds);
            }
            if !self.is_unlocked(x, y) {
                return Err(ErrorCode::ErrorOutOfBounds);
            }
            if self.get_building(x, y, z).is_some() {
                return Err(ErrorCode::ErrorCollision);
            }
            if z == 0
                && let Some(material) = self.get_terrain(x, y, z)
                && material == "water"
            {
                // Add other material_id maybe with an inside field
                return Err(ErrorCode::ErrorCrossingNotAllowed);
            }
            if (z == 1 || z == -1)
                && let Some(building) = self.get_building(x, y, 0)
            {
                let allowed = catalog.get(&building.template_id).is_some_and(|t| {
                    if z == 1 {
                        t.crossing_flags.bridge_above_allowed
                    } else {
                        t.crossing_flags.tunnel_below_allowed
                    }
                });
                if !allowed {
                    return Err(ErrorCode::ErrorCrossingNotAllowed);
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

    pub fn can_remove_infrastructure(
        &self,
        coordinates: &[(i32, i32, i32)],
    ) -> Result<(), ErrorCode> {
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

    pub fn can_place_building(
        &self,
        origin: (i32, i32, i32),
        footprint: &[(i32, i32)],
        rotation: Rotation,
    ) -> Result<(), ErrorCode> {
        let (ox, oy, oz) = origin;
        let rotated = rotate_footprint(footprint, rotation);

        for (dx, dy) in rotated {
            let (x, y, z) = (ox + dx, oy + dy, oz);

            if !self.is_within_bounds(x, y, z) || !self.is_level_available(z) {
                return Err(ErrorCode::ErrorOutOfBounds);
            }
            if !self.is_unlocked(x, y) {
                return Err(ErrorCode::ErrorOutOfBounds);
            }
            if self.get_building(x, y, z).is_some() {
                return Err(ErrorCode::ErrorCollision);
            }
            if let Some(material) = self.get_terrain(x, y, z)
                && material == "water"
            {
                //Careful: "water" hardly coded to be redefined in function of the property of the material {block_paths: bool}
                return Err(ErrorCode::ErrorCollision);
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
mod tests {
    use super::*;

    fn build_test_map() -> ParkMap {
        let map_id = "map-1".to_string();
        let bounds_3d = Bounds3d::new(0, 0, 0, 0, 0, 0);
        ParkMap::new(map_id, bounds_3d)
    }

    mod movement_cost_for_and_vertical {
        use super::*;

        #[test]
        fn test_movement_cost_for_with_different_material_id() {
            let mut result = movement_cost_for("path");
            assert_eq!(result, 1);
            result = movement_cost_for("ramp");
            assert_eq!(result, 2);
            result = movement_cost_for("grass");
            assert_eq!(result, 5);
            result = movement_cost_for("water");
            assert_eq!(result, 10);
        }

        #[test]
        fn test_vertical_movement_cost_for_with_different_infrastructure() {
            let mut result = vertical_movement_cost_for(&InfrastructureShape::Path);
            assert_eq!(result, 0);
            result = vertical_movement_cost_for(&InfrastructureShape::Ramp { to_z: 1 });
            assert_eq!(result, 1);
            result = vertical_movement_cost_for(&InfrastructureShape::Stairs { to_z: 1 });
            assert_eq!(result, 2);
        }
    }

    mod rotate_footprint {
        use super::*;

        #[test]
        fn test_rotate_footprint_0_degrees_is_identity() {
            let footprint = vec![(1, 0), (0, 1)];
            let rotated = rotate_footprint(&footprint, Rotation::Deg0);
            assert_eq!(rotated, footprint);
        }

        #[test]
        fn test_rotate_footprint_90_degrees() {
            let footprint = vec![(1, 0)];
            let rotated = rotate_footprint(&footprint, Rotation::Deg90);
            assert_eq!(rotated, vec![(0, 1)]);
        }

        #[test]
        fn test_rotate_footprint_180_degrees() {
            let footprint = vec![(1, 2)];
            let rotated = rotate_footprint(&footprint, Rotation::Deg180);
            assert_eq!(rotated, vec![(-1, -2)]);
        }

        #[test]
        fn test_rotate_footprint_270_degrees() {
            let footprint = vec![(1, 0)];
            let rotated = rotate_footprint(&footprint, Rotation::Deg270);
            assert_eq!(rotated, vec![(0, -1)]);
        }
    }

    mod terrain {
        use super::*;

        #[test]
        fn test_set_and_get_terrain() {
            let mut park_map = build_test_map();

            assert!(park_map.get_terrain(0, 0, 0).is_none());

            park_map.set_terrain(0, 0, 0, "grass".into());

            let material = park_map
                .get_terrain(0, 0, 0)
                .expect("The terrain should exist");
            assert_eq!(material, "grass");
        }
    }

    mod infrastructure {
        use super::*;

        #[test]
        fn test_set_get_remove_infrastructure() {
            let mut park_map = build_test_map();

            assert!(park_map.get_infrastructure(0, 0, 0).is_none());

            park_map.set_infrastructure(0, 0, 0, InfrastructureShape::Path);

            let infrastructure_kind = park_map
                .get_infrastructure(0, 0, 0)
                .expect("The infrastructure should exist");
            assert_eq!(infrastructure_kind, &InfrastructureShape::Path);

            park_map.remove_infrasture(0, 0, 0);
            assert!(park_map.get_infrastructure(0, 0, 0).is_none())
        }
    }

    mod random_walkable_cell {
        use super::*;

        #[test]
        fn test_returns_a_different_cell_when_several_are_walkable() {
            let mut map = build_test_map();
            map.set_infrastructure(0, 0, 0, InfrastructureShape::Path);
            map.set_infrastructure(1, 0, 0, InfrastructureShape::Path);

            let result = map.random_walkable_cell((0, 0, 0));

            assert_eq!(result, Some((1, 0, 0)));
        }

        #[test]
        fn test_falls_back_to_excluded_cell_when_it_is_the_only_walkable_one() {
            let mut map = build_test_map();
            map.set_infrastructure(0, 0, 0, InfrastructureShape::Path);

            let result = map.random_walkable_cell((0, 0, 0));

            assert_eq!(result, Some((0, 0, 0)));
        }

        #[test]
        fn test_returns_none_when_no_cell_is_walkable() {
            let map = build_test_map();

            let result = map.random_walkable_cell((0, 0, 0));

            assert_eq!(result, None);
        }
    }

    mod buildings {
        use super::*;

        #[test]
        fn test_set_and_get_building() {
            let mut park_map = build_test_map();

            assert!(park_map.get_building(0, 0, 0).is_none());

            park_map.set_building(
                0,
                0,
                0,
                BuildingId {
                    building_id: "coaster-1".into(),
                    template_id: "b&m-1".into(),
                },
            );

            let building_id = park_map
                .get_building(0, 0, 0)
                .expect("The building should exist");
            assert_eq!(building_id.building_id, "coaster-1");
        }

        #[test]
        fn test_place_building_and_get_building_coords_by_building_id() {
            let mut park_map = build_test_map();
            let origin = (0, 0, 0);
            let footprint = [(0, 0), (0, 1), (1, 1)];
            let rotation = Rotation::Deg180;
            let building_id = BuildingId {
                building_id: "building-1".to_string(),
                template_id: "restaurant-1".to_string(),
            };
            assert!(park_map.get_building(0, 0, 0).is_none());

            park_map.place_building(origin, &footprint, rotation, building_id);

            let mut coords = park_map.get_building_coords_by_building_id("building-1");
            coords.sort();
            assert_eq!(coords, [(-1, -1, 0), (0, -1, 0), (0, 0, 0)])
        }

        #[test]
        fn test_get_building_coords_by_building_id_that_doesnt_exist() {
            let park_map = build_test_map();

            let result = park_map.get_building_coords_by_building_id("non-existent");
            assert!(result.is_empty());
        }

        #[test]
        fn test_remove_building_on_empty_cell_does_nothing() {
            let mut park_map = build_test_map();

            park_map.remove_building(0, 0, 0);

            assert!(park_map.get_building(0, 0, 0).is_none())
        }

        #[test]
        fn test_remove_building_purges_full_footprint_from_any_cell() {
            let mut park_map = build_test_map();
            let origin = (0, 0, 0);
            let footprint = [(0, 0), (0, 1), (1, 1)];
            let building_id = BuildingId {
                building_id: "building-1".to_string(),
                template_id: "restaurant-1".to_string(),
            };

            park_map.place_building(origin, &footprint, Rotation::Deg0, building_id);
            assert!(park_map.get_building(0, 0, 0).is_some());
            assert!(park_map.get_building(0, 1, 0).is_some());
            assert!(park_map.get_building(1, 1, 0).is_some());

            park_map.remove_building(1, 1, 0);

            assert!(park_map.get_building(0, 0, 0).is_none());
            assert!(park_map.get_building(0, 1, 0).is_none());
            assert!(park_map.get_building(1, 1, 0).is_none());
        }
    }

    mod parcels_and_levels {
        use super::*;

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
            assert!(!park_map.is_unlocked(5, 5));
        }

        #[test]
        fn test_new_map_has_one_level() {
            let park_map = build_test_map();
            assert!(park_map.is_level_available(0));
            assert!(!park_map.is_level_available(-1));
            assert!(!park_map.is_level_available(1));
        }
    }

    mod can_apply_terrain {
        use super::*;

        #[test]
        fn test_can_apply_terrain_succeeds_on_unlocked_cell() {
            let mut park_map = build_test_map();
            park_map.parcels.push(Parcel {
                id: "start".into(),
                cells: vec![(0, 0)],
                unlocked: true,
                price: 0,
            });

            let result = park_map.can_apply_terrain(0, 0, 0);

            assert!(result.is_ok());
        }

        #[test]
        fn test_can_apply_terrain_fails_out_of_bounds() {
            let park_map = build_test_map();

            let result = park_map.can_apply_terrain(5, 5, 0);

            assert_eq!(result, Err(ErrorCode::ErrorOutOfBounds));
        }

        #[test]
        fn test_can_apply_terrain_fails_on_locked_parcel() {
            let park_map = build_test_map();

            let result = park_map.can_apply_terrain(0, 0, 0);

            assert_eq!(result, Err(ErrorCode::ErrorOutOfBounds));
        }

        #[test]
        fn test_can_apply_terrain_fails_on_building_collision() {
            let mut park_map = build_test_map();
            park_map.parcels.push(Parcel {
                id: "start".into(),
                cells: vec![(0, 0)],
                unlocked: true,
                price: 0,
            });
            park_map.set_building(
                0,
                0,
                0,
                BuildingId {
                    building_id: "coaster-1".into(),
                    template_id: "b&m-1".into(),
                },
            );

            let result = park_map.can_apply_terrain(0, 0, 0);

            assert_eq!(result, Err(ErrorCode::ErrorCollision));
        }

        #[test]
        fn test_can_apply_terrain_fails_on_locked_level() {
            let mut park_map = ParkMap::new("map-1".into(), Bounds3d::new(0, 0, 0, 0, 0, 1));
            park_map.parcels.push(Parcel {
                id: "start".into(),
                cells: vec![(0, 0)],
                unlocked: true,
                price: 0,
            });

            let result = park_map.can_apply_terrain(0, 0, 1);

            assert_eq!(result, Err(ErrorCode::ErrorOutOfBounds));
        }
    }

    mod can_place_infrastructure {
        use super::*;
        use crate::building_template::{BuildingCatalog, CatalogSource};

        fn test_catalog() -> BuildingCatalog {
            BuildingCatalog::load(CatalogSource::Embedded(r#"{
                "templates": [
                    {
                        "template_id": "bridge_support",
                        "name": "Bridge Support",
                        "category": "ShopUtility",
                        "footprint": [[0,0]],
                        "cost": 100,
                        "visitor_behavior": "short_stay",
                        "crossing_flags": { "bridge_above_allowed": true, "tunnel_below_allowed": true },
                        "needs_relief": {},
                        "tags": []
                    }
                ]
            }"#)).unwrap()
        }

        fn build_infra_test_map() -> ParkMap {
            let mut park_map = ParkMap::new("map-1".into(), Bounds3d::new(0, 0, 0, 0, -1, 2));
            park_map.unlocked_levels.insert(1);
            park_map.parcels.push(Parcel {
                id: "start".into(),
                cells: vec![(0, 0)],
                unlocked: true,
                price: 0,
            });
            park_map
        }

        #[test]
        fn test_can_place_infrastructure_succeeds_for_path() {
            let park_map = build_infra_test_map();

            // to_z ignoré pour Path
            let result = park_map.can_place_infrastructure(
                &test_catalog(),
                InfrastructureShape::Path,
                0,
                &[(0, 0, 0)],
            );

            assert!(result.is_ok());
        }

        #[test]
        fn test_can_place_infrastructure_fails_out_of_bounds() {
            let park_map = build_infra_test_map();

            let result = park_map.can_place_infrastructure(
                &test_catalog(),
                InfrastructureShape::Path,
                0,
                &[(5, 5, 0)],
            );

            assert_eq!(result, Err(ErrorCode::ErrorOutOfBounds));
        }

        #[test]
        fn test_can_place_infrastructure_fails_on_locked_parcel() {
            let park_map = ParkMap::new("map-1".into(), Bounds3d::new(0, 0, 0, 0, -1, 2)); // pas de parcelle

            let result = park_map.can_place_infrastructure(
                &test_catalog(),
                InfrastructureShape::Path,
                0,
                &[(0, 0, 0)],
            );

            assert_eq!(result, Err(ErrorCode::ErrorOutOfBounds));
        }

        #[test]
        fn test_can_place_infrastructure_fails_on_building_collision() {
            let mut park_map = build_infra_test_map();
            park_map.set_building(
                0,
                0,
                0,
                BuildingId {
                    building_id: "coaster-1".into(),
                    template_id: "b&m-1".into(),
                },
            );

            let result = park_map.can_place_infrastructure(
                &test_catalog(),
                InfrastructureShape::Path,
                0,
                &[(0, 0, 0)],
            );

            assert_eq!(result, Err(ErrorCode::ErrorCollision));
        }

        #[test]
        fn test_can_place_infrastructure_rejects_path_on_water_at_ground_level() {
            let mut park_map = build_infra_test_map();
            park_map.set_terrain(0, 0, 0, "water".into());

            let result = park_map.can_place_infrastructure(
                &test_catalog(),
                InfrastructureShape::Path,
                0,
                &[(0, 0, 0)],
            );

            assert_eq!(result, Err(ErrorCode::ErrorCrossingNotAllowed));
        }

        #[test]
        fn test_can_place_infrastructure_allows_path_bridge_over_water() {
            let mut park_map = build_infra_test_map();
            park_map.set_terrain(0, 0, 0, "water".into());

            // Pont = Path à z=+1 au-dessus de l'eau à z=0
            let result = park_map.can_place_infrastructure(
                &test_catalog(),
                InfrastructureShape::Path,
                0,
                &[(0, 0, 1)],
            );

            assert!(result.is_ok());
        }

        #[test]
        fn test_can_place_infrastructure_fails_crossing_not_allowed_above_building() {
            let mut park_map = build_infra_test_map();
            park_map.set_building(
                0,
                0,
                0,
                BuildingId {
                    building_id: "shop-1".into(),
                    template_id: "shop".into(),
                },
            );

            let result = park_map.can_place_infrastructure(
                &test_catalog(),
                InfrastructureShape::Path,
                0,
                &[(0, 0, 1)],
            );

            assert_eq!(result, Err(ErrorCode::ErrorCrossingNotAllowed));
        }

        #[test]
        fn test_can_place_infrastructure_allows_passerelle_above_building_with_flag() {
            let mut park_map = build_infra_test_map();
            park_map.set_building(
                0,
                0,
                0,
                BuildingId {
                    building_id: "support-1".into(),
                    template_id: "bridge_support".into(),
                },
            );

            let result = park_map.can_place_infrastructure(
                &test_catalog(),
                InfrastructureShape::Path,
                0,
                &[(0, 0, 1)],
            );

            assert!(result.is_ok());
        }

        #[test]
        fn test_can_place_infrastructure_succeeds_for_ramp_to_adjacent_level() {
            let park_map = build_infra_test_map();

            let result = park_map.can_place_infrastructure(
                &test_catalog(),
                InfrastructureShape::Ramp { to_z: 1 },
                1,
                &[(0, 0, 0)],
            );

            assert!(result.is_ok());
        }

        #[test]
        fn test_can_place_infrastructure_fails_for_ramp_to_non_adjacent_level() {
            let mut park_map = build_infra_test_map();
            park_map.unlocked_levels.insert(2);

            let result = park_map.can_place_infrastructure(
                &test_catalog(),
                InfrastructureShape::Ramp { to_z: 2 },
                2,
                &[(0, 0, 0)],
            );

            assert_eq!(result, Err(ErrorCode::ErrorCrossingNotAllowed));
        }

        #[test]
        fn test_can_place_infrastructure_fails_for_ramp_to_locked_level() {
            let park_map = build_infra_test_map();
            // to_z = -1 est dans les bornes mais pas dans unlocked_levels

            let result = park_map.can_place_infrastructure(
                &test_catalog(),
                InfrastructureShape::Stairs { to_z: -1 },
                -1,
                &[(0, 0, 0)],
            );

            assert_eq!(result, Err(ErrorCode::ErrorOutOfBounds));
        }
    }

    mod can_remove_infrastructure {
        use super::*;

        #[test]
        fn test_can_remove_infrastructure_succeeds_when_something_exists() {
            let mut park_map = build_test_map();
            park_map.set_infrastructure(0, 0, 0, InfrastructureShape::Path);

            assert!(park_map.can_remove_infrastructure(&[(0, 0, 0)]).is_ok());
        }

        #[test]
        fn test_can_remove_infrastructure_fails_on_empty_cell() {
            let park_map = build_test_map();

            let result = park_map.can_remove_infrastructure(&[(0, 0, 0)]);

            assert_eq!(result, Err(ErrorCode::ErrorCollision));
        }

        #[test]
        fn test_can_remove_infrastructure_fails_out_of_bounds() {
            let park_map = build_test_map();

            let result = park_map.can_remove_infrastructure(&[(5, 5, 0)]);

            assert_eq!(result, Err(ErrorCode::ErrorOutOfBounds));
        }

        #[test]
        fn test_can_remove_infrastructure_fails_on_locked_level() {
            let park_map = ParkMap::new("map-1".into(), Bounds3d::new(0, 0, 0, 0, 0, 1));
            // z=1 est dans les bornes mais pas dans unlocked_levels

            let result = park_map.can_remove_infrastructure(&[(0, 0, 1)]);

            assert_eq!(result, Err(ErrorCode::ErrorOutOfBounds));
        }
    }

    mod can_place_building {
        use super::*;

        fn build_placement_test_map() -> ParkMap {
            let mut park_map = ParkMap::new("map-1".into(), Bounds3d::new(0, 5, 0, 5, 0, 0));
            park_map.parcels.push(Parcel {
                id: "start".into(),
                cells: (0..=5).flat_map(|x| (0..=5).map(move |y| (x, y))).collect(),
                unlocked: true,
                price: 0,
            });
            park_map
        }

        #[test]
        fn test_can_place_building_succeeds_on_free_cells() {
            let park_map = build_placement_test_map();
            let footprint = vec![(0, 0), (1, 0), (1, 1)];

            let result = park_map.can_place_building((2, 2, 0), &footprint, Rotation::Deg90);

            assert!(result.is_ok());
        }

        #[test]
        fn test_can_place_building_fails_out_of_bounds() {
            let park_map = build_placement_test_map();
            let footprint = vec![(0, 0), (1, 0)];

            let result = park_map.can_place_building((5, 5, 0), &footprint, Rotation::Deg0);

            assert_eq!(result, Err(ErrorCode::ErrorOutOfBounds));
        }

        #[test]
        fn test_can_place_building_fails_on_locked_parcel() {
            let park_map = ParkMap::new("map-1".into(), Bounds3d::new(0, 5, 0, 5, 0, 0)); // no parcel created
            let footprint = vec![(0, 0)];

            let result = park_map.can_place_building((2, 2, 0), &footprint, Rotation::Deg270);

            assert_eq!(result, Err(ErrorCode::ErrorOutOfBounds));
        }

        #[test]
        fn test_can_place_building_fails_on_building_collision() {
            let mut park_map = build_placement_test_map();
            park_map.set_building(
                2,
                2,
                0,
                BuildingId {
                    building_id: "existing".into(),
                    template_id: "shop".into(),
                },
            );
            let footprint = vec![(0, 0)];

            let result = park_map.can_place_building((2, 2, 0), &footprint, Rotation::Deg0);

            assert_eq!(result, Err(ErrorCode::ErrorCollision));
        }

        #[test]
        fn test_can_place_building_fails_on_water() {
            let mut park_map = build_placement_test_map();
            park_map.set_terrain(2, 2, 0, "water".into());
            let footprint = vec![(0, 0)];

            let result = park_map.can_place_building((2, 2, 0), &footprint, Rotation::Deg0);

            assert_eq!(result, Err(ErrorCode::ErrorCollision));
        }

        #[test]
        fn test_can_place_building_fails_on_locked_level() {
            let mut park_map = ParkMap::new("map-1".into(), Bounds3d::new(0, 5, 0, 5, 0, 1));
            park_map.parcels.push(Parcel {
                id: "start".into(),
                cells: (0..=5).flat_map(|x| (0..=5).map(move |y| (x, y))).collect(),
                unlocked: true,
                price: 0,
            });
            // z=1 est dans les bornes mais pas dans unlocked_levels (seul z=0 l'est par défaut)
            let footprint = vec![(0, 0)];

            let result = park_map.can_place_building((2, 2, 1), &footprint, Rotation::Deg0);

            assert_eq!(result, Err(ErrorCode::ErrorOutOfBounds));
        }

        #[test]
        fn test_can_place_building_fails_on_collision_only_visible_after_rotation() {
            let mut park_map = build_placement_test_map();
            park_map.set_building(
                1,
                2,
                0,
                BuildingId {
                    building_id: "existing".into(),
                    template_id: "shop".into(),
                },
            );

            let footprint = vec![(0, 0), (0, 1)];

            let result_deg0 = park_map.can_place_building((2, 2, 0), &footprint, Rotation::Deg0);
            assert!(result_deg0.is_ok());

            let result_deg90 = park_map.can_place_building((2, 2, 0), &footprint, Rotation::Deg90);
            assert_eq!(result_deg90, Err(ErrorCode::ErrorCollision));
        }
    }

    mod can_remove_building {
        use super::*;

        #[test]
        fn test_can_remove_building_succeeds_when_something_exists() {
            let mut park_map = build_test_map();
            park_map.set_building(
                0,
                0,
                0,
                BuildingId {
                    building_id: "coaster-1".into(),
                    template_id: "b&m-giga".into(),
                },
            );

            assert!(park_map.can_remove_building(0, 0, 0).is_ok());
        }

        #[test]
        fn test_can_remove_building_fails_on_empty_cell() {
            let park_map = build_test_map();

            let result = park_map.can_remove_building(0, 0, 0);

            assert_eq!(result, Err(ErrorCode::ErrorCollision));
        }

        #[test]
        fn test_can_remove_building_fails_out_of_bounds() {
            let park_map = build_test_map();

            let result = park_map.can_remove_building(5, 5, 0);

            assert_eq!(result, Err(ErrorCode::ErrorOutOfBounds));
        }
    }
}
