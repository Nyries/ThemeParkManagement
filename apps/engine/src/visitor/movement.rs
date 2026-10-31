use crate::{balance::STEERING_FACTOR, map::ParkMap, visitor::Visitor};
use rand::seq::SliceRandom;

pub(super) fn distance(a: (f32, f32, f32), b: (f32, f32, f32)) -> f32 {
    let dx = b.0 - a.0;
    let dy = b.1 - a.1;
    let dz = b.2 - a.2;
    (dx * dx + dy * dy + dz * dz).sqrt()
}

pub(crate) fn direction(from: (f32, f32, f32), to: (f32, f32, f32)) -> (f32, f32, f32) {
    let d = distance(from, to);
    if d == 0.0 {
        return (0.0, 0.0, 0.0);
    }
    (
        (to.0 - from.0) / d,
        (to.1 - from.1) / d,
        (to.2 - from.2) / d,
    )
}

pub(super) fn normalize(v: (f32, f32, f32)) -> (f32, f32, f32) {
    let len = (v.0 * v.0 + v.1 * v.1 + v.2 * v.2).sqrt();
    if len == 0.0 {
        return (0.0, 0.0, 0.0);
    }
    (v.0 / len, v.1 / len, v.2 / len)
}

// fn dot(a: (f32, f32, f32), b: (f32, f32, f32)) -> f32 {
//     a.0 * b.0 + a.1 * b.1 + a.2 * b.2
// }

pub(super) fn lerp_direction(
    current: (f32, f32, f32),
    desired: (f32, f32, f32),
    factor: f32,
) -> (f32, f32, f32) {
    let blended = (
        current.0 + (desired.0 - current.0) * factor,
        current.1 + (desired.1 - current.1) * factor,
        current.2 + (desired.2 - current.2) * factor,
    );
    normalize(blended)
}

pub fn pick_random_walking_destination(park_map: &ParkMap) -> Option<(i32, i32, i32)> {
    let mut rng = rand::thread_rng();
    let keys: Vec<&(i32, i32, i32)> = park_map.infrastructure.keys().collect();
    keys.choose(&mut rng).copied().copied()
}

impl Visitor {
    pub fn advance(&mut self, speed: f32, dt: f32) {
        let Some(&next) = self.path.first() else {
            return;
        };
        let next_f = (next.0 as f32, next.1 as f32, next.2 as f32);
        let distance_to_next = distance(self.position, next_f);

        let desired = direction(self.position, next_f);
        self.heading = if self.heading == (0.0, 0.0, 0.0) {
            desired
        } else {
            lerp_direction(self.heading, desired, STEERING_FACTOR)
        };
        let step = speed * dt;

        if step >= distance_to_next {
            self.path.remove(0);
            self.position = next_f;
        } else {
            self.position.0 += self.heading.0 * step;
            self.position.1 += self.heading.1 * step;
            self.position.2 += self.heading.2 * step;
        }
    }

    pub fn assign_new_destination(&mut self, park_map: &ParkMap, old_cell: (i32, i32, i32)) {
        if !self.path.is_empty() || self.is_leaving {
            return;
        }
        let new_destination = pick_random_walking_destination(park_map).unwrap_or(old_cell);
        self.destination = new_destination;
        self.path = park_map.path_excluding_start(old_cell, new_destination);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::map::Bounds3d;

    fn build_map() -> ParkMap {
        ParkMap::new("map-1".into(), Bounds3d::new(0, 5, 0, 5, -1, 1))
    }

    mod pick_random_destination {
        use super::*;

        #[test]
        fn test_pick_random_destination_success() {
            let mut park_map = build_map();
            park_map
                .infrastructure
                .insert((0, 0, 0), crate::map::InfrastructureShape::Path);
            park_map
                .infrastructure
                .insert((1, 0, 0), crate::map::InfrastructureShape::Path);
            park_map
                .infrastructure
                .insert((2, 0, 0), crate::map::InfrastructureShape::Path);

            let result = pick_random_walking_destination(&park_map);

            assert!(result.is_some());
            let destination = result.unwrap();
            assert!(park_map.infrastructure.contains_key(&destination));
        }

        #[test]
        fn test_pick_random_destination_empty_map() {
            let park_map = &build_map();
            let result = pick_random_walking_destination(park_map);
            assert!(result.is_none())
        }
    }

    mod advance {
        use super::*;

        fn build_visitor() -> Visitor {
            Visitor {
                id: "v1".into(),
                position: (0.0, 0.0, 0.0),
                path: vec![(1,0,0), (2, 0, 0)],
                destination: (2, 0, 0),
                ticks_since_spawn: 0,
                heading: (0.0, 0.0, 0.0),
                is_leaving: false,
                ..Default::default()
            }
        }
        #[test]
        fn test_advance_empty_path() {
            let mut visitor = build_visitor();
            visitor.path = vec![];
            visitor.advance(5.0, 1.0);

            assert_eq!(visitor.position, (0.0, 0.0, 0.0));
        }

        #[test]
        fn test_advance_partial_step() {
            let mut visitor = build_visitor();

            // Vitesse de 1.0, dt de 1.0 -> step = 1.0 (inférieur à la distance de 10.0)
            visitor.advance(1.0, 1.0);

            assert_eq!(visitor.path.len(), 1);
            assert_eq!(visitor.path[0], (2, 0, 0));

            // Le visiteur doit avoir avancé partiellement sur l'axe X
            assert_eq!(visitor.position.0, 1.0);
        }

        #[test]
        fn test_advance_reaches_waypoint() {
            let mut visitor = build_visitor();
            visitor.position = (1.0, 0.0, 0.0);
            visitor.path = vec![(2, 0, 0)];

            visitor.advance(4.0, 1.0);

            assert!(visitor.path.is_empty());

            assert_eq!(visitor.position, (2.0, 0.0, 0.0));
        }
    }
}
