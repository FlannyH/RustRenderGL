use glam::Vec3;

#[derive(Debug, Clone)]
pub struct Material {
    // Textures - indices to Resources::textures array
    pub tex_alb: i32,
    pub tex_nrm: i32,
    pub tex_mtl_rgh: i32,
    pub tex_emm: i32,

    // Scalars
    pub scl_rgh: f32,
    pub scl_mtl: f32,
    pub scl_emm: Vec3,
}

impl Material {
    pub fn new() -> Self {
        Material {
            tex_alb: -1,
            tex_nrm: -1,
            tex_mtl_rgh: -1,
            tex_emm: -1,
            scl_rgh: 0.0,
            scl_mtl: 0.0,
            scl_emm: Vec3::ZERO,
        }
    }
}
