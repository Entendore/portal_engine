use crate::camera::Camera;
use crate::math_util::{portal_link_matrix, teleport_point, teleport_dir, lerp, ray_plane_intersect};
use crate::portal::Portal;
use crate::world::World;
use crate::gel::GelType;
use glam::{Quat, Vec3};

pub struct Player {
    pub velocity: Vec3,
    pub on_ground: bool,
    pub height: f32,
    pub bob_time: f32,
    pub teleported: bool,
    pub dead: bool,
    pub ghost_portal: Option<(Vec3, Vec3, Vec3, bool)>,
}

impl Player {
    pub fn new() -> Self { Self { velocity: Vec3::ZERO, on_ground: false, height: 1.7, bob_time: 0.0, teleported: false, dead: false, ghost_portal: None } }

    pub fn update(&mut self, camera: &mut Camera, input: &InputState, dt: f32, world: &World) {
        if self.dead { return; }
        self.teleported = false;
        let speed = if input.sprint { 8.0 } else { 5.0 };
        let mut wish = Vec3::ZERO;
        let fwd = camera.transform.forward() * Vec3::new(1.0, 0.0, 1.0).normalize_or_zero();
        let right = camera.transform.right() * Vec3::new(1.0, 0.0, 1.0).normalize_or_zero();
        if input.forward  { wish += fwd; }
        if input.backward { wish -= fwd; }
        if input.right     { wish += right; }
        if input.left      { wish -= right; }
        if wish.length_squared() > 0.0 { wish = wish.normalize() * speed; }

        let accel = if self.on_ground { 50.0 } else { 5.0 };
        self.velocity.x = lerp(self.velocity.x, wish.x, accel * dt);
        self.velocity.z = lerp(self.velocity.z, wish.z, accel * dt);

        let target_height = if input.crouch { 0.8 } else { 1.7 };
        self.height = lerp(self.height, target_height, 10.0 * dt);

        if input.jump && self.on_ground {
            self.velocity.y = 6.0;
            self.on_ground = false;
        }

        self.velocity.y -= 9.81 * dt;
        let mut new_pos = camera.transform.position + self.velocity * dt;

        let horizontal_speed = Vec3::new(self.velocity.x, 0.0, self.velocity.z).length();
        if self.on_ground && horizontal_speed > 0.5 {
            self.bob_time += dt * horizontal_speed * 1.5;
        } else {
            self.bob_time = lerp(self.bob_time, 0.0, 10.0 * dt);
        }

        for door in &world.doors {
            if !door.is_open {
                let min = door.pos - door.half_size;
                let max = door.pos + door.half_size;
                if new_pos.x > min.x - 0.5 && new_pos.x < max.x + 0.5 &&
                   new_pos.z > min.z - 0.5 && new_pos.z < max.z + 0.5 {
                    let dx = new_pos.x - door.pos.x;
                    let dz = new_pos.z - door.pos.z;
                    if dx.abs() > dz.abs() {
                        new_pos.x = if dx > 0.0 { max.x + 0.5 } else { min.x - 0.5 };
                        self.velocity.x = 0.0;
                    } else {
                        new_pos.z = if dz > 0.0 { max.z + 0.5 } else { min.z - 0.5 };
                        self.velocity.z = 0.0;
                    }
                }
            }
        }

        for (center, half_size) in &world.pillars {
            let min = *center - Vec3::splat(*half_size);
            let max = *center + Vec3::splat(*half_size);
            let closest = new_pos.clamp(min, max);
            let diff = new_pos - closest;
            let dist_sq = diff.length_squared();
            let player_radius = 0.5;
            if dist_sq < player_radius * player_radius {
                let dist = dist_sq.sqrt();
                let normal = if dist > 0.0 { diff / dist } else { Vec3::Y };
                new_pos = closest + normal * player_radius;
                let vel_dot = self.velocity.dot(normal);
                if vel_dot < 0.0 { self.velocity -= normal * vel_dot; }
            }
        }

        let mut ground_y = self.height;
        let mut on_gel = None;

        for gel in &world.gel_patches {
            let min = gel.pos - gel.half_size;
            let max = gel.pos + gel.half_size;
            if new_pos.x > min.x && new_pos.x < max.x && new_pos.z > min.z && new_pos.z < max.z {
                if new_pos.y >= max.y {
                    if self.velocity.y <= 0.0 && (new_pos.y - max.y) < 2.0 {
                        ground_y = max.y + self.height;
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
                        if self.velocity.y < -5.0 { self.velocity.y = -self.velocity.y * 0.8; self.on_ground = false; } 
                        else { self.velocity.y = 0.0; self.on_ground = true; }
                    }
                    GelType::Propulsion => {
                        self.velocity.y = 0.0;
                        self.on_ground = true;
                        let horiz = Vec3::new(self.velocity.x, 0.0, self.velocity.z);
                        if horiz.length() > 0.5 {
                            let dir = horiz.normalize();
                            self.velocity.x += dir.x * 10.0 * dt;
                            self.velocity.z += dir.z * 10.0 * dt;
                        }
                    }
                }
            } else {
                self.velocity.y = 0.0;
                self.on_ground = true;
            }
        } else { self.on_ground = false; }

        for fp in &world.faith_plates {
            let min = fp.pos - fp.half_size;
            let max = fp.pos + fp.half_size;
            if new_pos.x > min.x - 0.5 && new_pos.x < max.x + 0.5 &&
               new_pos.z > min.z - 0.5 && new_pos.z < max.z + 0.5 && new_pos.y < max.y + 0.5 {
                self.velocity = fp.launch_dir * fp.force;
                self.on_ground = false;
            }
        }

        for p in &world.portals {
            let prev = camera.transform.position;
            let n = p.transform.forward();
            let d_prev = (prev - p.transform.position).dot(n);
            let d_new  = (new_pos - p.transform.position).dot(n);
            if d_prev > 0.0 && d_new <= 0.0 {
                let local = prev - p.transform.position;
                if p.transform.up().dot(local).abs() < p.half_height + 0.5 && p.transform.right().dot(local).abs() < p.half_width + 0.5 {
                    if let Some(linked) = p.linked {
                        let target = &world.portals[linked];
                        let link = portal_link_matrix(&p.transform, p.scale, &target.transform, target.scale);
                        new_pos = teleport_point(new_pos, link);
                        new_pos += target.transform.forward() * 0.5;
                        self.velocity = teleport_dir(self.velocity, link);
                        let link_q = Quat::from_mat4(&link).normalize();
                        camera.transform.orientation = (link_q * camera.transform.orientation).normalize();
                        self.teleported = true;
                    }
                }
            }
        }
        
        if camera.transform.position.y < -10.0 { self.dead = true; }

        self.ghost_portal = None;
        if let Some((pos, normal, up, center, hw, hh, portalable)) = self.shoot_portal_hitscan(camera, world, true) {
            let right_dir = normal.cross(up).normalize();
            let local = pos - center;
            let r_off = right_dir.dot(local);
            let u_off = up.dot(local);
            let max_r = hw - 0.9;
            let max_u = hh - 1.2;
            let is_valid = portalable && r_off.abs() <= max_r && u_off.abs() <= max_u;
            self.ghost_portal = Some((pos, normal, up, is_valid));
        }

        camera.transform.position = new_pos;
    }

