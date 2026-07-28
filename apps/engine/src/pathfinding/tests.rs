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

        // Ligne droite bloquée : (1,0,0) et (2,0,0) n'ont pas d'infrastructure
        // (équivalent à de l'eau : aucune infra n'y a jamais pu être posée).
        map.set_infrastructure(0, 0, 0, InfrastructureShape::Path);
        map.set_infrastructure(3, 0, 0, InfrastructureShape::Path);

        // Détour par y=1
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
        // (5,5,0) reste isolé : aucune infrastructure ne mène jusque-là

        let result = map.find_path((0, 0, 0), (5, 5, 0));

        assert_eq!(result, None);
    }
}