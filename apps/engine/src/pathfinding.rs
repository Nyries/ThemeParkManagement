use crate::map::{InfrastructureShape, ParkMap, movement_cost_for, vertical_movement_cost_for};

pub type PathResult = (Vec<(i32, i32, i32)>, u32);

impl ParkMap {
    pub(crate) fn is_walkable(&self, x: i32, y: i32, z: i32) -> bool {
        matches!(
            self.get_infrastructure(x, y, z),
            Some(InfrastructureShape::Path)
                | Some(InfrastructureShape::Ramp { .. })
                | Some(InfrastructureShape::Stairs { .. })
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
                .get_terrain(nx, ny, nz)
                .map(|material| movement_cost_for(material))
                .unwrap_or(5);

            result.push(((nx, ny, nz), cost));
        }

        if let Some(
            shape @ (InfrastructureShape::Ramp { to_z } | InfrastructureShape::Stairs { to_z }),
        ) = self.get_infrastructure(x, y, z)
            && self.is_within_bounds(x, y, *to_z)
        {
            result.push(((x, y, *to_z), vertical_movement_cost_for(shape)));
        }
        result
    }

    fn heuristic(&(x, y, z): &(i32, i32, i32), target: (i32, i32, i32)) -> u32 {
        ((x - target.0).abs() + (y - target.1).abs() + (z - target.2).abs()) as u32
    }

    fn bresenham_line(x0: i32, y0: i32, x1: i32, y1: i32) -> Vec<(i32, i32)> {
        let mut points = Vec::new();
        let (dx, dy) = ((x1 - x0).abs(), -(y1 - y0).abs());
        let (sx, sy) = ((x1 - x0).signum(), (y1 - y0).signum());
        let (mut x, mut y) = (x0, y0);
        let mut err = dx + dy;
        loop {
            points.push((x, y));
            if (x, y) == (x1, y1) {
                break;
            }
            let e2 = 2 * err;
            if e2 >= dy {
                err += dy;
                x += sx;
            }
            if e2 <= dx {
                err += dx;
                y += sy;
            }
        }
        points
    }

    fn has_line_of_sight(&self, origin: (i32, i32, i32), target: (i32, i32, i32)) -> bool {
        let (x0, y0, z0) = origin;
        let (x1, y1, z1) = target;
        if z0 != z1 {
            return false;
        }

        Self::bresenham_line(x0, y0, x1, y1)
            .into_iter()
            .all(|(x, y)| self.is_walkable(x, y, z0))
    }

    fn simplify_line_of_sight(&self, path: Vec<(i32, i32, i32)>) -> Vec<(i32, i32, i32)> {
        if path.is_empty() {
            return path;
        }

        let mut anchor = path[0];
        let mut simplified = vec![anchor];

        for i in 1..path.len() {
            if !self.has_line_of_sight(anchor, path[i]) {
                anchor = path[i - 1];
                if simplified.last() != Some(&anchor) {
                    simplified.push(anchor);
                }
            }
        }

        if simplified.last() != Some(&path[path.len() - 1]) {
            simplified.push(path[path.len() - 1]);
        }

        simplified
    }

    pub fn find_path(&self, start: (i32, i32, i32), target: (i32, i32, i32)) -> Option<PathResult> {
        let (raw_path, cost) = pathfinding::prelude::astar(
            &start,
            |node| self.successors(node),
            |node| Self::heuristic(node, target),
            |&node| node == target,
        )?;

        Some((self.simplify_line_of_sight(raw_path), cost))
    }

