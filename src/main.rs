mod graphics;
use graphics::Renderer;

fn main() {
    let mut renderer = Renderer::new(1280, 720, "FlanRustRenderer (OpenGL)");

    // Main loop
    while !renderer.should_close() {
        renderer.begin_frame();
        renderer.end_frame();
    }
}
