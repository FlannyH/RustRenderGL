use gl::types::GLenum;
use glam::Mat4;
use glfw::{Action, Context, Glfw, Key, Window, WindowEvent};
use std::{sync::mpsc::Receiver, mem::size_of, path::Path, fs::File, io::Read, f32::consts::PI};
use queues::{queue, Queue, IsQueue};
use memoffset::offset_of;

use crate::{structs::{Vertex, Transform}, mesh::Model};

pub struct Renderer {
    // Window stuff
    glfw: Glfw,
    window: Window,
    events: Receiver<(f64, WindowEvent)>,

    // Mesh render queue
    mesh_queue: Queue<MeshGPU>,

    // Main triangle shader
    triangle_shader: u32,

    // Constant buffers
    const_buffer_cpu: GlobalConstBuffer,
    const_buffer_gpu: u32,
}

#[derive(Clone)]
pub struct MeshGPU {
    vao: u32,
    vbo: u32,
    n_vertices: i32
}

impl MeshGPU {
    pub fn new() -> Self {
        MeshGPU {
            vao: 0,
            vbo: 0,
            n_vertices: 0,
        }
    }
}

pub struct ModelGPU {
    meshes: Vec<MeshGPU>,
}

enum ShaderPart {
    Vertex,
    Fragment,
}

pub struct GlobalConstBuffer {
    view_projection_matrix: Mat4,
}

impl Renderer {
    pub fn new(width: u32, height: u32, title: &str) -> Result<Self, &str> {
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
        unsafe {
            let error = gl::GetError();
            if error != gl::NO_ERROR {
                return Err("Error {error} occured when initializing OpenGL!")
            }
        }

        // Create renderer
        let mut renderer = Renderer {
            glfw,
            window,
            events,
            mesh_queue: queue![],
            triangle_shader: 0,
            const_buffer_cpu: GlobalConstBuffer { view_projection_matrix: Mat4::IDENTITY  },
            const_buffer_gpu: 0,
        };

        // Load shaders
        renderer.triangle_shader = renderer.load_shader(Path::new("assets/shaders/lit")).expect("Shader loading failed!");

        // Create const buffer
        unsafe {
            gl::GenBuffers(1, &mut renderer.const_buffer_gpu);
            gl::BindBuffer(gl::UNIFORM_BUFFER, renderer.const_buffer_gpu);
            gl::BufferData(gl::UNIFORM_BUFFER, size_of::<GlobalConstBuffer>() as isize, std::mem::transmute(&renderer.const_buffer_cpu), gl::STATIC_DRAW);
        }

        // Return a new renderer object
        Ok(renderer)
    }

    pub fn should_close(&self) -> bool {
        self.window.should_close()
    }

    pub fn update_camera(&mut self, camera_transform: &Transform) {
        // Update CPU-side buffer
        let view_matrix = Mat4::from_rotation_translation(camera_transform.rotation, camera_transform.translation);
        let proj_matrix = Mat4::perspective_rh(PI / 4.0, 16.0 / 9.0, 0.1, 1000.0);
        self.const_buffer_cpu.view_projection_matrix = proj_matrix * view_matrix;

        // Update GPU-side buffer
        unsafe {
            gl::BindBuffer(gl::UNIFORM_BUFFER, self.const_buffer_gpu);
            gl::BufferData(gl::UNIFORM_BUFFER, size_of::<GlobalConstBuffer>() as isize, std::mem::transmute(&self.const_buffer_cpu), gl::STATIC_DRAW);
            gl::BindBuffer(gl::UNIFORM_BUFFER, 0);
        }
    }

    pub fn begin_frame(&self) {
        // Clear the screen
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
            gl::UseProgram(self.triangle_shader);
        }

