use crate::balance::{AVOIDING_RADIUS, DENSITY_CAP, K_REPULSION};

pub type VisitorId = String;

pub struct Visitor {
    pub id: VisitorId,
    pub position: (f32, f32, f32),
    pub path: Vec<(i32, i32, i32)>,
    pub target: (i32, i32, i32),
}

fn distance(a: (f32, f32, f32), b: (f32, f32, f32)) -> f32 {
    let dx = b.0 - a.0;
    let dy = b.1 - a.1;
    let dz = b.2 - a.2;
    (dx * dx + dy * dy + dz * dz).sqrt()
}

fn direction(from: (f32, f32, f32), to: (f32, f32, f32)) -> (f32, f32, f32) {
    let d = distance(from, to);
    if d == 0.0 {
        return (0.0, 0.0, 0.0);
    }
    ((to.0 - from.0) / d, (to.1 - from.1) / d, (to.2 - from.2) / d)
}

pub fn speed_at(base_speed: f32, density: usize) -> f32 {
    let f = (1.0 - density as f32 / DENSITY_CAP as f32).max(0.0);
    base_speed * f
}

pub fn repulsion_force(my_position: (f32, f32, f32), neighbor_position: (f32, f32, f32)) -> (f32, f32, f32) {
    let d = distance(my_position, neighbor_position);
    if d >= AVOIDING_RADIUS {
        return (0.0, 0.0, 0.0);
    }
    let (dx, dy, dz) = direction(neighbor_position, my_position);
    let intensity = K_REPULSION * (AVOIDING_RADIUS - d) / AVOIDING_RADIUS;
    (intensity * dx, intensity * dy, intensity * dz)
}


#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_distance_from_two_points() {
        let a = (0.0, 0.0, 0.0);
        let b = (3.0, 2.0, 2.0);
        let expected = (3.0_f32 * 3.0 + 2.0 * 2.0 + 2.0 * 2.0).sqrt(); // = 17.0_f32.sqrt() ≈ 4.1231

        let result = distance(a, b);

        assert_eq!(result, expected);
    }

    #[test]
    fn test_direction_when_same_point() {
        let a = (3.0, 2.0, 2.0);

        let result = direction(a, a);

        assert_eq!(result, (0.0, 0.0, 0.0))
    }

    #[test]
    fn test_direction_when_different_point() {
        let a = (0.0, 0.0, 0.0);
        let b = (3.0, 2.0, 2.0);
        let d = 17.0_f32.sqrt();
        let expected = (3.0 / d, 2.0 / d, 2.0 / d);

        let result = direction(a, b);

        assert_eq!(result, expected)
    }

    #[test]
    fn test_speed_at() {
        let base_speed = 1.0;
        let density = 2.0;
        let expected = 0.6; // (1.0 - 2.0/5.0).max(0.0) * 1.0 = 0.6 * 1.0

        let result = speed_at(base_speed, density as usize);

        assert_eq!(result, expected);
    }

    fn assert_close(actual: (f32, f32, f32), expected: (f32, f32, f32)) {
        const EPSILON: f32 = 1e-5;
        assert!((actual.0 - expected.0).abs() < EPSILON, "x: {} != {}", actual.0, expected.0);
        assert!((actual.1 - expected.1).abs() < EPSILON, "y: {} != {}", actual.1, expected.1);
        assert!((actual.2 - expected.2).abs() < EPSILON, "z: {} != {}", actual.2, expected.2);
    }

    #[test]
    fn test_repulsion_force_is_zero_beyond_avoiding_radius() {
        let force = repulsion_force((0.0, 0.0, 0.0), (1.0, 0.0, 0.0));
        assert_close(force, (0.0, 0.0, 0.0));
    }

    #[test]
    fn test_repulsion_force_is_zero_exactly_at_avoiding_radius() {
        let force = repulsion_force((0.0, 0.0, 0.0), (0.2, 0.0, 0.0));
        assert_close(force, (0.0, 0.0, 0.0));
    }

    #[test]
    fn test_repulsion_force_pushes_away_from_close_neighbor() {
        let force = repulsion_force((0.0, 0.0, 0.0), (0.1, 0.0, 0.0));
        assert_close(force, (-0.1, 0.0, 0.0));
    }

    #[test]
    fn test_repulsion_force_is_zero_when_visitors_overlap_exactly() {
        let force = repulsion_force((0.0, 0.0, 0.0), (0.0, 0.0, 0.0));
        assert_close(force, (0.0, 0.0, 0.0));
    }
}