use crate::map::{InfrastructureShape, ParkMap, movement_cost_for, vertical_movement_cost_for};


impl ParkMap {
    fn successors(&self, &(x, y, z): &(i32, i32, i32)) -> Vec<((i32, i32, i32), u32)> {
        let mut result = Vec::new();

        for (dx, dy) in [(1,0), (-1,0), (0,1), (0,-1)] {
            let (nx,ny,nz) = (x+dx, y+dy, z);

            if !self.is_within_bounds(nx, ny, nz) {
                continue;
            }
            let walkable = matches!(
                self.get_infrastructure(nx, ny, nz),
                Some(InfrastructureShape::Path) 
                    | Some(InfrastructureShape::Ramp { to_z: _ }) 
                    | Some(InfrastructureShape::Stairs { to_z: _ })
            );
            if !walkable {
                continue;
            }

            let cost = self.get_terrain(nx, ny, nz)
                .map(|material| movement_cost_for(material))
                .unwrap_or(5);

            result.push(((nx,ny,nz), cost));
        }

        if let Some(shape @ (InfrastructureShape::Ramp { to_z } | InfrastructureShape::Stairs { to_z })) =
            self.get_infrastructure(x, y, z) 
        {
            if self.is_within_bounds(x, y, *to_z) {
                result.push(((x,y,*to_z), vertical_movement_cost_for(shape)));
            }
        }
        result
    }

    fn heuristic(&(x,y,z): &(i32,i32,i32), target: (i32,i32,i32)) -> u32 {
        ((x- target.0).abs() + (y - target.1).abs() + (z - target.2).abs()) as u32

    }

    pub fn find_path(&self, start: (i32, i32, i32), target: (i32, i32, i32)) -> Option<(Vec<(i32,i32,i32)>, u32)> {
        pathfinding::prelude::astar(
            &start, 
            |node| self.successors(node), 
            |node| Self::heuristic(node, target), 
            |&node| node == target
        )
    }

}

#[cfg(test)]
mod tests;