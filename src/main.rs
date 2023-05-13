#![allow(clippy::identity_op, dead_code, unused_variables)]

mod graphics;
mod helpers;
mod mesh;
mod structs;
mod texture;
use std::path::Path;

use graphics::Renderer;
use mesh::Model;

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

    // Main loop
    while !renderer.should_close() {
        renderer.begin_frame();
        renderer.draw_model(&model_spyro_gpu);
        renderer.end_frame();
    }
}
