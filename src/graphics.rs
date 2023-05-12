use glfw::{Action, Context, Glfw, Key, Window, WindowEvent};
use std::sync::mpsc::Receiver;

pub struct Renderer {
    glfw: Glfw,
    window: Window,
    events: Receiver<(f64, WindowEvent)>,
}

pub struct MeshGPU {
    vao: i32,
}

pub struct ModelGPU {
    meshes: Vec<MeshGPU>,
}

impl Renderer {
    pub fn new(width: u32, height: u32, title: &str) -> Renderer {
        // Initialize GLFW
        let mut glfw = glfw::init(glfw::FAIL_ON_ERRORS).unwrap();

        // Create window
        let (mut window, events) = glfw
            .create_window(width, height, title, glfw::WindowMode::Windowed)
            .expect("Failed to create window.");

        // Set context to this window
        glfw.make_context_current(Some(&window));
        window.set_key_polling(true);

        // Init OpenGL
        gl::load_with(|f_name| glfw.get_proc_address_raw(f_name));

        // Return a new renderer object
        Renderer {
            glfw,
            window,
            events,
        }
    }

    pub fn should_close(&self) -> bool {
        self.window.should_close()
    }

    pub fn begin_frame(&self) {
        unsafe {
            gl::ClearColor(0.1, 0.1, 0.2, 1.0);
            gl::Clear(gl::COLOR_BUFFER_BIT);
        }
    }

    pub fn end_frame(&mut self) {
        // Swap front and back buffers
        self.window.swap_buffers();

        // Poll for and process events
        self.glfw.poll_events();
        for (_, event) in glfw::flush_messages(&self.events) {
            println!("{:?}", event);
            if let glfw::WindowEvent::Key(Key::Escape, _, Action::Press, _) = event {
                self.window.set_should_close(true)
            }
        }
    }

    pub fn upload_model(&self, model_spyro: crate::mesh::Model) {
        let mut model_gpu = ModelGPU { meshes: Vec::new() };

        for (name, mesh) in model_spyro.meshes {
            println!()
        }
        unsafe {
            //gl::GenVertexArrays(1, &mut vao);
        }
    }
}
