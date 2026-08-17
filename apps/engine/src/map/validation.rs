use crate::{building_template::BuildingCatalog, simulation::ErrorCode, simulation::Rotation};

use super::{InfrastructureShape, ParkMap, rotate_footprint};

/// TPM-35 placement rules — one `impl ParkMap` block per file, split from the
/// accessors/mutators in `map/mod.rs`.
impl ParkMap {
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
            if matches!(
                kind,
                InfrastructureShape::Ramp { .. } | InfrastructureShape::Stairs { .. }
            ) {
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
    use crate::map::{Bounds3d, BuildingId, Parcel};

    fn build_test_map() -> ParkMap {
        let map_id = "map-1".to_string();
        let bounds_3d = Bounds3d::new(0, 0, 0, 0, 0, 0);
        ParkMap::new(map_id, bounds_3d)
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
        fn test_can_place_infrastructure_succeeds_for_queue_without_requiring_a_to_z_adjacency() {
            // Regression: Queue is horizontal like Path, unlike Ramp/Stairs — it must
            // not be rejected by the vertical-transition check just for not being Path.
            let park_map = build_infra_test_map();

            let result = park_map.can_place_infrastructure(
                &test_catalog(),
                InfrastructureShape::Queue {
                    attraction_id: BuildingId::default(),
                },
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
