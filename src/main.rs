#![allow(clippy::identity_op, dead_code, unused_variables)]

mod graphics;
mod helpers;
mod mesh;
mod structs;
mod texture;
use graphics::Renderer;

fn main() {
    let mut renderer = Renderer::new(1280, 720, "FlanRustRenderer (OpenGL)");

    // Main loop
    while !renderer.should_close() {
        renderer.begin_frame();
        renderer.end_frame();
    }
}