    pub fn try_grab_cube(&self, camera: &Camera, cube: &crate::cube::Cube, portals: &[Portal]) -> bool {
        let mut ray_origin = camera.transform.position;
        let mut ray_dir = camera.transform.forward();
        let mut max_dist = 4.0;

        for p in portals {
            if let Some(t) = ray_plane_intersect(ray_origin, ray_dir, p.transform.position, p.transform.forward()) {
                if t > 0.0 && t < max_dist {
                    let hit_pos = ray_origin + ray_dir * t;
                    let local = hit_pos - p.transform.position;
                    if p.transform.up().dot(local).abs() < p.half_height && p.transform.right().dot(local).abs() < p.half_width {
                        if let Some(linked) = p.linked {
                            let link = portal_link_matrix(&p.transform, p.scale, &portals[linked].transform, portals[linked].scale);
                            ray_origin = link.transform_point3(hit_pos);
                            ray_dir = link.transform_vector3(ray_dir).normalize();
                            max_dist -= t;
                            if max_dist <= 0.0 { return false; }
                            break;
                        }
                    }
                }
            }
        }

        let to_cube = cube.position - ray_origin;
        let proj = to_cube.dot(ray_dir);
        if proj < 0.0 || proj > max_dist { return false; } 
        let closest = ray_origin + ray_dir * proj;
        let dist = (closest - cube.position).length();
        dist < cube.half_size + 0.2
    }

