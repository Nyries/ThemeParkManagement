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

        assert!(matches!(result, Err(MapLoadError::UnknownInfrastructureKind(k)) if k == "teleporter"));
    }
}

mod load {
    use super::*;

    fn write_temp_json(content: &str) -> std::path::PathBuf {
        let path = std::env::temp_dir().join(format!("map_template_test_{}.json", uuid::Uuid::new_v4()));
        std::fs::write(&path, content).unwrap();
        path
    }

    #[test]
    fn test_load_valid_json_succeeds() {
        let json = r#"{
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
        let path = write_temp_json(json);

        let result = MapTemplate::load(&path);

        assert!(result.is_ok());
        let template = result.unwrap();
        assert_eq!(template.archetype, "test");
        assert_eq!(template.dimensions.width, 2);

        std::fs::remove_file(&path).unwrap();
    }

    #[test]
    fn test_load_malformed_json_fails() {
        let path = write_temp_json("{ not valid json");

        let result = MapTemplate::load(&path);

        assert!(matches!(result, Err(MapLoadError::InvalidJson(_))));

        std::fs::remove_file(&path).unwrap();
    }

    #[test]
    fn test_load_missing_file_fails() {
        let path = std::env::temp_dir().join("this_file_does_not_exist_12345.json");

        let result = MapTemplate::load(&path);

        assert!(matches!(result, Err(MapLoadError::InvalidJson(_))));
    }
}

mod into_park_map {
    use super::*;

    fn base_template() -> MapTemplate {
        MapTemplate {
            archetype: "test".into(),
            name: "Test".into(),
            dimensions: Dimensions { width: 2, height: 2, levels: vec![0] },
            default_terrain: "grass".into(),
            terrain: vec![],
            infrastructure: vec![InfrastructureEntry {
                x: 0, y: 0, z: 0,
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
            x: 1, y: 0, z: 0,
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
        assert_eq!(park_map.get_infrastructure(0, 0, 0), Some(&InfrastructureShape::Path));
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
        template.terrain.push(TerrainEntry { x: 1, y: 1, z: 0, material: "water".into() });

        let park_map = template.into_park_map().unwrap();

        assert_eq!(park_map.get_terrain(1, 1, 0), Some(&"water".to_string()));
    }

    #[test]
    fn test_out_of_bounds_terrain_entry_fails_explicitly() {
        let mut template = base_template();
        template.terrain.push(TerrainEntry { x: 99, y: 0, z: 0, material: "grass".into() });

        let result = template.into_park_map();

        assert!(matches!(result, Err(MapLoadError::OutOfBounds { x: 99, y: 0, z: 0 })));
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

        assert!(matches!(result, Err(MapLoadError::OutOfBounds { x: 99, y: 99, .. })));
    }

    #[test]
    fn test_unknown_infrastructure_kind_fails_explicitly() {
        let mut template = base_template();
        template.infrastructure.push(InfrastructureEntry {
            x: 1, y: 1, z: 0,
            kind: "teleporter".into(),
            to_z: None,
        });

        let result = template.into_park_map();

        assert!(matches!(result, Err(MapLoadError::UnknownInfrastructureKind(k)) if k == "teleporter"));
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
