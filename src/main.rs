#![allow(clippy::identity_op, dead_code, unused_variables)]

mod camera;
mod graphics;
mod helpers;
mod input;
mod material;
mod mesh;
mod resources;
mod structs;
mod texture;
use std::{cell::RefCell, path::Path, rc::Rc};

use camera::Camera;
use graphics::Renderer;
use input::UserInput;
use resources::Resources;
use structs::Transform;

fn main() {
    // Create renderer and input
    let resources = Rc::new(RefCell::new(Resources::new()));
    let renderer = Rc::new(RefCell::new(
        Renderer::new(1280, 720, "FlanRustRenderer (OpenGL)", resources.clone())
            .expect("Failed to initialize renderer"),
    ));
    let mut user_input = UserInput::new();

    // todo: implement source-style error model in code, for when a mesh isn't there
    let model_spyro = resources
        .borrow_mut()
        .load_model(Path::new("assets/models/spyro.gltf"))
        .unwrap_or_else(|_| {
            println!("Model not found!");
            0
        });

    // Upload the mesh to the GPU
    renderer
        .borrow_mut()
        .upload_model(&model_spyro)
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
        let mut renderer_ref = renderer.borrow_mut();
        if renderer_ref.should_close() {
            break;
        }
        renderer_ref.update_input(&mut user_input);
        camera.update(&user_input, 0.016);
        renderer_ref.update_camera(&camera);
        renderer_ref.begin_frame();
        renderer_ref.draw_model(&model_spyro);
        renderer_ref.end_frame();
    }
}
