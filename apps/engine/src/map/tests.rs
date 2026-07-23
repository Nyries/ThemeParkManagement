use super::*;

fn build_test_map() -> ParkMap {
    let map_id = "map-1".to_string();
    let bounds_3d = Bounds3d::new(0, 0, 0, 0, 0, 0);
    ParkMap::new(map_id, bounds_3d)
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

        park_map.set_buildings(0, 0, 0, BuildingId { building_id: "coaster-1".into(), template_id: "b&m-1".into() });

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
        park_map.set_buildings(0, 0, 0, BuildingId { building_id: "coaster-1".into(), template_id: "b&m-1".into() });

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
        let footprint = vec![(0, 0), (1, 0)];

        let result = park_map.can_place_building((2, 2, 0), &footprint, Rotation::Deg0, "coaster");

        assert!(result.is_ok());
    }

    #[test]
    fn test_can_place_building_fails_out_of_bounds() {
        let park_map = build_placement_test_map();
        let footprint = vec![(0, 0), (1, 0)]; 

        let result = park_map.can_place_building((5, 5, 0), &footprint, Rotation::Deg0, "coaster");

        assert_eq!(result, Err(ErrorCode::ErrorOutOfBounds));
    }

    #[test]
    fn test_can_place_building_fails_on_locked_parcel() {
        let park_map = ParkMap::new("map-1".into(), Bounds3d::new(0, 5, 0, 5, 0, 0)); // no parcel created
        let footprint = vec![(0, 0)];

        let result = park_map.can_place_building((2, 2, 0), &footprint, Rotation::Deg0, "coaster");

        assert_eq!(result, Err(ErrorCode::ErrorOutOfBounds));
    }

    #[test]
    fn test_can_place_building_fails_on_building_collision() {
        let mut park_map = build_placement_test_map();
        park_map.set_buildings(2, 2, 0, BuildingId { building_id: "existing".into(), template_id: "shop".into() });
        let footprint = vec![(0, 0)];

        let result = park_map.can_place_building((2, 2, 0), &footprint, Rotation::Deg0, "coaster");

        assert_eq!(result, Err(ErrorCode::ErrorCollision));
    }

    #[test]
    fn test_can_place_building_fails_on_water() {
        let mut park_map = build_placement_test_map();
        park_map.set_terrain(2, 2, 0, Material { material_id: "water".into() });
        let footprint = vec![(0, 0)];

        let result = park_map.can_place_building((2, 2, 0), &footprint, Rotation::Deg0, "coaster");

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

        let result = park_map.can_place_building((2, 2, 1), &footprint, Rotation::Deg0, "coaster");

        assert_eq!(result, Err(ErrorCode::ErrorOutOfBounds));
    }
}

mod can_remove {
    use super::*;

    #[test]
    fn test_can_remove_succeeds_when_something_exists() {
        let mut park_map = build_test_map();
        park_map.set_terrain(0, 0, 0, Material { material_id: "grass".into() });

        assert!(park_map.can_remove(0, 0, 0).is_ok());
    }

    #[test]
    fn test_can_remove_fails_on_empty_cell() {
        let park_map = build_test_map();

        let result = park_map.can_remove(0, 0, 0);

        assert_eq!(result, Err(ErrorCode::ErrorCollision));
    }

    #[test]
    fn test_can_remove_fails_out_of_bounds() {
        let park_map = build_test_map();

        let result = park_map.can_remove(5, 5, 0);

        assert_eq!(result, Err(ErrorCode::ErrorOutOfBounds));
    }
}