        // Render mesh queue
        while let Ok(mesh) = self.mesh_queue.remove() {
            // Render the first mesh in the queue
            unsafe {
                // Bind the vertex buffer
                gl::BindVertexArray(mesh.vao);
                gl::BindBuffer(gl::ARRAY_BUFFER, mesh.vbo);
    
                // Bind the constant buffer
                gl::BindBufferBase(gl::UNIFORM_BUFFER, 0, self.const_buffer_gpu);

                // Draw the model
                gl::DrawArrays(gl::TRIANGLES, 0, mesh.n_vertices);
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

    pub fn upload_model(&self, model_cpu: &Model) -> Result<ModelGPU, u32> {
        let mut model_gpu = ModelGPU { meshes: Vec::new() };

        // For each submesh in the model
        for (name, mesh) in &model_cpu.meshes {
            println!("Parsing mesh \"{name}\"");
            // Create a new mesh entry in the model_gpu object
            let mut curr_mesh = MeshGPU::new();

            // Let's put this on the GPU shall we
            unsafe {
                // Create GPU buffers
                gl::GenVertexArrays(1, &mut curr_mesh.vao);
                gl::GenBuffers(1, &mut curr_mesh.vbo);

                // Bind GPU buffers
                gl::BindVertexArray(curr_mesh.vao);
                gl::BindBuffer(gl::ARRAY_BUFFER , curr_mesh.vbo);

                // Define vertex layout
                gl::VertexAttribPointer(0, 3, gl::FLOAT, gl::FALSE, size_of::<Vertex>() as i32, offset_of!(Vertex, position) as *const _);
                gl::VertexAttribPointer(1, 3, gl::FLOAT, gl::TRUE,  size_of::<Vertex>() as i32, offset_of!(Vertex, normal) as *const _);
                gl::VertexAttribPointer(2, 4, gl::FLOAT, gl::FALSE, size_of::<Vertex>() as i32, offset_of!(Vertex, tangent) as *const _);
                gl::VertexAttribPointer(3, 4, gl::FLOAT, gl::FALSE, size_of::<Vertex>() as i32, offset_of!(Vertex, colour) as *const _);
                gl::VertexAttribPointer(4, 2, gl::FLOAT, gl::FALSE, size_of::<Vertex>() as i32, offset_of!(Vertex, uv0) as *const _);
                gl::VertexAttribPointer(5, 2, gl::FLOAT, gl::FALSE, size_of::<Vertex>() as i32, offset_of!(Vertex, uv1) as *const _);

                // Enable each attribute
                gl::EnableVertexAttribArray(0);
                gl::EnableVertexAttribArray(1);
                gl::EnableVertexAttribArray(2);
                gl::EnableVertexAttribArray(3);
                gl::EnableVertexAttribArray(4);
                gl::EnableVertexAttribArray(5);

                // Populate vertex buffer
                gl::BufferData(gl::ARRAY_BUFFER, (size_of::<Vertex>() * mesh.verts.len()) as isize, std::mem::transmute(&mesh.verts[0]), gl::STATIC_DRAW);
               
                // Unbind buffer
                gl::BindVertexArray(0);
                gl::BindBuffer(gl::ARRAY_BUFFER, 0);

                // If we get an error, stop and don't return the model - this should be very unlikely though
                let error = gl::GetError();
                if error != gl::NO_ERROR {
                    return Err(error)
                }

                // Let's set the number of triangles this mesh has
                curr_mesh.n_vertices = (mesh.verts.len()) as i32;
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

    pub fn load_shader(&mut self, path: &Path) -> Result<u32, &str> {
        // Strip out file name
        let file_name = match path.file_name() {
            Some(name) => name,
            None => return Err("Failed to load shader!")
        };

        // Create shader program object
        let program;
        unsafe {
            program = gl::CreateProgram();
        }

        // Load and compile shader parts
        load_shader_part(gl::VERTEX_SHADER, path.with_extension("vert").as_path(), program);
        load_shader_part(gl::FRAGMENT_SHADER, path.with_extension("frag").as_path(), program);
        unsafe {
            gl::LinkProgram(program);
        }

        Ok(program)
    }
}

fn load_shader_part(shader_type: GLenum, path: &Path, program: u32) {
    // Load shader source
    let mut file = File::open(path).expect("Failed to open shader file");
    let mut source = String::new();
    file.read_to_string(&mut source).expect("Failed to read file");
    let source_len = source.len() as i32;
    
    unsafe {
        // Create shader part 
        let shader = gl::CreateShader(shader_type);
        gl::ShaderSource(shader, 1, &source.as_bytes().as_ptr().cast(), &source_len);
        gl::CompileShader(shader);

        // Check for errors
        let mut result = 0;
        let mut log_length = 0;
        gl::GetShaderiv(shader, gl::COMPILE_STATUS, &mut result);
        gl::GetShaderiv(shader, gl::INFO_LOG_LENGTH, &mut log_length);
        let mut error_message: Vec<u8> = vec![0; log_length as usize];
        gl::GetShaderInfoLog(shader, log_length, std::ptr::null_mut(), error_message.as_mut_ptr().cast());
        
        // Did we get an error?
        if log_length > 0 {
            println!("Shader compilation error!\n{}", std::str::from_utf8(error_message.as_slice()).unwrap())
        }

        // Attach to program
        gl::AttachShader(program, shader);
    }
}
