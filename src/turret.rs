use crate::math_util::ray_plane_intersect;
use crate::world::World;
use glam::Vec3;

pub struct Turret {
    pub pos: Vec3,
    pub forward: Vec3,
    pub up: Vec3,
    pub active: bool,
    pub fire_timer: f32,
}

impl Turret {
    pub fn update(&mut self, dt: f32, player_pos: Vec3, world: &World) -> bool {
        if !self.active { return false; }
        let to_player = player_pos - self.pos;
        let dist = to_player.length();
        if dist > 15.0 { return false; }
        let dir = to_player / dist;
        if dir.dot(self.forward) < 0.8 { return false; }

        for wall in &world.walls {
            if let Some(t) = ray_plane_intersect(self.pos, dir, wall.center, wall.normal) {
                if t > 0.0 && t < dist {
                    let hit_pos = self.pos + dir * t;
                    let local = hit_pos - wall.center;
                    let right_dir = wall.normal.cross(wall.up).normalize();
                    if wall.up.dot(local).abs() < wall.half_height && right_dir.dot(local).abs() < wall.half_width {
                        return false;
                    }
                }
            }
        }

        self.fire_timer -= dt;
        if self.fire_timer <= 0.0 {
            self.fire_timer = 0.1;
            return true;
        }
        false
    }
}
