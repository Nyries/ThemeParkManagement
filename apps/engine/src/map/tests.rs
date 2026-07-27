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

mod footprint_for {
    use crate::map::footprint_for;
 
    #[test]
    fn test_footprint_for_stub() {
        let template_id = "";
        let result = footprint_for(&template_id);
        assert_eq!(result,[(0,0), (0,1), (1,1)]);
    }
}

mod terrain {
    use super::*;

    #[test]
    fn test_set_and_get_terrain() {
        let mut park_map = build_test_map();

        assert!(park_map.get_terrain(0, 0, 0).is_none());

        park_map.set_terrain(0, 0, 0, "grass".into());

        let material = park_map.get_terrain(0, 0, 0).expect("The terrain should exist");
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

        let infrastructure_kind = park_map.get_infrastructure(0, 0, 0).expect("The infrastructure should exist");
        assert_eq!(infrastructure_kind, &InfrastructureShape::Path);
        
        park_map.remove_infrasture(0, 0, 0);
        assert!(park_map.get_infrastructure(0, 0, 0).is_none())

    }
}

mod buildings {
    use super::*;

    #[test]
    fn test_set_and_get_building() {
        let mut park_map = build_test_map();

        assert!(park_map.get_building(0, 0, 0).is_none());

        park_map.set_building(0, 0, 0, BuildingId { building_id: "coaster-1".into(), template_id: "b&m-1".into() });

        let building_id = park_map.get_building(0, 0, 0).expect("The building should exist");
        assert_eq!(building_id.building_id, "coaster-1");
    }

    #[test]
    fn test_place_building_and_get_building_coords_by_building_id() {
        let mut park_map = build_test_map();
        let origin = (0,0,0);
        let footprint = [(0,0), (0,1), (1,1)];
        let rotation = Rotation::Deg180;
        let building_id = BuildingId { building_id: "building-1".to_string(), template_id: "restaurant-1".to_string() };
        assert!(park_map.get_building(0, 0, 0).is_none());

        park_map.place_building(origin, &footprint, rotation, building_id);

        assert_eq!(park_map.get_building_coords_by_building_id("building-1"),[(0,0,0), (0, -1,0), (-1,-1,0)])
    }

    #[test]
    fn test_get_building_coords_by_building_id_that_doesnt_exist() {
        let park_map = build_test_map();

        let result = park_map.get_building_coords_by_building_id("non-existent");
        assert!(result.is_empty());
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
        park_map.set_building(0, 0, 0, BuildingId { building_id: "coaster-1".into(), template_id: "b&m-1".into() });

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
        let result = park_map.can_place_infrastructure(InfrastructureShape::Path, 0, &[(0, 0, 0)]);

        assert!(result.is_ok());
    }

    #[test]
    fn test_can_place_infrastructure_fails_out_of_bounds() {
        let park_map = build_infra_test_map();

        let result = park_map.can_place_infrastructure(InfrastructureShape::Path, 0, &[(5, 5, 0)]);

        assert_eq!(result, Err(ErrorCode::ErrorOutOfBounds));
    }

    #[test]
    fn test_can_place_infrastructure_fails_on_locked_parcel() {
        let park_map = ParkMap::new("map-1".into(), Bounds3d::new(0, 0, 0, 0, -1, 2)); // pas de parcelle

        let result = park_map.can_place_infrastructure(InfrastructureShape::Path, 0, &[(0, 0, 0)]);

        assert_eq!(result, Err(ErrorCode::ErrorOutOfBounds));
    }

    #[test]
    fn test_can_place_infrastructure_fails_on_building_collision() {
        let mut park_map = build_infra_test_map();
        park_map.set_building(0, 0, 0, BuildingId { building_id: "coaster-1".into(), template_id: "b&m-1".into() });

        let result = park_map.can_place_infrastructure(InfrastructureShape::Path, 0, &[(0, 0, 0)]);

        assert_eq!(result, Err(ErrorCode::ErrorCollision));
    }

    #[test]
    fn test_can_place_infrastructure_rejects_path_on_water_at_ground_level() {
        let mut park_map = build_infra_test_map();
        park_map.set_terrain(0, 0, 0, "water".into());

        let result = park_map.can_place_infrastructure(InfrastructureShape::Path, 0, &[(0, 0, 0)]);

        assert_eq!(result, Err(ErrorCode::ErrorCrossingNotAllowed));
    }

    #[test]
    fn test_can_place_infrastructure_allows_path_bridge_over_water() {
        let mut park_map = build_infra_test_map();
        park_map.set_terrain(0, 0, 0, "water".into());

        // Pont = Path à z=+1 au-dessus de l'eau à z=0
        let result = park_map.can_place_infrastructure(InfrastructureShape::Path, 0, &[(0, 0, 1)]);

        assert!(result.is_ok());
    }

    #[test]
    fn test_can_place_infrastructure_fails_crossing_not_allowed_above_building() {
        let mut park_map = build_infra_test_map();
        park_map.set_building(0, 0, 0, BuildingId { building_id: "shop-1".into(), template_id: "shop".into() });

        let result = park_map.can_place_infrastructure(InfrastructureShape::Path, 0, &[(0, 0, 1)]);

        assert_eq!(result, Err(ErrorCode::ErrorCrossingNotAllowed));
    }

    #[test]
    fn test_can_place_infrastructure_allows_passerelle_above_building_with_flag() {
        let mut park_map = build_infra_test_map();
        park_map.set_building(0, 0, 0, BuildingId { building_id: "support-1".into(), template_id: "bridge_support".into() });

        let result = park_map.can_place_infrastructure(InfrastructureShape::Path, 0, &[(0, 0, 1)]);

        assert!(result.is_ok());
    }

    #[test]
    fn test_can_place_infrastructure_succeeds_for_ramp_to_adjacent_level() {
        let park_map = build_infra_test_map();

        let result = park_map.can_place_infrastructure(InfrastructureShape::Ramp { to_z: 1 }, 1, &[(0, 0, 0)]);

        assert!(result.is_ok());
    }

    #[test]
    fn test_can_place_infrastructure_fails_for_ramp_to_non_adjacent_level() {
        let mut park_map = build_infra_test_map();
        park_map.unlocked_levels.insert(2);

        let result = park_map.can_place_infrastructure(InfrastructureShape::Ramp { to_z: 2 }, 2, &[(0, 0, 0)]);

        assert_eq!(result, Err(ErrorCode::ErrorCrossingNotAllowed));
    }

    #[test]
    fn test_can_place_infrastructure_fails_for_ramp_to_locked_level() {
        let park_map = build_infra_test_map();
        // to_z = -1 est dans les bornes mais pas dans unlocked_levels

        let result = park_map.can_place_infrastructure(InfrastructureShape::Stairs { to_z: -1 }, -1, &[(0, 0, 0)]);

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
        let footprint = vec![(0,0), (1,0), (1,1)];

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
        park_map.set_building(2, 2, 0, BuildingId { building_id: "existing".into(), template_id: "shop".into() });
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
        park_map.set_building(1, 2, 0, BuildingId { building_id: "existing".into(), template_id: "shop".into() });

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
        park_map.set_building(0, 0, 0, BuildingId { building_id: "coaster-1".into(), template_id: "b&m-giga".into() });

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
