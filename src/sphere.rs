use glam::Vec3;

pub struct Sphere {
    pub position: Vec3,
    pub radius_squared: f32,
}

impl Sphere {
    pub fn new(position: Vec3, radius: f32) -> Self {
        Self {
            position,
            radius_squared: radius * radius,
        }
    }
}
