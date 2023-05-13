#![allow(clippy::identity_op, dead_code, unused_variables)]

mod camera;
mod graphics;
mod helpers;
mod input;
mod mesh;
mod structs;
mod texture;
use std::path::Path;

use camera::Camera;
use graphics::Renderer;
use input::UserInput;
use mesh::Model;
use structs::Transform;

fn main() {
    // Create renderer and input
    let mut renderer = Renderer::new(1280, 720, "FlanRustRenderer (OpenGL)")
        .expect("Failed to initialize renderer");
    let mut user_input = UserInput::new();

    // todo: implement source-style error model in code, for when a mesh isn't there
    let model_spyro_cpu = match Model::load_gltf(Path::new("assets/models/spyro.gltf")) {
        Ok(model) => model,
        Err(err) => {
            println!("{}", err);
            Model::new()
        }
    };

    // Upload the mesh to the GPU
    let model_spyro_gpu = renderer
        .upload_model(&model_spyro_cpu)
        .expect("Failed to upload model!");

    // Create a camera
    let mut camera = Camera::new(
        Transform {
            translation: glam::vec3(0.0, 0.0, 3.0),
            rotation: glam::quat(0.0, 0.0, 0.0, 1.0),
            scale: glam::vec3(1.0, 1.0, 1.0),
        },
        5.0,
        0.005,
    );

    // Main loop
    while !renderer.should_close() {
        renderer.update_input(&mut user_input);
        camera.update(&user_input, 0.016);
        renderer.update_camera(&camera);
        renderer.begin_frame();
        renderer.draw_model(&model_spyro_gpu);
        renderer.end_frame();
        println!("{}", camera.transform.translation);
    }
}
