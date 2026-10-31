use pathfinding::prelude::astar;

use crate::{
    balance::MOVEMENT_COST_PATH,
    map::{InfrastructureShape, movement_cost_for},
};

use super::ParkMap;

pub type PathResult = (Vec<(i32, i32, i32)>, u32);

impl ParkMap {
    fn is_walkable(&self, x: i32, y: i32, z: i32) -> bool {
        matches!(
            self.get_infrastructure(x, y, z),
            Some(InfrastructureShape::Path)
                | Some(InfrastructureShape::Ramp { .. })
                | Some(InfrastructureShape::Stairs { .. })
                | Some(InfrastructureShape::Queue { .. })
        )
    }

    fn successors(&self, &(x, y, z): &(i32, i32, i32)) -> Vec<((i32, i32, i32), u32)> {
        let mut result = Vec::new();

        for (dx, dy) in [(1, 0), (-1, 0), (0, 1), (0, -1)] {
            let (nx, ny, nz) = (x + dx, y + dy, z);

            if !self.is_within_bounds(nx, ny, nz) {
                continue;
            }
            if !self.is_walkable(nx, ny, nz) {
                continue;
            }

            let cost = self
                .get_infrastructure(nx, ny, nz)
                .map(movement_cost_for)
                .unwrap_or(MOVEMENT_COST_PATH);

            result.push(((nx, ny, nz), cost));
        }
        //TODO: add vertical successors (stairs, ramp)
        result
    }

    // Manhattan distance in a 2D grid
    fn heuristic(&(x, y, z): &(i32, i32, i32), destination: (i32, i32, i32)) -> u32 {
        ((x - destination.0).abs() + (y - destination.1).abs() + (z - destination.2).abs()) as u32
    }

    pub fn find_path(
        &self,
        start: (i32, i32, i32),
        destination: (i32, i32, i32),
    ) -> Option<PathResult> {
        let (path, cost) = astar(
            &start,
            |node| self.successors(node),
            |node| Self::heuristic(node, destination),
            |&node| node == destination,
        )?;

        Some((path, cost))
    }

    pub fn path_excluding_start(
        &self,
        start: (i32, i32, i32),
        destination: (i32, i32, i32),
    ) -> Vec<(i32, i32, i32)> {
        let mut path = self
            .find_path(start, destination)
            .map(|(p, _)| p)
            .unwrap_or_default();
        if !path.is_empty() {
            path.remove(0);
        }
        path
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use crate::map::Bounds3d;

    fn build_map() -> ParkMap {
        ParkMap::new("map-1".into(), Bounds3d::new(0, 5, 0, 5, -1, 1))
    }

    mod is_walkable {
        use super::*;
        use crate::map::{BuildingId, InfrastructureShape};

        #[test]
        fn test_a_queue_cell_is_walkable() {
            let mut map = build_map();
            map.set_infrastructure(
                0,
                0,
                0,
                InfrastructureShape::Queue {
                    attraction_id: BuildingId::default(),
                },
            );

            assert!(map.is_walkable(0, 0, 0));
        }
    }

    mod successors {
        use super::*;
        use crate::map::InfrastructureShape;

        #[test]
        fn test_successors_returns_walkable_infrastructure_neighbors_with_infrastructure_cost() {
            let mut map = build_map();

            // Cost must come from the infrastructure at each neighbor, never
            // the terrain underneath — TPM-138.
            map.set_infrastructure(1, 0, 0, InfrastructureShape::Path);
            map.set_infrastructure(0, 1, 0, InfrastructureShape::Stairs { to_z: 1 });

            let mut result = map.successors(&(0, 0, 0));
            result.sort();

            let mut expected = vec![
                ((1, 0, 0), 1), // Path -> movement_cost_for = 1
                ((0, 1, 0), 3), // Stairs -> movement_cost_for = 3
            ];
            expected.sort();

            assert_eq!(result, expected);
        }

        #[test]
        fn test_successors_excludes_cells_without_infrastructure() {
            let mut map = build_map();
            map.set_terrain(1, 0, 0, "grass".into());

            let result = map.successors(&(0, 0, 0));

            assert!(result.is_empty());
        }

        #[test]
        fn test_successors_excludes_out_of_bounds_neighbors() {
            let map = build_map();

            let result = map.successors(&(0, 0, 0));

            assert!(result.iter().all(|(coord, _)| *coord != (-1, 0, 0)));
        }
    }

    mod heuristic {
        use super::*;

        #[test]
        fn test_heuristic_computes_manhattan_distance_to_target() {
            let target = (5, 3, 1);

            let result = ParkMap::heuristic(&(0, 0, 0), target);

            assert_eq!(result, 5 + 3 + 1); // 9
        }

        #[test]
        fn test_heuristic_is_zero_when_already_at_target() {
            let target = (2, 2, 0);

            let result = ParkMap::heuristic(&target, target);

            assert_eq!(result, 0);
        }

        #[test]
        fn test_heuristic_handles_negative_deltas() {
            let target = (0, 0, -1);

            let result = ParkMap::heuristic(&(3, 4, 0), target);

            assert_eq!(result, 3 + 4 + 1); // 8
        }
    }

    mod find_path {
        use super::*;

        #[test]
        fn test_find_path_direct_path_between_connected_points() {
            let mut map = build_map();
            for x in 0..=3 {
                map.set_terrain(x, 0, 0, "path".into());
                map.set_infrastructure(x, 0, 0, InfrastructureShape::Path);
            }

            let result = map.find_path((0, 0, 0), (3, 0, 0));

            assert_eq!(
                result,
                Some((vec![(0, 0, 0), (1, 0, 0), (2, 0, 0), (3, 0, 0)], 3))
            );
        }

        #[test]
        fn test_find_path_bypasses_unwalkable_obstacle() {
            let mut map = build_map();

            // Straight line blocked : (1,0,0) and (2,0,0) don't have infrastructure
            map.set_infrastructure(0, 0, 0, InfrastructureShape::Path);
            map.set_infrastructure(3, 0, 0, InfrastructureShape::Path);

            // Detour to y=1
            for x in 0..=3 {
                map.set_infrastructure(x, 1, 0, InfrastructureShape::Path);
            }

            let result = map.find_path((0, 0, 0), (3, 0, 0));

            assert!(result.is_some());
            let (path, _cost) = result.unwrap();
            assert!(!path.contains(&(1, 0, 0)));
            assert!(!path.contains(&(2, 0, 0)));
            assert_eq!(path.first(), Some(&(0, 0, 0)));
            assert_eq!(path.last(), Some(&(3, 0, 0)));
        }

        #[test]
        fn test_find_path_returns_none_without_panic_when_target_is_unreachable() {
            let mut map = build_map();
            map.set_infrastructure(0, 0, 0, InfrastructureShape::Path);

            let result = map.find_path((0, 0, 0), (5, 5, 0));

            assert_eq!(result, None);
        }
    }
}
