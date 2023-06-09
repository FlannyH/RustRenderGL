use gl::types::GLenum;
use glam::Mat4;
use glfw::{Context, Glfw, Window, WindowEvent};
use memoffset::offset_of;
use queues::{queue, IsQueue, Queue};
use std::{
    f32::consts::PI, ffi::c_void, fs::File, io::Read, mem::size_of, path::Path, sync::mpsc::Receiver, collections::{HashMap, hash_map::DefaultHasher}, hash::Hasher,
};
use std::hash::Hash;

use crate::{camera::Camera, input::UserInput, structs::Vertex, mesh::Model, texture::Texture};

pub struct Renderer {
    // Window stuff
    glfw: Glfw,
    window: Window,
    events: Receiver<(f64, WindowEvent)>,

    // Resources
    models: HashMap<u64, Model>,

    // Mesh render queue
    mesh_queue: Queue<MeshQueueEntry>,

    // Main triangle shader
    triangle_shader: u32,

    // Constant buffers
    const_buffer_cpu: GlobalConstBuffer,
    const_buffer_gpu: u32,
}

#[derive(Clone)]
pub struct MeshQueueEntry {
    vao: u32,
    vbo: u32,
    n_vertices: i32,
    material: crate::material::Material,
}

pub struct GlobalConstBuffer {
    view_projection_matrix: Mat4,
}

impl Renderer {
    pub fn new(
        width: u32,
        height: u32,
        title: &str,
    ) -> Result<Self, ()> {
        // Initialize GLFW
        let mut glfw = glfw::init(glfw::FAIL_ON_ERRORS).unwrap();

        // Create window
        let (mut window, events) = glfw
            .create_window(width, height, title, glfw::WindowMode::Windowed)
            .expect("Failed to create window.");

        // Set context to this window
        glfw.make_context_current(Some(&window));
        window.set_all_polling(true);

        // Init OpenGL
        gl::load_with(|f_name| glfw.get_proc_address_raw(f_name));
        unsafe {
            let error = gl::GetError();
            if error != gl::NO_ERROR {
                return Err(());
            }
        }

        // Create renderer
        let mut renderer = Renderer {
            glfw,
            window,
            events,
            mesh_queue: queue![],
            triangle_shader: 0,
            const_buffer_cpu: GlobalConstBuffer {
                view_projection_matrix: Mat4::IDENTITY,
            },
            const_buffer_gpu: 0,
            models: HashMap::new(),
        };

        // Load shaders
        renderer.triangle_shader = renderer
            .load_shader(Path::new("assets/shaders/lit"))
            .expect("Shader loading failed!");

        // Create const buffer
        unsafe {
            gl::GenBuffers(1, &mut renderer.const_buffer_gpu);
            gl::BindBuffer(gl::UNIFORM_BUFFER, renderer.const_buffer_gpu);
            gl::BufferData(
                gl::UNIFORM_BUFFER,
                size_of::<GlobalConstBuffer>() as isize,
                &renderer.const_buffer_cpu as *const GlobalConstBuffer as *const c_void,
                gl::STATIC_DRAW,
            );
        }

        // Return a new renderer object
        Ok(renderer)
    }

    pub fn should_close(&self) -> bool {
        self.window.should_close()
    }

    pub fn update_camera(&mut self, camera: &Camera) {
        // Update CPU-side buffer
        let view_matrix = camera.transform.view_matrix();
        let proj_matrix = Mat4::perspective_rh(PI / 4.0, 16.0 / 9.0, 0.1, 1000.0);
        self.const_buffer_cpu.view_projection_matrix = proj_matrix * view_matrix;

        // Update GPU-side buffer
        unsafe {
            gl::BindBuffer(gl::UNIFORM_BUFFER, self.const_buffer_gpu);
            gl::BufferData(
                gl::UNIFORM_BUFFER,
                size_of::<GlobalConstBuffer>() as isize,
                &self.const_buffer_cpu as *const GlobalConstBuffer as *const c_void,
                gl::STATIC_DRAW,
            );
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

                // Bind the texture
                gl::BindTexture(gl::TEXTURE_2D, mesh.material.tex_alb as u32);

                // Draw the model
                gl::DrawArrays(gl::TRIANGLES, 0, mesh.n_vertices);
            }
        }

