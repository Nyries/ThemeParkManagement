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
            ((0, 1, 0), 5), // "grass" -> movement_cost_for = 3
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