use crate::math_util::ray_plane_intersect;
use crate::world::World;
use crate::cube::Cube;
use glam::Vec3;

pub struct LaserSegment {
    pub start: Vec3,
    pub end: Vec3,
}

pub fn calculate_laser_path(start: Vec3, dir: Vec3, world: &World, cube: &Cube, max_bounces: u32) -> Vec<LaserSegment> {
    let mut segments = Vec::new();
    let mut current_pos = start;
    let mut current_dir = dir.normalize();
    let mut bounces = 0;

    segments.push(LaserSegment { start: current_pos, end: current_pos });

    while bounces <= max_bounces {
        let mut closest_t = 100.0;
        let mut hit_type = 0;

        for wall in &world.walls {
            if let Some(t) = ray_plane_intersect(current_pos, current_dir, wall.center, wall.normal) {
                if t > 0.0 && t < closest_t {
                    let hit_pos = current_pos + current_dir * t;
                    let local = hit_pos - wall.center;
                    let right = wall.normal.cross(wall.up).normalize();
                    if wall.up.dot(local).abs() < wall.half_height && right.dot(local).abs() < wall.half_width {
                        closest_t = t; hit_type = 1;
                    }
                }
            }
        }

        for c in &world.laser_catchers {
            if let Some(t) = ray_plane_intersect(current_pos, current_dir, c.pos, c.normal) {
                if t > 0.0 && t < closest_t {
                    let hit_pos = current_pos + current_dir * t;
                    let local = hit_pos - c.pos;
                    let right = c.normal.cross(c.up).normalize();
                    if c.up.dot(local).abs() < c.half_h && right.dot(local).abs() < c.half_w {
                        closest_t = t; hit_type = 2;
                    }
                }
            }
        }

        if !cube.grabbed {
            let to_cube = cube.position - current_pos;
            let proj = to_cube.dot(current_dir);
            if proj > 0.0 && proj < closest_t {
                let closest = current_pos + current_dir * proj;
                if (closest - cube.position).length() < cube.half_size {
                    closest_t = proj; hit_type = 3;
                }
            }
        }

        let hit_pos = current_pos + current_dir * closest_t;
        segments.last_mut().unwrap().end = hit_pos;

        if hit_type == 1 || hit_type == 2 { break; }
        if hit_type == 3 {
            let normal = (hit_pos - cube.position).normalize();
            current_dir = current_dir - normal * 2.0 * current_dir.dot(normal);
            current_pos = hit_pos + normal * 0.1;
            segments.push(LaserSegment { start: current_pos, end: current_pos });
            bounces += 1;
        } else { break; }
    }
    segments
}
