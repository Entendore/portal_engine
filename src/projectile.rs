use crate::math_util::{ray_plane_intersect, portal_link_matrix, teleport_point, teleport_dir};
use crate::world::World;
use glam::Vec3;

pub struct Projectile {
    pub prev_pos: Vec3,
    pub pos: Vec3,
    pub vel: Vec3,
    pub is_blue: bool,
    pub alive: bool,
    pub lifetime: f32,
}

impl Projectile {
    pub fn new(start: Vec3, dir: Vec3, is_blue: bool) -> Self {
        Self { prev_pos: start, pos: start, vel: dir * 40.0, is_blue, alive: true, lifetime: 2.0 }
    }

    pub fn update(&mut self, dt: f32, world: &World) -> bool {
        if !self.alive { return false; }
        self.lifetime -= dt;
        if self.lifetime <= 0.0 { self.alive = false; return false; }

        self.prev_pos = self.pos;
        self.pos += self.vel * dt;

        for p in &world.portals {
            let n = p.transform.forward();
            let d_prev = (self.prev_pos - p.transform.position).dot(n);
            let d_new  = (self.pos - p.transform.position).dot(n);
            if d_prev > 0.0 && d_new <= 0.0 {
                let local = self.prev_pos - p.transform.position;
                if p.transform.up().dot(local).abs() < p.half_height && p.transform.right().dot(local).abs() < p.half_width {
                    if let Some(linked) = p.linked {
                        let link = portal_link_matrix(&p.transform, p.scale, &world.portals[linked].transform, world.portals[linked].scale);
                        self.pos = teleport_point(self.pos, link);
                        self.vel = teleport_dir(self.vel, link);
                        return false;
                    }
                }
            }
        }

        for wall in &world.walls {
            if !wall.portalable { continue; }
            if let Some(t) = ray_plane_intersect(self.prev_pos, self.vel, wall.center, wall.normal) {
                let hit_pos = self.prev_pos + self.vel * t;
                let local = hit_pos - wall.center;
                let right_dir = wall.normal.cross(wall.up).normalize();
                if wall.up.dot(local).abs() < wall.half_height && right_dir.dot(local).abs() < wall.half_width {
                    self.alive = false;
                    return true;
                }
            }
        }
        false
    }
}