    pub fn path_excluding_start(
        &self,
        from: (i32, i32, i32),
        to: (i32, i32, i32),
    ) -> Vec<(i32, i32, i32)> {
        let mut path = self.find_path(from, to).map(|(p, _)| p).unwrap_or_default();
        if !path.is_empty() {
            path.remove(0);
        }
        path
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::map::{Bounds3d, ParkMap};

    fn build_map() -> ParkMap {
        ParkMap::new("map-1".into(), Bounds3d::new(0, 5, 0, 5, -1, 1))
    }

    mod successors {
        use super::*;
        use crate::map::InfrastructureShape;

        #[test]
        fn test_successors_returns_walkable_infrastructure_neighbors_with_terrain_cost() {
            let mut map = build_map();

            map.set_terrain(1, 0, 0, "path".into());
            map.set_infrastructure(1, 0, 0, InfrastructureShape::Path);

            map.set_terrain(0, 1, 0, "grass".into());
            map.set_infrastructure(0, 1, 0, InfrastructureShape::Path);

            let mut result = map.successors(&(0, 0, 0));
            result.sort();

            let mut expected = vec![
                ((1, 0, 0), 1), // "path" -> movement_cost_for = 1
                ((0, 1, 0), 5), // "grass" -> movement_cost_for = 5
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

            assert_eq!(result, Some((vec![(0, 0, 0), (3, 0, 0)], 3)));
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
        fn test_find_path_forced_ramp_to_reach_upper_level() {
            let mut map = build_map();
            map.set_infrastructure(0, 0, 0, InfrastructureShape::Ramp { to_z: 1 });
            map.set_infrastructure(0, 0, 1, InfrastructureShape::Path);

            let result = map.find_path((0, 0, 0), (0, 0, 1));

            assert_eq!(result, Some((vec![(0, 0, 0), (0, 0, 1)], 1)));
        }

        #[test]
        fn test_find_path_returns_none_without_panic_when_target_is_unreachable() {
            let mut map = build_map();
            map.set_infrastructure(0, 0, 0, InfrastructureShape::Path);

            let result = map.find_path((0, 0, 0), (5, 5, 0));

            assert_eq!(result, None);
        }
    }

    mod bresenham_line {
        use super::*;

        #[test]
        fn test_bresenham_line() {
            let (x0, y0, x1, y1) = (0, 0, 4, 1);

            let result = ParkMap::bresenham_line(x0, y0, x1, y1);

            assert_eq!(result, [(0, 0), (1, 0), (2, 1), (3, 1), (4, 1)]);
        }
    }

    mod has_line_of_sight {
        use super::*;

        #[test]
        fn test_true_when_all_cells_on_the_line_are_walkable() {
            let mut map = build_map();
            for x in 0..=3 {
                map.set_infrastructure(x, 0, 0, InfrastructureShape::Path);
            }

            assert!(map.has_line_of_sight((0, 0, 0), (3, 0, 0)));
        }

        #[test]
        fn test_false_when_a_cell_on_the_line_is_not_walkable() {
            let mut map = build_map();
            map.set_infrastructure(0, 0, 0, InfrastructureShape::Path);
            // (1,0,0) volontairement laissée sans infrastructure
            map.set_infrastructure(2, 0, 0, InfrastructureShape::Path);
            map.set_infrastructure(3, 0, 0, InfrastructureShape::Path);

            assert!(!map.has_line_of_sight((0, 0, 0), (3, 0, 0)));
        }

        #[test]
        fn test_false_when_points_are_on_different_levels() {
            let mut map = build_map();
            map.set_infrastructure(0, 0, 0, InfrastructureShape::Path);
            map.set_infrastructure(0, 0, 1, InfrastructureShape::Path);

            assert!(!map.has_line_of_sight((0, 0, 0), (0, 0, 1)));
        }

        #[test]
        fn test_true_when_origin_equals_target() {
            let mut map = build_map();
            map.set_infrastructure(2, 2, 0, InfrastructureShape::Path);

            assert!(map.has_line_of_sight((2, 2, 0), (2, 2, 0)));
        }
    }

    mod simplify_line_of_sight {
        use super::*;

        #[test]
        fn test_removes_useless_intermediate_points_on_a_straight_corridor() {
            let mut map = build_map();
            for x in 0..=3 {
                map.set_infrastructure(x, 0, 0, InfrastructureShape::Path);
            }
            let raw_path = vec![(0, 0, 0), (1, 0, 0), (2, 0, 0), (3, 0, 0)];

            let result = map.simplify_line_of_sight(raw_path);

            assert_eq!(result, vec![(0, 0, 0), (3, 0, 0)]);
        }

        #[test]
        fn test_keeps_the_corner_point_on_an_l_shaped_path() {
            let mut map = build_map();
            // Chemin en L : (0,0,0) -> (2,0,0) -> (2,2,0)
            for x in 0..=2 {
                map.set_infrastructure(x, 0, 0, InfrastructureShape::Path);
            }
            for y in 0..=2 {
                map.set_infrastructure(2, y, 0, InfrastructureShape::Path);
            }
            let raw_path = vec![(0, 0, 0), (1, 0, 0), (2, 0, 0), (2, 1, 0), (2, 2, 0)];

            let result = map.simplify_line_of_sight(raw_path);

            assert_eq!(result, vec![(0, 0, 0), (2, 0, 0), (2, 2, 0)]);
        }

        #[test]
        fn test_never_cuts_through_a_blocked_cell() {
            let mut map = build_map();
            // U turn : (1,0,0)/(2,0,0), are not walkable here.
            map.set_infrastructure(0, 0, 0, InfrastructureShape::Path);
            map.set_infrastructure(3, 0, 0, InfrastructureShape::Path);
            for x in 0..=3 {
                map.set_infrastructure(x, 1, 0, InfrastructureShape::Path);
            }
            let raw_path = vec![
                (0, 0, 0),
                (0, 1, 0),
                (1, 1, 0),
                (2, 1, 0),
                (3, 1, 0),
                (3, 0, 0),
            ];

            let result = map.simplify_line_of_sight(raw_path);

            assert!(!result.contains(&(1, 0, 0)));
            assert!(!result.contains(&(2, 0, 0)));
            assert_eq!(result.first(), Some(&(0, 0, 0)));
            assert_eq!(result.last(), Some(&(3, 0, 0)));
        }

        #[test]
        fn test_empty_path_returns_empty() {
            let map = build_map();

            let result = map.simplify_line_of_sight(vec![]);

            assert!(result.is_empty());
        }

        #[test]
        fn test_single_point_path_returns_same_point() {
            let map = build_map();

            let result = map.simplify_line_of_sight(vec![(1, 1, 0)]);

            assert_eq!(result, vec![(1, 1, 0)]);
        }
    }
}
