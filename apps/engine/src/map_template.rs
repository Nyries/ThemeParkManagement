use serde::Deserialize;

use crate::map::{Bounds3d, BuildingId, InfrastructureShape, Parcel, ParkMap};

pub enum MapSource {
    Embedded(&'static str),
    File(std::path::PathBuf),
}

#[derive(Debug)]
pub enum MapLoadError {
    InvalidJson(String),
    OutOfBounds { x: i32, y: i32, z: i32 },
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
    pub x: i32,
    pub y: i32,
    pub z: i32,
    pub material: String,
}

#[derive(Deserialize)]
pub struct InfrastructureEntry {
    pub x: i32,
    pub y: i32,
    pub z: i32,
    pub kind: String,
    pub to_z: Option<i32>,
}

#[derive(Deserialize)]
pub struct BuildingEntry {
    pub x: i32,
    pub y: i32,
    pub z: i32,
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
pub struct Coord {
    pub x: i32,
    pub y: i32,
    pub z: i32,
}

fn parse_infrastructure_kind(
    kind: &str,
    to_z: Option<i32>,
) -> Result<InfrastructureShape, MapLoadError> {
    match kind {
        "path" => Ok(InfrastructureShape::Path),
        "ramp" => {
            let to_z = to_z.ok_or_else(|| {
                MapLoadError::InvalidJson("infrastructure ramp without to_z".into())
            })?;
            Ok(InfrastructureShape::Ramp { to_z })
        }
        "stairs" => {
            let to_z = to_z.ok_or_else(|| {
                MapLoadError::InvalidJson("infrastructure stairs without to_z".into())
            })?;
            Ok(InfrastructureShape::Stairs { to_z })
        }
        other => Err(MapLoadError::UnknownInfrastructureKind(other.to_string())),
    }
}

#[derive(Deserialize)]
pub struct MapTemplate {
    pub archetype: String,
    pub name: String,
    pub dimensions: Dimensions,
    pub default_terrain: String,
    pub terrain: Vec<TerrainEntry>,
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
        serde_json::from_str(&raw).map_err(|e| MapLoadError::InvalidJson(e.to_string()))
    }

