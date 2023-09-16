use glam::Vec3;

pub struct AABB {
	pub min: Vec3,
	pub max: Vec3,
}

impl AABB {
	pub fn new() -> AABB {
		AABB{
			min: Vec3 {
				x: f32::INFINITY, 
				y: f32::INFINITY, 
				z: f32::INFINITY, 
			}, 
			max: Vec3 {
				x: -f32::INFINITY, 
				y: -f32::INFINITY, 
				z: -f32::INFINITY, 
			},
		}
	}

	pub fn grow(&mut self, position: Vec3) {
		self.min.x = position.x.min(position.x);
		self.min.y = position.y.min(position.y);
		self.min.z = position.z.min(position.z);
		self.max.x = position.x.max(position.x);
		self.max.y = position.y.max(position.y);
		self.max.z = position.z.max(position.z);
	}

	pub fn area(&mut self) -> f32 {
		let size = self.max - self.min;
		size.x * size.y + 
		size.y * size.z + 
		size.z * size.x
	}
}