use glam::Vec3;

use crate::{aabb::AABB, structs::Triangle, bvh::Bvh};

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

impl AABB {
	pub fn intersects(&self, ray: &Ray) -> bool {
		let tx1 = (self.min.x - ray.position.x) * ray.one_over_direction.x;
		let tx2 = (self.max.x - ray.position.x) * ray.one_over_direction.x;

		let mut tmin = f32::min(tx1, tx2);
		let mut tmax = f32::max(tx1, tx2);
		
		let ty1 = (self.min.y - ray.position.y) * ray.one_over_direction.y;
		let ty2 = (self.max.y - ray.position.y) * ray.one_over_direction.y;
	
		tmin = f32::max(f32::min(ty1, ty2), tmin);
		tmax = f32::min(f32::max(ty1, ty2), tmax);
	
		let tz1 = (self.min.z - ray.position.z) * ray.one_over_direction.z;
		let tz2 = (self.max.z - ray.position.z) * ray.one_over_direction.z;
	
		tmin = f32::max(f32::min(tz1, tz2), tmin);
		tmax = f32::min(f32::max(tz1, tz2), tmax);
		
		return tmax >= tmin && tmax >= 0.0;
	}
}
