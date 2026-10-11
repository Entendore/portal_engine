use crate::math_util::{ray_plane_intersect, portal_link_matrix, teleport_point, teleport_dir};
use crate::world::World;
use glam::Vec3;

pub struct BridgeSegment {
    pub start: Vec3,
    pub end: Vec3,
    pub tiles: Vec<Vec3>,
}

pub fn calculate_bridge_path(start: Vec3, dir: Vec3, world: &World) -> BridgeSegment {
    let mut current_pos = start;
    let mut current_dir = dir.normalize();
    let mut tiles = Vec::new();
    let mut closest_t = 30.0;
    let mut hit_portal = false;

    for wall in &world.walls {
        if let Some(t) = ray_plane_intersect(current_pos, current_dir, wall.center, wall.normal) {
            if t > 0.0 && t < closest_t {
                let hit_pos = current_pos + current_dir * t;
                let local = hit_pos - wall.center;
                let right = wall.normal.cross(wall.up).normalize();
                if wall.up.dot(local).abs() < wall.half_height && right.dot(local).abs() < wall.half_width {
                    closest_t = t;
                }
            }
        }
    }

    for p in &world.portals {
        if let Some(t) = ray_plane_intersect(current_pos, current_dir, p.transform.position, p.transform.forward()) {
            if t > 0.0 && t < closest_t {
                let hit_pos = current_pos + current_dir * t;
                let local = hit_pos - p.transform.position;
                if p.transform.up().dot(local).abs() < p.half_height && p.transform.right().dot(local).abs() < p.half_width {
                    if let Some(linked) = p.linked {
                        let target = &world.portals[linked];
                        let link = portal_link_matrix(&p.transform, p.scale, &target.transform, target.scale);
                        current_pos = teleport_point(hit_pos, link);
                        current_dir = teleport_dir(current_dir, link);
                        hit_portal = true;
                        break;
                    }
                }
            }
        }
    }

    let mut dist = 0.0;
    let mut tile_pos = start;
    while dist < closest_t {
        tiles.push(tile_pos);
        tile_pos += current_dir * 1.0;
        dist += 1.0;
    }
    
    if hit_portal {
        let mut rest = calculate_bridge_path(current_pos, current_dir, world);
        tiles.extend(rest.tiles);
        return BridgeSegment { start, end: rest.end, tiles };
    }

    BridgeSegment { start, end: start + current_dir * closest_t, tiles }
}
