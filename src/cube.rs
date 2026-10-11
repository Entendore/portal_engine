use crate::math_util::{portal_link_matrix, teleport_point, teleport_dir};
use crate::world::World;
use crate::gel::GelType;
use glam::Vec3;

pub struct Cube {
    pub position: Vec3,
    pub velocity: Vec3,
    pub half_size: f32,
    pub grabbed: bool,
    pub base_half_size: f32,
    pub teleported: bool,
    pub dead: bool,
}

impl Cube {
    pub fn new(pos: Vec3) -> Self {
        Self { position: pos, velocity: Vec3::ZERO, half_size: 0.4, grabbed: false, base_half_size: 0.4, teleported: false, dead: false }
    }

    pub fn update(&mut self, dt: f32, world: &World) {
        if self.grabbed || self.dead { return; }
        self.teleported = false;
        self.velocity.y -= 9.81 * dt;
        let mut new_pos = self.position + self.velocity * dt;

        for f in &world.fizzlers {
            let min = f.pos - f.half_size;
            let max = f.pos + f.half_size;
            if new_pos.x > min.x - self.half_size && new_pos.x < max.x + self.half_size &&
               new_pos.y > min.y - self.half_size && new_pos.y < max.y + self.half_size &&
               new_pos.z > min.z - self.half_size && new_pos.z < max.z + self.half_size {
                self.dead = true;
            }
        }

        for b in &world.breakables {
            if b.broken { continue; }
            let min = b.pos - b.half_size;
            let max = b.pos + b.half_size;
            if new_pos.x > min.x - self.half_size && new_pos.x < max.x + self.half_size &&
               new_pos.y > min.y - self.half_size && new_pos.y < max.y + self.half_size &&
               new_pos.z > min.z - self.half_size && new_pos.z < max.z + self.half_size {
                let speed = self.velocity.length();
                if speed > 10.0 {
                    b.hp -= speed;
                    if b.hp <= 0.0 { b.broken = true; } 
                    else {
                        let normal = (self.position - b.pos).normalize();
                        self.velocity -= normal * self.velocity.dot(normal) * 1.5;
                    }
                } else {
                    let normal = (self.position - b.pos).normalize();
                    self.velocity -= normal * self.velocity.dot(normal) * 1.5;
                }
            }
        }

        let mut ground_y = self.half_size;
        let mut on_gel = None;

        for gel in &world.gel_patches {
            let min = gel.pos - gel.half_size;
            let max = gel.pos + gel.half_size;
            if new_pos.x > min.x && new_pos.x < max.x && new_pos.z > min.z && new_pos.z < max.z {
                if new_pos.y >= max.y {
                    if self.velocity.y <= 0.0 && (new_pos.y - max.y) < 2.0 {
                        ground_y = max.y + self.half_size;
                        on_gel = Some(gel.gel_type);
                    }
                }
            }
        }

        if new_pos.y < ground_y {
            new_pos.y = ground_y;
            if let Some(gel_type) = on_gel {
                match gel_type {
                    GelType::Repulsion => {
                        if self.velocity.y < -5.0 { self.velocity.y = -self.velocity.y * 0.8; } 
                        else { self.velocity.y = 0.0; }
                    }
                    GelType::Propulsion => {
                        self.velocity.y = 0.0;
                        let horiz = Vec3::new(self.velocity.x, 0.0, self.velocity.z);
                        if horiz.length() > 0.5 {
                            let dir = horiz.normalize();
                            self.velocity.x += dir.x * 10.0 * dt;
                            self.velocity.z += dir.z * 10.0 * dt;
                        }
                    }
                }
            } else {
                self.velocity.y *= -0.4; 
                self.velocity.x *= 0.8; self.velocity.z *= 0.8;
            }
        }

        for (center, half_size) in &world.pillars {
            let min = *center - Vec3::splat(*half_size);
            let max = *center + Vec3::splat(*half_size);
            let closest = new_pos.clamp(min, max);
            let diff = new_pos - closest;
            let dist_sq = diff.length_squared();
            if dist_sq < self.half_size * self.half_size {
                let dist = dist_sq.sqrt();
                let normal = if dist > 0.0 { diff / dist } else { Vec3::Y };
                new_pos = closest + normal * self.half_size;
                let vel_dot = self.velocity.dot(normal);
                if vel_dot < 0.0 { self.velocity -= normal * vel_dot * 1.5; }
            }
        }

        for p in &world.portals {
            let prev = self.position;
            let n = p.transform.forward();
            let d_prev = (prev - p.transform.position).dot(n);
            let d_new  = (new_pos - p.transform.position).dot(n);
            if d_prev > 0.0 && d_new <= 0.0 {
                let local = prev - p.transform.position;
                if p.transform.up().dot(local).abs() < p.half_height + self.half_size && p.transform.right().dot(local).abs() < p.half_width + self.half_size {
                    if let Some(linked) = p.linked {
                        let target = &world.portals[linked];
                        let link = portal_link_matrix(&p.transform, p.scale, &target.transform, target.scale);
                        new_pos = teleport_point(new_pos, link);
                        new_pos += target.transform.forward() * 0.5;
                        self.velocity = teleport_dir(self.velocity, link);
                        let scale_factor = target.scale / p.scale;
                        self.half_size = (self.base_half_size * scale_factor).max(0.1);
                        self.teleported = true;
                    }
                }
            }
        }
        
        if self.position.y < -10.0 { self.dead = true; }
        self.position = new_pos;
    }

    pub fn respawn(&mut self, spawn: Vec3) {
        self.dead = false;
        self.velocity = Vec3::ZERO;
        self.position = spawn;
        self.half_size = self.base_half_size;
    }
}
