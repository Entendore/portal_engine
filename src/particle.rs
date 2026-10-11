use glam::Vec3;

pub struct Particle {
    pub pos: Vec3,
    pub vel: Vec3,
    pub color: [f32; 3],
    pub life: f32,
    pub max_life: f32,
}

impl Particle {
    pub fn new(pos: Vec3, normal: Vec3, color: [f32; 3]) -> Self {
        let tangent1 = if normal.y.abs() > 0.9 { Vec3::X } else { Vec3::Y };
        let tangent1 = normal.cross(tangent1).normalize();
        let tangent2 = normal.cross(tangent1).normalize();
        let angle = rand::random::<f32>() * std::f32::consts::TAU;
        let speed = 2.0 + rand::random::<f32>() * 3.0;
        let dir = (tangent1 * angle.cos() + tangent2 * angle.sin()).normalize();
        Self {
            pos, vel: dir * speed + normal * 1.0, color,
            life: 0.5, max_life: 0.5,
        }
    }

    pub fn update(&mut self, dt: f32) -> bool {
        self.life -= dt;
        if self.life <= 0.0 { return false; }
        self.pos += self.vel * dt;
        self.vel.y -= 5.0 * dt;
        true
    }
}
