use glam::Vec3;

#[derive(Clone, Copy, PartialEq)]
pub enum GelType {
    Repulsion,
    Propulsion,
}

#[derive(Clone)]
pub struct GelPatch {
    pub pos: Vec3,
    pub half_size: Vec3,
    pub gel_type: GelType,
}
