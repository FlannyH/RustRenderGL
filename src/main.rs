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
mod sphere;
mod structs;
mod texture;
mod light;
mod shader;
mod raster;
mod raytrace_cpu;
mod raytrace_gpu;

use std::path::Path;

use camera::Camera;
use glam::Vec3;
use glfw::Key;
use graphics::Renderer;
use input::UserInput;
use light::Light;
use sphere::Sphere;
use structs::Transform;

use crate::graphics::RenderMode;

fn main() {
    // Create renderer and input
    let mut renderer = Renderer::new(1280, 720, "FlanRustRenderer (OpenGL)")
        .expect("Failed to initialize renderer");
    let mut user_input = UserInput::new();

    // Upload the mesh to the GPU
    println!("hmm");
    let model_sponza = renderer
        .load_model(Path::new("assets/models/sponza/glTF/Sponza.gltf"))
        .expect("Failed to upload model!");

    // Create a camera
    let mut camera = Camera::new(
        Transform {
            translation: glam::vec3(0.0, 0.0, 3.0),
            rotation: glam::quat(0.0, 0.0, 0.0, 1.0),
            scale: glam::vec3(1.0, 1.0, 1.0),
        },
        0.5,
        0.005,
    );

    renderer.add_model(
        &model_sponza, 
        Transform {
            translation: glam::vec3(0.0, 0.0, 3.0),
            rotation: glam::quat(0.0, 0.0, 0.0, 1.0),
            scale: glam::vec3(1.0, 1.0, 1.0),
        },
    );
    renderer.add_light(Light {
        position: Vec3::new(1.0, 1.5, 0.5) * 1000.0,
        color: Vec3::new(1.0, 1.0, 0.8),
        intensity: 200000000.0,
        _pad: 0.0,
    });
    renderer.add_light(Light {
        position: Vec3::new(-1.0, 0.5, 0.5) * 1000.0,
        color: Vec3::new(0.1, 0.1, 0.2),
        intensity: 61000000.0,
        _pad: 0.0,
    });
    renderer.add_light(Light {
        position: Vec3::new(1.0, -0.5,- 0.5) * 1000.0,
        color: Vec3::new(0.2, 0.2, 1.0),
        intensity: 24000000.0,
        _pad: 0.0,
    });
    // Main loop
    loop {
        if renderer.should_close() {
            break;
        }
        renderer.update_input(&mut user_input);
        camera.update(&user_input, 0.016); //todo: actual delta time
        renderer.update_camera(&camera);
        renderer.begin_frame();
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
    }
}
