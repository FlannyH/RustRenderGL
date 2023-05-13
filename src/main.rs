#![allow(clippy::identity_op, dead_code, unused_variables)]

mod graphics;
mod helpers;
mod mesh;
mod structs;
mod texture;
use std::path::Path;

use glam::{Quat, Vec3};
use graphics::Renderer;
use mesh::Model;
use structs::Transform;

fn main() {
    // Create renderer
    let mut renderer = Renderer::new(1280, 720, "FlanRustRenderer (OpenGL)").expect("Failed to initialize renderer");

    // todo: implement source-style error model in code, for when a mesh isn't there
    let model_spyro_cpu = match Model::load_gltf(Path::new("assets/models/spyro.gltf")) {
        Ok(model) => model,
        Err(err) => {
            println!("{}", err);
            Model::new()
        }
    };

    // Upload the mesh to the GPU
    let model_spyro_gpu = renderer.upload_model(&model_spyro_cpu).expect("Failed to upload model!");

    // Create a camera
    let mut camera_transform = Transform {
        translation: Vec3::ZERO,
        rotation: Quat::IDENTITY,
        scale: Vec3::ONE,
    };

    // Main loop
    while !renderer.should_close() {
        renderer.update_camera(&camera_transform);
        renderer.begin_frame();
        renderer.draw_model(&model_spyro_gpu);
        renderer.end_frame();
        camera_transform.translation.z -= 0.02;
    }
}
