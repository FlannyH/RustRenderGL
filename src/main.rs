#![allow(clippy::identity_op)]
#![allow(clippy::needless_return)]

mod aabb;
mod bvh;
mod camera;
mod graphics;
mod helpers;
mod input;
mod material;
mod mesh;
mod ray;
mod structs;
mod texture;
use std::path::Path;

use camera::Camera;
use graphics::Renderer;
use input::UserInput;

use structs::Transform;

use crate::graphics::RenderMode;

fn main() {
    // Create renderer and input
    let mut renderer = Renderer::new(1280, 720, "FlanRustRenderer (OpenGL)")
        .expect("Failed to initialize renderer");
    let mut user_input = UserInput::new();

    // Upload the mesh to the GPU
    let model_spyro = renderer
        .load_model(Path::new("assets/models/bvh_test_tiny.gltf"))
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
    let mut counter = 0;
    loop {
        if renderer.should_close() {
            break;
        }
        renderer.update_input(&mut user_input);
        camera.update(&user_input, 0.016); //todo: actual delta time
        renderer.update_camera(&camera);
        renderer.begin_frame();
        renderer.draw_model(&model_spyro);
        renderer.end_frame();
        counter += 1;
        if counter == 60 {
            counter = 0;
            if renderer.mode == RenderMode::Rasterized {
                renderer.mode = RenderMode::RaytracedCPU;
            } else if renderer.mode == RenderMode::RaytracedCPU {
                renderer.mode = RenderMode::Rasterized;
            }
        }
        println!("player pos {:?}", camera.transform.translation);
    }
}
