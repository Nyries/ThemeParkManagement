use crate::map::{InfrastructureShape, ParkMap, movement_cost_for, vertical_movement_cost_for};


impl ParkMap {

    fn is_walkable(&self, x: i32, y: i32, z: i32) -> bool {
        matches!(
            self.get_infrastructure(x, y, z),
            Some(InfrastructureShape::Path)
                | Some(InfrastructureShape::Ramp { .. })
                | Some(InfrastructureShape::Stairs { .. })
        )
    }

    fn successors(&self, &(x, y, z): &(i32, i32, i32)) -> Vec<((i32, i32, i32), u32)> {
        let mut result = Vec::new();

        for (dx, dy) in [(1,0), (-1,0), (0,1), (0,-1)] {
            let (nx,ny,nz) = (x+dx, y+dy, z);

            if !self.is_within_bounds(nx, ny, nz) {
                continue;
            }
            if !self.is_walkable(nx, ny, nz) {
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

    
    fn bresenham_line (x0: i32, y0: i32, x1: i32, y1: i32) -> Vec<(i32, i32)> {
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
            if e2 >= dy { err += dy; x += sx; } 
            if e2 <= dx { err += dx; y += sy; }
        }
        points
    }
    
    fn has_line_of_sight(&self, origin: (i32,i32,i32), target: (i32,i32,i32)) -> bool {
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
        if path.is_empty() {return path;}
        
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
            simplified.push(path[path.len() -1]);
        }
        
        simplified        
    }
    
    pub fn find_path(&self, start: (i32, i32, i32), target: (i32, i32, i32)) -> Option<(Vec<(i32,i32,i32)>, u32)> {
        let (raw_path, cost) = pathfinding::prelude::astar(
            &start, 
            |node| self.successors(node), 
            |node| Self::heuristic(node, target), 
            |&node| node == target
        )?;

        Some((self.simplify_line_of_sight(raw_path), cost))
    }

}

#[cfg(test)]
mod tests;