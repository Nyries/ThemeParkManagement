use super::*;

fn build_test_map() -> ParkMap {
    let map_id = "map-1".to_string();
    let bounds_3d = Bounds3d::new(0, 0, 0, 0, 0, 0);
    ParkMap::new(map_id, bounds_3d)
}

mod terrain {
    use super::*;

    #[test]
    fn test_set_and_get_terrain() {
        let mut park_map = build_test_map();

        assert!(park_map.get_terrain(0, 0, 0).is_none());

        park_map.set_terrain(0, 0, 0, Material { material_id: "grass".into() });

        let material = park_map.get_terrain(0, 0, 0).expect("The terrain should exist");
        assert_eq!(material.material_id, "grass");
    }
}

mod infrastructure {
    use super::*;

    #[test]
    fn test_set_and_get_infrastructure() {
        let mut park_map = build_test_map();

        assert!(park_map.get_infrastructure(0, 0, 0).is_none());

        park_map.set_infrastructure(0, 0, 0, InfrastructureKind::Path);

        let infrastructure_kind = park_map.get_infrastructure(0, 0, 0).expect("The infrastructure should exist");
        assert_eq!(infrastructure_kind, &InfrastructureKind::Path);
    }
}

mod buildings {
    use super::*;

    #[test]
    fn test_set_and_get_buildings() {
        let mut park_map = build_test_map();

        assert!(park_map.get_buildings(0, 0, 0).is_none());

        park_map.set_buildings(0, 0, 0, BuildingId { building_id: "coaster-1".into() });

        let building_id = park_map.get_buildings(0, 0, 0).expect("The building should exist");
        assert_eq!(building_id.building_id, "coaster-1");
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

mod can_apply_brush {
    use super::*;

    #[test]
    fn test_can_apply_brush_succeeds_on_unlocked_cell() {
        let mut park_map = build_test_map();
        park_map.parcels.push(Parcel {
            id: "start".into(),
            cells: vec![(0, 0)],
            unlocked: true,
            price: 0,
        });

        let result = park_map.can_apply_brush(0, 0, 0, Layer::Terrain);

        assert!(result.is_ok());
    }

    #[test]
    fn test_can_apply_brush_fails_out_of_bounds() {
        let park_map = build_test_map();

        let result = park_map.can_apply_brush(5, 5, 0, Layer::Terrain);

        assert_eq!(result, Err(ErrorCode::ErrorOutOfBounds));
    }

    #[test]
    fn test_can_apply_brush_fails_on_locked_parcel() {
        let park_map = build_test_map();

        let result = park_map.can_apply_brush(0, 0, 0, Layer::Terrain);

        assert_eq!(result, Err(ErrorCode::ErrorOutOfBounds));
    }

    #[test]
    fn test_can_apply_brush_fails_on_building_collision() {
        let mut park_map = build_test_map();
        park_map.parcels.push(Parcel {
            id: "start".into(),
            cells: vec![(0, 0)],
            unlocked: true,
            price: 0,
        });
        park_map.set_buildings(0, 0, 0, BuildingId { building_id: "coaster-1".into() });

        let result = park_map.can_apply_brush(0, 0, 0, Layer::Infrastructure);

        assert_eq!(result, Err(ErrorCode::ErrorCollision));
    }

    #[test]
    fn test_can_apply_brush_rejects_path_on_water_at_ground_level() {
        let mut park_map = build_test_map();
        park_map.parcels.push(Parcel {
            id: "start".into(),
            cells: vec![(0, 0)],
            unlocked: true,
            price: 0,
        });
        park_map.set_terrain(0, 0, 0, Material { material_id: "water".into() });

        let result = park_map.can_apply_brush(0, 0, 0, Layer::Infrastructure);

        assert_eq!(result, Err(ErrorCode::ErrorCrossingNotAllowed));
    }

    #[test]
    fn test_can_apply_brush_allows_bridge_over_water() {
        let mut park_map = ParkMap::new("map-1".into(), Bounds3d::new(0, 0, 0, 0, 0, 1));
        park_map.unlocked_levels.insert(1);
        park_map.parcels.push(Parcel {
            id: "start".into(),
            cells: vec![(0, 0)],
            unlocked: true,
            price: 0,
        });
        park_map.set_terrain(0, 0, 0, Material { material_id: "water".into() });

        let result = park_map.can_apply_brush(0, 0, 1, Layer::Infrastructure);

        assert!(result.is_ok());
    }
}
