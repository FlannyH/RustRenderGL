use glam::Vec3;

#[repr(C)]
pub struct Light {
    pub position: Vec3,
    pub _pad: f32,
    pub color: Vec3,
    pub intensity: f32,
}