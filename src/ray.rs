use glam::{Vec2, Vec3};

use crate::{
    aabb::AABB,
    bvh::Bvh,
    sphere::Sphere,
    structs::{Triangle, Vertex},
};

pub struct Ray {
    pub position: Vec3,
    pub direction: Vec3,
    pub one_over_direction: Vec3,
    pub length: f32,
}

pub struct HitInfo {
    pub distance: f32,
    pub normal: Vec3,
    pub uv: Vec2,
    pub triangle_index: i32,
}

pub struct HitInfoExt {
    pub distance: f32,
    pub vertex_interpolated: Vertex,
}

impl Vertex {
    pub fn from_triangle_with_uv(triangle: &Triangle, u: f32, v: f32) -> Self {
        triangle.v0 + ((triangle.v1 - triangle.v0) * u) + ((triangle.v2 - triangle.v0) * v)
    }
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

        if det <= 0.0 {
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
        if v < 0.0 || (u + v) > 1.0 {
            return None;
        }

        let t = inv_det * edge2.dot(q);
        if t > 0.0 {
            return Some(HitInfo {
                distance: t,
                normal: edge1.cross(edge2).normalize(),
                triangle_index: -1,
                uv: Vec2 { x: u, y: v },
            });
        }

        return None;
    }
}

impl Bvh {
    pub fn intersects(&self, ray: &Ray) -> Option<HitInfoExt> {
        let mut hit_info = HitInfo {
            distance: f32::INFINITY,
            normal: Vec3::ZERO,
            uv: Vec2::ZERO,
            triangle_index: -1,
        };
        self.intersects_sub(ray, 0, &mut hit_info);
        if hit_info.distance == f32::INFINITY {
            None
        } else {
            Some(HitInfoExt {
                distance: hit_info.distance,
                vertex_interpolated: Vertex::from_triangle_with_uv(
                    self.triangles
                        .get(hit_info.triangle_index as usize)
                        .unwrap(),
                    hit_info.uv.x,
                    hit_info.uv.y,
                ),
            })
        }
    }

    fn intersects_sub(&self, ray: &Ray, node_index: i32, hit_info: &mut HitInfo) {
        let node = &self.nodes[node_index as usize];

        // Intersect node bounding box
        if node.bounds.intersects(ray) {
            // If it's a leaf node
            if node.count != -1 {
                // Loop over all triangles it contains
                let begin = node.left_first;
                let end = node.left_first + node.count;
                for i in begin..end {
                    // Perform intersection test
                    let triangle = &self.triangles[self.indices[i as usize] as usize];
                    if let Some(new_hit_info) = triangle.intersects(ray) {
                        // Is this one closer than the previous one we tested?
                        if new_hit_info.distance < hit_info.distance && new_hit_info.distance >= 0.0
                        {
                            // If so, copy the new hit info data
                            *hit_info = new_hit_info;
                            hit_info.triangle_index = self.indices[i as usize] as i32;
                        }
                    }
                }
                return;
            }
            self.intersects_sub(ray, node.left_first + 0, hit_info);
            self.intersects_sub(ray, node.left_first + 1, hit_info);
        }
    }
}

impl Sphere {
    pub fn intersects(&self, ray: &Ray) -> Option<HitInfoExt> {
        let o2c = ray.position - self.position;
        let b = o2c.dot(ray.direction);
        let c = o2c.dot(o2c) - self.radius_squared;
        let d = b * b - c;
        if d >= 0.0 {
            let sqrt_d = d.sqrt();
            let distance1 = (-b - sqrt_d);
            let distance2 = (-b + sqrt_d);
            if distance1 >= 0.0 {
                let mut hit = Vertex::zero();
                hit.position = ray.position + ray.direction * distance1;
                hit.normal = hit.position - self.position;
                return Some(HitInfoExt {
                    distance: distance1,
                    vertex_interpolated: hit,
                });
            } else {
                let mut hit = Vertex::zero();
                hit.position = ray.position + ray.direction * distance2;
                hit.normal = hit.position - self.position;
                return Some(HitInfoExt {
                    distance: distance2,
                    vertex_interpolated: hit,
                });
            }
        }
        return None;
    }
}

impl Ray {
    pub fn new(position: Vec3, direction: Vec3, length: Option<f32>) -> Self {
        Self {
            position,
            direction,
            one_over_direction: Vec3::ONE / direction,
            length: match length {
                Some(length) => length,
                None => f32::INFINITY,
            },
        }
    }
}