    pub fn shoot_portal_hitscan(&self, camera: &Camera, world: &World, is_blue: bool) -> Option<(Vec3, Vec3, Vec3, Vec3, f32, f32, bool)> {
        let ray_origin = camera.transform.position;
        let ray_dir = camera.transform.forward();
        let mut closest_hit: Option<(f32, Vec3, Vec3, Vec3, Vec3, f32, f32, bool)> = None;
        
        for wall in &world.walls {
            if let Some(t) = ray_plane_intersect(ray_origin, ray_dir, wall.center, wall.normal) {
                let hit_pos = ray_origin + ray_dir * t;
                let local = hit_pos - wall.center;
                let right_dir = wall.normal.cross(wall.up).normalize();
                let u = wall.up.dot(local);
                let r = right_dir.dot(local);
                if u.abs() < wall.half_height && r.abs() < wall.half_width {
                    if closest_hit.is_none() || t < closest_hit.unwrap().0 {
                        let portal_up = if wall.normal.y.abs() > 0.9 {
                            let fwd = ray_dir;
                            let projected = fwd - wall.normal * fwd.dot(wall.normal);
                            if projected.length_squared() > 0.001 { projected.normalize() } else { wall.up }
                        } else { wall.up };
                        closest_hit = Some((t, hit_pos, wall.normal, portal_up, wall.center, wall.half_width, wall.half_height, wall.portalable));
                    }
                }
            }
        }
        
        if let Some((_, pos, normal, up, center, hw, hh, portalable)) = closest_hit {
            if portalable {
                for blocker in &world.portal_blockers {
                    let min = blocker.pos - blocker.half_size;
                    let max = blocker.pos + blocker.half_size;
                    if pos.x > min.x && pos.x < max.x && pos.y > min.y && pos.y < max.y && pos.z > min.z && pos.z < max.z {
                        return Some((pos, normal, up, center, hw, hh, false));
                    }
                }
                return Some((pos, normal, up, center, hw, hh, true));
            }
            return Some((pos, normal, up, center, hw, hh, false));
        }
        None
    }

    pub fn spawn_portal_at(&self, pos: Vec3, normal: Vec3, up: Vec3, center: Vec3, wall_hw: f32, wall_hh: f32, is_blue: bool, world: &mut World) {
        let portal_idx = if is_blue { 0 } else { 1 };
        let other_idx = 1 - portal_idx;
        let dist_to_other = (world.portals[other_idx].transform.position - pos).length();
        if dist_to_other < 2.0 { return; }

        let right_dir = normal.cross(up).normalize();
        let local = pos - center;
        let r_off = right_dir.dot(local);
        let u_off = up.dot(local);
        let max_r = wall_hw - 0.9;
        let max_u = wall_hh - 1.2;
        if r_off.abs() > max_r || u_off.abs() > max_u { return; }

        let final_pos = pos; 
        let (w, h) = if is_blue { (0.9, 1.2) } else { (1.8, 2.4) };

        world.portals[portal_idx] = Portal::new(final_pos + normal * 0.05, normal, up, w, h);
        world.portals[portal_idx].color = if is_blue { [0.3, 0.7, 1.0] } else { [1.0, 0.5, 0.3] };
        world.portals[portal_idx].linked = Some(1 - portal_idx);
    }

    pub fn reset_portals(&self, world: &mut World) {
        world.portals[0].transform.position = Vec3::new(0.0, -100.0, -9.9);
        world.portals[1].transform.position = Vec3::new(0.0, -100.0, 9.9);
    }

    pub fn respawn(&mut self, camera: &mut Camera, spawn: Vec3) {
        self.dead = false;
        self.velocity = Vec3::ZERO;
        camera.transform.position = spawn;
    }
}

pub struct InputState {
    pub forward: bool, pub backward: bool,
    pub left: bool,    pub right: bool,
    pub sprint: bool, pub jump: bool, pub toggle_grab: bool,
    pub crouch: bool,
}
impl Default for InputState {
    fn default() -> Self {
        Self { forward: false, backward: false, left: false, right: false, sprint: false, jump: false, toggle_grab: false, crouch: false }
    }
}
