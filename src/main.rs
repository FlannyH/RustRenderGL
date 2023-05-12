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
    let mut renderer = Renderer::new(1280, 720, "FlanRustRenderer (OpenGL)");

    // todo: implement source-style error model in code, for when a mesh isn't there
    let model_spyro = match Model::load_gltf(Path::new("assets/spyro.gltf")) {
        Ok(model) => model,
        Err(err) => {
            println!("{}", err);
            Model::new()
        }
    };

    // Upload the mesh to the GPU
    renderer.upload_model(model_spyro).expect("Failed to upload model!");

    // Main loop
    while !renderer.should_close() {
        renderer.begin_frame();
        renderer.end_frame();
    }
}
