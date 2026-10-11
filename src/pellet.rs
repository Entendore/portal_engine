use crate::math_util::{ray_plane_intersect, portal_link_matrix, teleport_point, teleport_dir};
use crate::world::World;
use glam::Vec3;

pub struct Pellet {
    pub pos: Vec3,
    pub vel: Vec3,
    pub life: f32,
    pub max_life: f32,
}

impl Pellet {
    pub fn new(pos: Vec3, dir: Vec3) -> Self {
        Self { pos, vel: dir * 15.0, life: 6.0, max_life: 6.0 }
    }

    pub fn update(&mut self, dt: f32, world: &World) -> bool {
        self.life -= dt;
        if self.life <= 0.0 { return false; }
        let prev_pos = self.pos;
        self.pos += self.vel * dt;

        for p in &world.portals {
            let n = p.transform.forward();
            let d_prev = (prev_pos - p.transform.position).dot(n);
            let d_new  = (self.pos - p.transform.position).dot(n);
            if d_prev > 0.0 && d_new <= 0.0 {
                let local = prev_pos - p.transform.position;
                if p.transform.up().dot(local).abs() < p.half_height && p.transform.right().dot(local).abs() < p.half_width {
                    if let Some(linked) = p.linked {
                        let link = portal_link_matrix(&p.transform, p.scale, &world.portals[linked].transform, world.portals[linked].scale);
                        self.pos = teleport_point(self.pos, link);
                        self.vel = teleport_dir(self.vel, link);
                    }
                }
            }
        }

        for wall in &world.walls {
            if let Some(t) = ray_plane_intersect(prev_pos, self.vel, wall.center, wall.normal) {
                if t > 0.0 && t < (self.pos - prev_pos).length() {
                    let hit_pos = prev_pos + self.vel * t;
                    let local = hit_pos - wall.center;
                    let right_dir = wall.normal.cross(wall.up).normalize();
                    if wall.up.dot(local).abs() < wall.half_height && right_dir.dot(local).abs() < wall.half_width {
                        self.pos = hit_pos + wall.normal * 0.1;
                        self.vel -= wall.normal * self.vel.dot(wall.normal) * 2.0;
                        return false;
                    }
                }
            }
        }
        false
    }
}
