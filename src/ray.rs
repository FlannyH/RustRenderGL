use glam::Vec3;

use crate::{aabb::AABB, bvh::Bvh, structs::Triangle};

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

impl Triangle {
    pub fn intersects(&self, ray: &Ray) -> Option<HitInfo> {
        let edge1 = self.v1.position - self.v0.position;
        let edge2 = self.v2.position - self.v0.position;
        let h = ray.direction.cross(edge2);
        let det = edge1.dot(h);

        if det == 0.0 {
            return None;
        }

        let inv_det = 1.0 / det;
        let v0_ray = ray.position - self.v0.position;
        let u = inv_det * (v0_ray.dot(h));

        if !(0.0..=1.0).contains(&u) {
            return None;
        }

        let q = v0_ray.cross(edge1);
        let v = inv_det * ray.direction.dot(q);
        if !(0.0..=1.0).contains(&v) {
            return None;
        }

        let t = inv_det * edge2.dot(q);
        if t > 0.0 {
            return Some(HitInfo {
                distance: t,
                normal: edge1.cross(edge2).normalize(),
            });
        }

        return None;
    }
}

impl Bvh {
    pub fn intersects(&self, ray: &Ray) -> Option<HitInfo> {
        let mut hit_info = HitInfo {
            distance: f32::INFINITY,
            normal: Vec3::ZERO,
        };
        self.intersects_sub(ray, 0, &mut hit_info);
        match hit_info.distance {
            f32::INFINITY => None,
            _ => Some(hit_info),
        }
    }

    fn intersects_sub(&self, ray: &Ray, node_index: u32, hit_info: &mut HitInfo) {
        let node = &self.nodes[node_index as usize];

        // Intersect node bounding box
        if node.bounds.intersects(ray) {
            // If it's a leaf node
            if node.count > 0 {
                // Loop over all triangles it contains
                let begin = node.left_first;
                let end = node.left_first + node.count;
                for i in begin..end {
                    // Perform intersection test
                    let triangle = &self.triangles[self.indices[i as usize] as usize];
                    if let Some(new_hit_info) = triangle.intersects(ray) {
                        // Is this one closer than the previous one we tested?
                        if new_hit_info.distance < hit_info.distance {
                            // If so, copy the new hit info data
                            *hit_info = new_hit_info;
                        }
                    }
                }
            }
            return;
        }

        self.intersects_sub(ray, node.left_first + 0, hit_info);
        self.intersects_sub(ray, node.left_first + 1, hit_info);
    }
}
