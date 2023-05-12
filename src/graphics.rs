use glfw::{Action, Context, Glfw, Key, Window, WindowEvent};
use std::{sync::mpsc::Receiver, mem::size_of};
use queues::{queue, Queue, IsQueue};

use crate::{structs::Vertex};

pub struct Renderer {
    // Window stuff
    glfw: Glfw,
    window: Window,
    events: Receiver<(f64, WindowEvent)>,

    // Mesh render queue
    mesh_queue: Queue<MeshGPU>
}

#[derive(Clone)]
pub struct MeshGPU {
    vao: u32,
    vbo: u32,
    n_triangles: i32
}

impl MeshGPU {
    pub fn new() -> Self {
        MeshGPU {
            vao: 0,
            vbo: 0,
            n_triangles: 0,
        }
    }
}

pub struct ModelGPU {
    meshes: Vec<MeshGPU>,
}

impl Renderer {
    pub fn new(width: u32, height: u32, title: &str) -> Self {
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
            mesh_queue: queue![],
        }
    }

    pub fn should_close(&self) -> bool {
        self.window.should_close()
    }

    pub fn begin_frame(&self) {
        unsafe {
            gl::ClearColor(0.1, 0.1, 0.2, 1.0);
            gl::Clear(gl::COLOR_BUFFER_BIT | gl::DEPTH_BUFFER_BIT);
        }
    }

    pub fn end_frame(&mut self) {
        // Enable depth testing
        // todo: separate all the unsafe gl parts into separate functions
        unsafe {
            gl::Enable(gl::DEPTH_TEST);
            gl::Enable(gl::CULL_FACE);
        }

        // Render mesh queue
        while let Ok(mesh) = self.mesh_queue.remove() {
            // Render the first mesh in the queue
            unsafe {
                // Bind the vertex buffer
                gl::BindVertexArray(mesh.vao);
                gl::BindBuffer(gl::ARRAY_BUFFER, mesh.vbo);
    
                // Draw the model
                gl::DrawArrays(gl::TRIANGLES, 0, mesh.n_triangles);
            }
        }
        

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

    pub fn upload_model(&self, model_cpu: crate::mesh::Model) -> Result<ModelGPU, u32> {
        let mut model_gpu = ModelGPU { meshes: Vec::new() };

        // For each submesh in the model
        for (name, mesh) in model_cpu.meshes {
            println!("Parsing mesh \"{name}\"");
            // Create a new mesh entry in the model_gpu object
            let mut curr_mesh = MeshGPU::new();

            // Let's put this on the GPU shall we
            unsafe {
                // Create GPU buffers
                gl::GenVertexArrays(1, &mut curr_mesh.vao as *mut u32);
                gl::GenBuffers(1, &mut curr_mesh.vbo as *mut u32);

                // Bind GPU buffers
                gl::BindVertexArray(curr_mesh.vao);
                gl::BindBuffer(gl::ARRAY_BUFFER , curr_mesh.vbo);

                // Define vertex layout
                gl::VertexAttribPointer(0, 3, gl::FLOAT, gl::FALSE, size_of::<Vertex>() as i32, std::mem::transmute(mesh.verts.as_ptr()));
                gl::VertexAttribPointer(1, 3, gl::FLOAT, gl::TRUE,  size_of::<Vertex>() as i32, std::mem::transmute(mesh.verts.as_ptr()));
                gl::VertexAttribPointer(2, 4, gl::FLOAT, gl::FALSE, size_of::<Vertex>() as i32, std::mem::transmute(mesh.verts.as_ptr()));
                gl::VertexAttribPointer(3, 4, gl::FLOAT, gl::FALSE, size_of::<Vertex>() as i32, std::mem::transmute(mesh.verts.as_ptr()));
                gl::VertexAttribPointer(4, 2, gl::FLOAT, gl::FALSE, size_of::<Vertex>() as i32, std::mem::transmute(mesh.verts.as_ptr()));
                gl::VertexAttribPointer(5, 2, gl::FLOAT, gl::FALSE, size_of::<Vertex>() as i32, std::mem::transmute(mesh.verts.as_ptr()));

                // Enable each attribute
                gl::EnableVertexAttribArray(0);
                gl::EnableVertexAttribArray(1);
                gl::EnableVertexAttribArray(2);
                gl::EnableVertexAttribArray(3);
                gl::EnableVertexAttribArray(4);
                gl::EnableVertexAttribArray(5);

                // Populate vertex buffer
                gl::BufferData(gl::ARRAY_BUFFER, (size_of::<Vertex>() * mesh.verts.len()) as isize, std::mem::transmute(mesh.verts.as_ptr()), gl::STATIC_DRAW);
               
                // Unbind buffer
                gl::BindVertexArray(0);
                gl::BindBuffer(gl::ARRAY_BUFFER, 0);

                // If we get an error, stop and don't return the model - this should be very unlikely though
                let error = gl::GetError();
                if error != gl::NO_ERROR {
                    return Err(error)
                }

                // Let's set the number of triangles this mesh has
                curr_mesh.n_triangles = (mesh.verts.len() / 3) as i32;
            }

            // Add this mesh to the model_gpu object
            model_gpu.meshes.push(curr_mesh);
        }
        Ok(model_gpu)
    }

    pub fn draw_model(&mut self, model_gpu: &ModelGPU) {
        // Render each mesh separately
        for mesh in &model_gpu.meshes {
            self.draw_mesh(mesh);
        }
    }

    pub fn draw_mesh(&mut self, mesh: &MeshGPU) {
        self.mesh_queue.add(mesh.clone()).expect("Failed to add mesh to mesh queue");
    }
}
