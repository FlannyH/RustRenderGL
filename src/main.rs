use glfw::{Action, Context, Key};

fn main() {
    // Initialize GLFW
    let mut glfw = glfw::init(glfw::FAIL_ON_ERRORS).unwrap();

    // Create window
    let (mut window, events) = glfw
        .create_window(
            1920,
            1080,
            "FlanRustRenderer (OpenGL)",
            glfw::WindowMode::Windowed,
        )
        .expect("Failed to create window.");

    // Set context to this window
    window.make_current();
    window.set_key_polling(true);

    // Main loop
    while !window.should_close() {
        // Swap front and back buffers
        window.swap_buffers();

        // Poll for and process events
        glfw.poll_events();
        for (_, event) in glfw::flush_messages(&events) {
            println!("{:?}", event);
            if let glfw::WindowEvent::Key(Key::Escape, _, Action::Press, _) = event {
                window.set_should_close(true)
            }
        }
    }
}
