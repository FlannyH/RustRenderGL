use glam::Vec3;
pub struct Ray {
	pub position: Vec3,
	pub direction: Vec3,
	pub one_over_direction: Vec3,
	pub length: f32,
}

pub struct HitInfo {
	pub distance: f32,
	pub normal: Vec3,
}