    pub fn into_park_map(&self) -> Result<ParkMap, MapLoadError> {
        let min_z = *self
            .dimensions
            .levels
            .iter()
            .min()
            .ok_or_else(|| MapLoadError::InvalidJson("dimensions.levels is empty".into()))?;
        let max_z = *self.dimensions.levels.iter().max().unwrap();
        let bounds = Bounds3d::new(
            0,
            self.dimensions.width - 1,
            0,
            self.dimensions.height - 1,
            min_z,
            max_z,
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
                return Err(MapLoadError::OutOfBounds {
                    x: terrain_entry.x,
                    y: terrain_entry.y,
                    z: terrain_entry.z,
                });
            }
            park_map.set_terrain(
                terrain_entry.x,
                terrain_entry.y,
                terrain_entry.z,
                terrain_entry.material.clone(),
            );
        }
        // Placing infrastructure
        for infrastructure_entry in &self.infrastructure {
            if !park_map.is_within_bounds(
                infrastructure_entry.x,
                infrastructure_entry.y,
                infrastructure_entry.z,
            ) {
                return Err(MapLoadError::OutOfBounds {
                    x: infrastructure_entry.x,
                    y: infrastructure_entry.y,
                    z: infrastructure_entry.z,
                });
            }
            let shape =
                parse_infrastructure_kind(&infrastructure_entry.kind, infrastructure_entry.to_z)?;
            park_map.set_infrastructure(
                infrastructure_entry.x,
                infrastructure_entry.y,
                infrastructure_entry.z,
                shape,
            );
        }
        // Placing the buildings
        for building_entry in &self.buildings {
            if !park_map.is_within_bounds(building_entry.x, building_entry.y, building_entry.z) {
                return Err(MapLoadError::OutOfBounds {
                    x: building_entry.x,
                    y: building_entry.y,
                    z: building_entry.z,
                });
            }
            let building_id = BuildingId {
                building_id: building_entry.building_id.clone(),
                template_id: building_entry.template_id.clone(),
            };
            park_map.set_building(
                building_entry.x,
                building_entry.y,
                building_entry.z,
                building_id,
            );
        }
        // Splitting map into parcels
        for parcel_entry in &self.parcels {
            for &(x, y) in &parcel_entry.cells {
                if !park_map.is_within_bounds(x, y, park_map.bounds.min_z) {
                    return Err(MapLoadError::OutOfBounds {
                        x,
                        y,
                        z: park_map.bounds.min_z,
                    });
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
        if park_map
            .get_infrastructure(self.entrance.x, self.entrance.y, self.entrance.z)
            .is_none()
        {
            return Err(MapLoadError::InvalidEntrance);
        }
        park_map.entrance = Some((self.entrance.x, self.entrance.y, self.entrance.z));
        Ok(park_map)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    mod parse_infrastructure_kind {
        use super::*;

        #[test]
        fn test_path_returns_path_shape() {
            let result = parse_infrastructure_kind("path", None);

            assert_eq!(result.unwrap(), InfrastructureShape::Path);
        }

        #[test]
        fn test_ramp_with_to_z_returns_ramp_shape() {
            let result = parse_infrastructure_kind("ramp", Some(1));

            assert_eq!(result.unwrap(), InfrastructureShape::Ramp { to_z: 1 });
        }

        #[test]
        fn test_ramp_without_to_z_fails() {
            let result = parse_infrastructure_kind("ramp", None);

            assert!(matches!(result, Err(MapLoadError::InvalidJson(_))));
        }

        #[test]
        fn test_stairs_with_to_z_returns_stairs_shape() {
            let result = parse_infrastructure_kind("stairs", Some(-1));

            assert_eq!(result.unwrap(), InfrastructureShape::Stairs { to_z: -1 });
        }

        #[test]
        fn test_stairs_without_to_z_fails() {
            let result = parse_infrastructure_kind("stairs", None);

            assert!(matches!(result, Err(MapLoadError::InvalidJson(_))));
        }

        #[test]
        fn test_unknown_kind_fails() {
            let result = parse_infrastructure_kind("teleporter", None);

            assert!(
                matches!(result, Err(MapLoadError::UnknownInfrastructureKind(k)) if k == "teleporter")
            );
        }
    }

    mod load {
        use super::*;

        const VALID_JSON: &str = r#"{
            "archetype": "test",
            "name": "Test",
            "dimensions": { "width": 2, "height": 2, "levels": [0] },
            "default_terrain": "grass",
            "terrain": [],
            "infrastructure": [],
            "buildings": [],
            "parcels": [],
            "entrance": { "x": 0, "y": 0, "z": 0 }
        }"#;

        fn write_temp_json(content: &str) -> std::path::PathBuf {
            let path =
                std::env::temp_dir().join(format!("map_template_test_{}.json", uuid::Uuid::new_v4()));
            std::fs::write(&path, content).unwrap();
            path
        }

        #[test]
        fn test_load_embedded_valid_json_succeeds() {
            let result = MapTemplate::load(MapSource::Embedded(VALID_JSON));

            assert!(result.is_ok());
            let template = result.unwrap();
            assert_eq!(template.archetype, "test");
            assert_eq!(template.dimensions.width, 2);
        }

        #[test]
        fn test_load_embedded_malformed_json_fails() {
            let result = MapTemplate::load(MapSource::Embedded("{ not valid json"));

            assert!(matches!(result, Err(MapLoadError::InvalidJson(_))));
        }

        #[test]
        fn test_load_file_valid_json_succeeds() {
            let path = write_temp_json(VALID_JSON);

            let result = MapTemplate::load(MapSource::File(path.clone()));

            assert!(result.is_ok());
            let template = result.unwrap();
            assert_eq!(template.archetype, "test");

            std::fs::remove_file(&path).unwrap();
        }

        #[test]
        fn test_load_file_malformed_json_fails() {
            let path = write_temp_json("{ not valid json");

            let result = MapTemplate::load(MapSource::File(path.clone()));

            assert!(matches!(result, Err(MapLoadError::InvalidJson(_))));

            std::fs::remove_file(&path).unwrap();
        }

        #[test]
        fn test_load_file_missing_fails() {
            let path = std::env::temp_dir().join("this_file_does_not_exist_12345.json");

            let result = MapTemplate::load(MapSource::File(path));

            assert!(matches!(result, Err(MapLoadError::InvalidJson(_))));
        }
    }

    mod into_park_map {
        use super::*;

        fn base_template() -> MapTemplate {
            MapTemplate {
                archetype: "test".into(),
                name: "Test".into(),
                dimensions: Dimensions {
                    width: 2,
                    height: 2,
                    levels: vec![0],
                },
                default_terrain: "grass".into(),
                terrain: vec![],
                infrastructure: vec![InfrastructureEntry {
                    x: 0,
                    y: 0,
                    z: 0,
                    kind: "path".into(),
                    to_z: None,
                }],
                buildings: vec![],
                parcels: vec![],
                entrance: Coord { x: 0, y: 0, z: 0 },
            }
        }

        #[test]
        fn test_loads_all_four_layers_correctly() {
            let mut template = base_template();
            template.buildings.push(BuildingEntry {
                x: 1,
                y: 0,
                z: 0,
                building_id: "b1".into(),
                template_id: "restaurant-1".into(),
            });
            template.parcels.push(ParcelEntry {
                id: "p1".into(),
                cells: vec![(0, 0), (1, 0), (0, 1), (1, 1)],
                unlocked: true,
                price: 0,
            });

            let park_map = template.into_park_map().unwrap();

            assert_eq!(park_map.get_terrain(0, 1, 0), Some(&"grass".to_string()));
            assert_eq!(
                park_map.get_infrastructure(0, 0, 0),
                Some(&InfrastructureShape::Path)
            );
            assert!(park_map.get_building(1, 0, 0).is_some());
            assert!(park_map.is_unlocked(0, 0));
        }

        #[test]
        fn test_default_terrain_applied_to_unlisted_cell() {
            let template = base_template();

            let park_map = template.into_park_map().unwrap();

            assert_eq!(park_map.get_terrain(1, 1, 0), Some(&"grass".to_string()));
        }

        #[test]
        fn test_terrain_exception_overrides_default() {
            let mut template = base_template();
            template.terrain.push(TerrainEntry {
                x: 1,
                y: 1,
                z: 0,
                material: "water".into(),
            });

            let park_map = template.into_park_map().unwrap();

            assert_eq!(park_map.get_terrain(1, 1, 0), Some(&"water".to_string()));
        }

        #[test]
        fn test_out_of_bounds_terrain_entry_fails_explicitly() {
            let mut template = base_template();
            template.terrain.push(TerrainEntry {
                x: 99,
                y: 0,
                z: 0,
                material: "grass".into(),
            });

            let result = template.into_park_map();

            assert!(matches!(
                result,
                Err(MapLoadError::OutOfBounds { x: 99, y: 0, z: 0 })
            ));
        }

        #[test]
        fn test_out_of_bounds_parcel_cell_fails_explicitly() {
            let mut template = base_template();
            template.parcels.push(ParcelEntry {
                id: "p1".into(),
                cells: vec![(99, 99)],
                unlocked: true,
                price: 0,
            });

            let result = template.into_park_map();

            assert!(matches!(
                result,
                Err(MapLoadError::OutOfBounds { x: 99, y: 99, .. })
            ));
        }

        #[test]
        fn test_unknown_infrastructure_kind_fails_explicitly() {
            let mut template = base_template();
            template.infrastructure.push(InfrastructureEntry {
                x: 1,
                y: 1,
                z: 0,
                kind: "teleporter".into(),
                to_z: None,
            });

            let result = template.into_park_map();

            assert!(
                matches!(result, Err(MapLoadError::UnknownInfrastructureKind(k)) if k == "teleporter")
            );
        }

        #[test]
        fn test_entrance_matching_infrastructure_succeeds() {
            let template = base_template();

            let park_map = template.into_park_map().unwrap();

            assert_eq!(park_map.entrance, Some((0, 0, 0)));
        }

        #[test]
        fn test_entrance_without_infrastructure_fails_explicitly() {
            let mut template = base_template();
            template.entrance = Coord { x: 1, y: 1, z: 0 };

            let result = template.into_park_map();

            assert!(matches!(result, Err(MapLoadError::InvalidEntrance)));
        }

        #[test]
        fn test_empty_levels_fails_explicitly() {
            let mut template = base_template();
            template.dimensions.levels = vec![];

            let result = template.into_park_map();

            assert!(matches!(result, Err(MapLoadError::InvalidJson(_))));
        }
    }

    mod first_map_fixture {
        use super::*;

        #[test]
        fn test_first_map_json_loads_into_valid_park_map() {
            let template = MapTemplate::load(MapSource::Embedded(include_str!(
                "../assets/maps/first-map.json"
            )))
            .expect("first-map.json should parse");

            let park_map = template
                .into_park_map()
                .expect("first-map.json should build a valid ParkMap");

            assert_eq!(park_map.entrance, Some((1, 1, 0)));
            assert!(park_map.get_infrastructure(1, 1, 0).is_some());
        }
    }
}
