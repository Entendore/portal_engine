use crate::math_util::Transform;
use glam::{Vec3, Quat};

#[derive(Clone, Copy, Debug)]
pub struct Portal {
    pub transform: Transform,
    pub half_width: f32,
    pub half_height: f32,
    pub linked: Option<usize>,
    pub color: [f32; 3],
    pub scale: f32,
}

impl Portal {
    pub fn new(pos: Vec3, normal: Vec3, up: Vec3, w: f32, h: f32) -> Self {
        let z = -normal.normalize();
        let x = up.normalize().cross(z).normalize();
        let y = z.cross(x);
        let m = glam::Mat4::from_cols(
            glam::Vec4::new(x.x, x.y, x.z, 0.0),
            glam::Vec4::new(y.x, y.y, y.z, 0.0),
            glam::Vec4::new(z.x, z.y, z.z, 0.0),
            glam::Vec4::new(pos.x, pos.y, pos.z, 1.0),
        );
        let orientation = Quat::from_mat4(&m).normalize();
        let scale = (w * h).max(0.1);
        Self {
            transform: Transform::new(pos, orientation),
            half_width: w, half_height: h, linked: None, color: [0.5, 0.8, 1.0], scale,
        }
    }

    pub fn corners(&self) -> [Vec3; 4] {
        let t = &self.transform;
        let r = t.right() * self.half_width;
        let u = t.up()    * self.half_height;
        [
            t.position - r + u, t.position + r + u,
            t.position + r - u, t.position - r - u,
        ]
    }
}
