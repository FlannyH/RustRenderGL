use std::ops::{Add, Mul, Sub};

use glam::{Mat4, Quat, Vec2, Vec3, Vec4};

#[derive(Debug, Copy, Clone)]

#[repr(C)]
pub struct Vertex {
    pub position: Vec3,
    pub normal: Vec3,
    pub tangent: Vec4,
    pub colour: Vec4,
    pub uv0: Vec2,
    pub uv1: Vec2,
}

pub struct Triangle {
    pub v0: Vertex,
    pub v1: Vertex,
    pub v2: Vertex,
}

#[derive(Debug, Copy, Clone)]
pub struct FragIn {
    pub position: Vec4,
    pub normal: Vec3,
    pub tangent: Vec3,
    pub colour: Vec3,
    pub uv: Vec2,
}

pub struct Transform {
    pub translation: Vec3,
    pub rotation: Quat,
    pub scale: Vec3,
}

#[derive(Debug, Clone)]
pub struct Pixel32 {
    pub r: u8,
    pub g: u8,
    pub b: u8,
    pub a: u8,
}

impl FragIn {
    pub fn lerp(&self, rhs: FragIn, t: f32) -> FragIn {
        FragIn {
            position: self.position.lerp(rhs.position, t),
            normal: self.normal.lerp(rhs.normal, t),
            tangent: self.tangent.lerp(rhs.tangent, t),
            colour: self.colour.lerp(rhs.colour, t),
            uv: self.uv.lerp(rhs.uv, t),
        }
    }
}

impl Transform {
    pub fn right(&self) -> Vec3 {
        self.rotation * Vec3::X
    }

    pub fn up(&self) -> Vec3 {
        self.rotation * Vec3::Y
    }

    pub fn forward(&self) -> Vec3 {
        self.rotation * -Vec3::Z
    }
    pub fn view_matrix(&self) -> Mat4 {
        Mat4::look_at_rh(
            self.translation,
            self.translation + self.forward(),
            glam::vec3(0.0, 1.0, 0.0),
        )
    }
    pub fn trans_matrix(&self) -> Mat4 {
        Mat4::from_scale_rotation_translation(self.scale, self.rotation, self.translation)
    }
    pub fn local_matrix(&self) -> Mat4 {
        Mat4::from_translation(self.translation)
            * Mat4::from_quat(self.rotation)
            * Mat4::from_scale(self.scale)
    }
}

impl Mul<f32> for Vertex {
    type Output = Vertex;
    fn mul(self, rhs: f32) -> Self {
        Self {
            position: self.position * rhs,
            normal: self.normal * rhs,
            tangent: self.tangent * rhs,
            colour: self.colour * rhs,
            uv0: self.uv0 * rhs,
            uv1: self.uv1 * rhs,
        }
    }
}

impl Add<Vertex> for Vertex {
    type Output = Vertex;
    fn add(self, rhs: Vertex) -> Self {
        Self {
            position: self.position + rhs.position,
            normal: self.normal + rhs.normal,
            tangent: self.tangent + rhs.tangent,
            colour: self.colour + rhs.colour,
            uv0: self.uv0 + rhs.uv0,
            uv1: self.uv1 + rhs.uv1,
        }
    }
}

impl Sub<Vertex> for Vertex {
    type Output = Vertex;
    fn sub(self, rhs: Vertex) -> Self {
        Self {
            position: self.position - rhs.position,
            normal: self.normal - rhs.normal,
            tangent: self.tangent - rhs.tangent,
            colour: self.colour - rhs.colour,
            uv0: self.uv0 - rhs.uv0,
            uv1: self.uv1 - rhs.uv1,
        }
    }
}
