#![allow(clippy::identity_op)]
#![allow(clippy::needless_return)]
#![allow(dead_code)]

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
use glam::{Vec3, Vec4};
use glfw::Key;
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
        .load_model(Path::new("assets/models/spyro.gltf"))
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
    loop {
        if renderer.should_close() {
            break;
        }
        renderer.update_input(&mut user_input);
        camera.update(&user_input, 0.016); //todo: actual delta time
        renderer.update_camera(&camera);
        renderer.begin_frame();
        renderer.draw_model(&model_spyro);
        // Right line
        renderer.draw_line(
            Vec3::new(0.0, 0.0, 0.0),
            Vec3::new(4.0, 0.0, 0.0),
            Vec4::new(1.0, 0.0, 0.0, 1.0),
        );
        // Up line
        renderer.draw_line(
            Vec3::new(0.0, 0.0, 0.0),
            Vec3::new(0.0, 4.0, 0.0),
            Vec4::new(0.0, 1.0, 0.0, 1.0),
        );
        // Forward line
        renderer.draw_line(
            Vec3::new(0.0, 0.0, 0.0),
            Vec3::new(0.0, 0.0, 4.0),
            Vec4::new(0.0, 0.0, 1.0, 1.0),
        );
        renderer.end_frame();
        if user_input.is_key_down(Key::Num1) {
            renderer.mode = RenderMode::Rasterized;
        }
        if user_input.is_key_down(Key::Num2) {
            renderer.mode = RenderMode::RaytracedCPU;
        }
        if user_input.is_key_down(Key::Num3) {
            renderer.mode = RenderMode::RaytracedGPU;
        }

        println!("player pos {:?}", camera.transform.translation);
        println!("player rot {}, {}", camera.pitch, camera.yaw);
    }
}
