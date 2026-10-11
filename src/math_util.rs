use glam::{Mat4, Quat, Vec3};

#[derive(Clone, Copy, Debug)]
pub struct Transform {
    pub position: Vec3,
    pub orientation: Quat,
}

impl Transform {
    pub fn new(position: Vec3, orientation: Quat) -> Self {
        Self { position, orientation }
    }

    pub fn to_matrix(&self) -> Mat4 {
        Mat4::from_translation(self.position) * Mat4::from_quat(self.orientation)
    }

    pub fn view_matrix(&self) -> Mat4 {
        (Mat4::from_quat(self.orientation.inverse())
            * Mat4::from_translation(-self.position))
    }

    pub fn forward(&self) -> Vec3 { self.orientation * Vec3::NEG_Z }
    pub fn up(&self) -> Vec3      { self.orientation * Vec3::Y }
    pub fn right(&self) -> Vec3   { self.orientation * Vec3::X }
}

pub fn portal_link_matrix(a: &Transform, a_scale: f32, b: &Transform, b_scale: f32) -> Mat4 {
    let a_world = a.to_matrix();
    let world_to_a = a_world.inverse();
    let flip = Mat4::from_rotation_y(std::f32::consts::PI);
    let relative_scale = b_scale / a_scale;
    let scale_mat = Mat4::from_scale(Vec3::splat(relative_scale));
    b.to_matrix() * scale_mat * flip * world_to_a
}

pub fn teleport_point(world_p: Vec3, link: Mat4) -> Vec3 {
    link.transform_point3(world_p)
}

pub fn teleport_dir(world_d: Vec3, link: Mat4) -> Vec3 {
    link.transform_vector3(world_d)
}

pub fn ray_plane_intersect(ray_origin: Vec3, ray_dir: Vec3, plane_origin: Vec3, plane_normal: Vec3) -> Option<f32> {
    let denom = plane_normal.dot(ray_dir);
    if denom > -1e-6 && denom < 1e-6 { return None; }
    let t = (plane_origin - ray_origin).dot(plane_normal) / denom;
    if t >= 0.0 { Some(t) } else { None }
}

pub fn lerp(a: f32, b: f32, t: f32) -> f32 { a + (b - a) * t }