        // Swap front and back buffers
        self.window.swap_buffers();
    }

    pub fn update_input(&mut self, input: &mut UserInput) {
        // Poll for and process events
        self.glfw.poll_events();
        for (_, event) in glfw::flush_messages(&self.events) {
            input.process_event(&event);
        }
    }

    pub fn load_model(&mut self, path: &Path) -> Result<u64, u32> {
        // Try to load model
        let model = Model::load_gltf(path, self);
        if model.is_err() {
            println!("Error loading model: {}", model.err().unwrap());
            return Err(0)
        }
        let mut model_cpu = model.unwrap();

        // Upload each submesh in the model to OpenGL
        for (name, mesh) in &mut model_cpu.meshes {
            println!("Parsing mesh \"{name}\"");

            // Let's put this on the GPU shall we
            unsafe {
                // Create GPU buffers
                gl::GenVertexArrays(1, &mut mesh.vao);
                gl::GenBuffers(1, &mut mesh.vbo);

                // Bind GPU buffers
                gl::BindVertexArray(mesh.vao);
                gl::BindBuffer(gl::ARRAY_BUFFER, mesh.vbo);

                // Define vertex layout
                gl::VertexAttribPointer(
                    0,
                    3,
                    gl::FLOAT,
                    gl::FALSE,
                    size_of::<Vertex>() as i32,
                    offset_of!(Vertex, position) as *const _,
                );
                gl::VertexAttribPointer(
                    1,
                    3,
                    gl::FLOAT,
                    gl::TRUE,
                    size_of::<Vertex>() as i32,
                    offset_of!(Vertex, normal) as *const _,
                );
                gl::VertexAttribPointer(
                    2,
                    4,
                    gl::FLOAT,
                    gl::FALSE,
                    size_of::<Vertex>() as i32,
                    offset_of!(Vertex, tangent) as *const _,
                );
                gl::VertexAttribPointer(
                    3,
                    4,
                    gl::FLOAT,
                    gl::FALSE,
                    size_of::<Vertex>() as i32,
                    offset_of!(Vertex, colour) as *const _,
                );
                gl::VertexAttribPointer(
                    4,
                    2,
                    gl::FLOAT,
                    gl::FALSE,
                    size_of::<Vertex>() as i32,
                    offset_of!(Vertex, uv0) as *const _,
                );
                gl::VertexAttribPointer(
                    5,
                    2,
                    gl::FLOAT,
                    gl::FALSE,
                    size_of::<Vertex>() as i32,
                    offset_of!(Vertex, uv1) as *const _,
                );

                // Enable each attribute
                gl::EnableVertexAttribArray(0);
                gl::EnableVertexAttribArray(1);
                gl::EnableVertexAttribArray(2);
                gl::EnableVertexAttribArray(3);
                gl::EnableVertexAttribArray(4);
                gl::EnableVertexAttribArray(5);

                // Populate vertex buffer
                gl::BufferData(
                    gl::ARRAY_BUFFER,
                    (size_of::<Vertex>() * mesh.verts.len()) as isize,
                    &mesh.verts[0] as *const Vertex as *const c_void,
                    gl::STATIC_DRAW,
                );

                // Unbind buffer
                gl::BindVertexArray(0);
                gl::BindBuffer(gl::ARRAY_BUFFER, 0);

                // If we get an error, stop and don't return the model - this should be very unlikely though
                let error = gl::GetError();
                if error != gl::NO_ERROR {
                    return Err(error);
                }
            }
        }

        // Upload each material
        for (name, material) in &model_cpu.materials {
            // Combine name to follow this scheme "test.gltf::materials/mat_name/albedo"
            let new_name = format!("{}::materials/{}/albedo", path.display(), name);
            println!("{:?}", material);
        }

        // Calculate hash
        let mut s = DefaultHasher::new();
        path.hash(&mut s);
        let hash_id = s.finish();

        // Insert model in to model map
        self.models.insert(hash_id, model_cpu);

        // Return the handle
        Ok(hash_id)
    }

    pub fn draw_model(&mut self, model_id: &u64) {
        // Render each mesh separately
        if !self.models.contains_key(model_id) {
            return;
        }
        for (name, mesh) in &self.models.get(model_id).unwrap().meshes {
            self.mesh_queue
                .add(MeshQueueEntry {
                    vao: mesh.vao,
                    vbo: mesh.vbo,
                    n_vertices: mesh.verts.len() as i32,
                    material: self.models.get(model_id).unwrap().materials.get(name).unwrap().clone(),
                })
                .expect("Failed to add mesh to mesh queue");
        }
    }

    pub fn load_shader(&mut self, path: &Path) -> Result<u32, &str> {
        // Create shader program object
        let program;
        unsafe {
            program = gl::CreateProgram();
        }

        // Load and compile shader parts
        load_shader_part(
            gl::VERTEX_SHADER,
            path.with_extension("vert").as_path(),
            program,
        );
        load_shader_part(
            gl::FRAGMENT_SHADER,
            path.with_extension("frag").as_path(),
            program,
        );
        unsafe {
            gl::LinkProgram(program);
        }

        Ok(program)
    }

    pub fn upload_texture(&self, texture: &mut Texture) -> u32{
        unsafe {
            gl::GenTextures(1, &mut texture.gl_id);
            gl::BindTexture(gl::TEXTURE_2D, texture.gl_id);
            gl::TexImage2D(gl::TEXTURE_2D, 0,  gl::RGBA8 as i32, texture.width as i32, texture.height as i32, 0, gl::RGBA, gl::UNSIGNED_BYTE, texture.data.as_ptr()  as *const _);
            gl::GenerateMipmap(gl::TEXTURE_2D);
            gl::TexParameteri(gl::TEXTURE_2D, gl::TEXTURE_MIN_FILTER, gl::NEAREST as i32);
            gl::TexParameteri(gl::TEXTURE_2D, gl::TEXTURE_MAG_FILTER, gl::NEAREST as i32);
        }
        return texture.gl_id;
    }
}
fn load_shader_part(shader_type: GLenum, path: &Path, program: u32) {
    // Load shader source
    let mut file = File::open(path).expect("Failed to open shader file");
    let mut source = String::new();
    file.read_to_string(&mut source)
        .expect("Failed to read file");
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
        gl::GetShaderInfoLog(
            shader,
            log_length,
            std::ptr::null_mut(),
            error_message.as_mut_ptr().cast(),
        );

        // Did we get an error?
        if log_length > 0 {
            println!(
                "Shader compilation error!\n{}",
                std::str::from_utf8(error_message.as_slice()).unwrap()
            )
        }

        // Attach to program
        gl::AttachShader(program, shader);
    }
}