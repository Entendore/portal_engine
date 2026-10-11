use crate::math_util::Transform;
use glam::{Mat4, Vec3, Vec4, Quat};

pub struct Camera {
    pub transform: Transform,
    pub fov:  f32,
    pub near: f32,
    pub far:  f32,
    pub aspect: f32,
    pub proj_override: Option<Mat4>,
    pub bob_y: f32,
    pub bob_roll: f32,
}

impl Camera {
    pub fn new(fov: f32, aspect: f32) -> Self {
        Self {
            transform: Transform::new(Vec3::new(0.0, 1.7, 5.0), Quat::IDENTITY),
            fov, near: 0.05, far: 1000.0, aspect, proj_override: None,
            bob_y: 0.0, bob_roll: 0.0,
        }
    }

    pub fn view(&self) -> Mat4 { self.transform.view_matrix() }
    pub fn projection(&self) -> Mat4 {
        Mat4::perspective_rh(self.fov, self.aspect, self.near, self.far)
    }

    pub fn view_proj(&self) -> Mat4 {
        let proj = self.proj_override.unwrap_or_else(|| self.projection());
        let bobbed_pos = self.transform.position + Vec3::new(0.0, self.bob_y, 0.0);
        let mut bobbed_orient = self.transform.orientation;
        let roll_q = Quat::from_rotation_z(self.bob_roll);
        bobbed_orient = (bobbed_orient * roll_q).normalize();
        let view = Mat4::from_quat(bobbed_orient.inverse()) * Mat4::from_translation(-bobbed_pos);
        proj * view
    }

    pub fn virtual_through(&self, link: Mat4, target: &Transform) -> Camera {
        let pos = link.transform_point3(self.transform.position);
        let rot_q = Quat::from_mat4(&link).normalize();
        let new_orient = (rot_q * self.transform.orientation).normalize();
        
        let mut v_cam = Camera {
            transform: Transform::new(pos, new_orient),
            fov: self.fov, near: self.near, far: self.far, aspect: self.aspect,
            proj_override: None, bob_y: 0.0, bob_roll: 0.0,
        };

        let view = v_cam.view();
        let portal_pos = target.position;
        let portal_norm = target.forward();
        let view_pos = view.transform_point3(portal_pos);
        let view_norm = view.transform_vector3(portal_norm);
        let d = -view_norm.dot(view_pos);
        let clip_plane = Vec4::new(view_norm.x, view_norm.y, view_norm.z, d);
        v_cam.proj_override = Some(Self::oblique_projection(v_cam.projection(), clip_plane));
        v_cam
    }

    fn oblique_projection(base_proj: Mat4, clip_plane: Vec4) -> Mat4 {
        let mut p = base_proj;
        let q = p.inverse() * Vec4::new(
            clip_plane.x.signum(), clip_plane.y.signum(), 1.0, 1.0
        );
        let c = clip_plane * (2.0 / clip_plane.dot(q).max(1e-6));
        p.x_axis.z = c.x; p.y_axis.z = c.y; p.z_axis.z = c.z; p.w_axis.z = c.w;
        p
    }

    pub fn is_in_view(&self, pos: Vec3, radius: f32) -> bool {
        let to_pos = pos - self.transform.position;
        let dist = to_pos.length();
        if dist < radius { return true; }
        let dir_to_pos = to_pos / dist;
        let forward = self.transform.forward();
        let dot = dir_to_pos.dot(forward);
        let fov_half_cos = (self.fov / 2.0 + 0.2).cos();
        dot > fov_half_cos || dist < radius * 2.0
    }
}

#[repr(C)]
#[derive(Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
pub struct CameraUniform {
    pub view_proj: [[f32; 4]; 4],
    pub pos:       [f32; 4],
}

impl CameraUniform {
    pub fn from(cam: &Camera, time: f32) -> Self {
        let vp = cam.view_proj();
        Self {
            view_proj: vp.to_cols_array_2d(),
            pos: [cam.transform.position.x, cam.transform.position.y, cam.transform.position.z, time],
        }
    }
}
