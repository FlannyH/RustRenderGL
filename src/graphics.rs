use glam::{Mat4, Vec3, Vec4};
use glfw::{Context, Glfw, Window, WindowEvent};
use memoffset::offset_of;
use std::hash::Hash;
use std::sync::Arc;
use std::{
    collections::{hash_map::DefaultHasher, HashMap},
    ffi::c_void,
    hash::Hasher,
    mem::size_of,
    path::Path,
    ptr::null,
    sync::mpsc::Receiver,
};

use crate::aabb::AABB;
use crate::bvh::{Bvh, BvhNode};
use crate::light::Light;
use crate::material::Material;
use crate::mesh::Mesh;
use crate::shader::ShaderProgram;
use crate::sphere::Sphere;
use crate::{
    camera::Camera,
    input::UserInput,
    mesh::Model,
    structs::{Pixel32, Vertex},
    texture::Texture,
};

#[derive(PartialEq, Eq, Debug)]
pub enum RenderMode {
    None,
    Rasterized,
    RaytracedCPU,
    RaytracedGPU,
}

pub struct Renderer {
    // Window stuff
    pub glfw: Glfw,
    pub window: Window,
    pub events: Receiver<(f64, WindowEvent)>,
    pub depth_buffer_texture: u32,
    pub framebuffer_texture: u32,
    pub framebuffer_object: u32,
    pub quad_vbo: u32,
    pub quad_vao: u32,
    pub fbo_shader: Option<ShaderProgram>,
    pub window_resolution_prev: [i32; 2],
    pub mode: RenderMode,

    // Resources
    pub models: HashMap<u64, Model>,

    // Mesh render queue
    pub mesh_queue: Vec<MeshQueueEntry>,
    pub line_queue: Vec<LineQueueEntry>,
    pub light_queue: Vec<Light>,
    pub request_reupload: bool,
    pub gpu_lights: u32,

    // Main triangle shader
    pub triangle_shader: Option<ShaderProgram>,
    pub line_shader: Option<ShaderProgram>,

    // Primitives
    pub gpu_spheres: u32,
    pub sphere_queue: Vec<Sphere>,
    pub primitives_model: u64, // key into models hashmap

    // Raytracing stuff
    pub raytracing_shader: Option<ShaderProgram>,
    pub framebuffer_cpu: Vec<Pixel32>,
    pub framebuffer_cpu_to_gpu: u32,

    // Camera
    pub camera_position: Vec3,
    pub camera_rotation_euler: Vec3,
    pub fov: f32, // in radians
    pub aspect_ratio: f32,
    pub viewport_height: f32,
    pub viewport_width: f32,
    pub viewport_depth: f32,

    // Constant buffers
    pub const_buffer_cpu: GlobalConstBuffer,
    pub const_buffer_gpu: u32,
}

#[derive(Clone)]
pub struct MeshQueueEntry {
    pub mesh: Arc<Mesh>,
    pub material: Arc<Material>,
}

#[derive(Clone)]
pub struct LineQueueEntry {
    pub position: Vec3,
    pub color: Vec4,
}

pub struct GlobalConstBuffer {
    pub view_projection_matrix: Mat4,
}

impl Renderer {
    pub fn new(width: u32, height: u32, title: &str) -> Result<Self, ()> {
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
        
        // Enable debug callback
        unsafe {
            gl::DebugMessageCallback(Some(debug_callback), std::ptr::null());
            gl::Enable(gl::DEBUG_OUTPUT);
        }

        // Create renderer
        let mut renderer = Renderer {
            glfw,
            window,
            events,
            depth_buffer_texture: 0,
            framebuffer_texture: 0,
            framebuffer_object: 0,
            quad_vbo: 0,
            quad_vao: 0,
            fbo_shader: None,
            window_resolution_prev: [0, 0],
            mode: RenderMode::RaytracedCPU,
            models: HashMap::new(),
            mesh_queue: vec![],
            line_queue: vec![],
            light_queue: vec![],
            triangle_shader: None,
            line_shader: None,
            raytracing_shader: None,
            framebuffer_cpu: Vec::new(),
            framebuffer_cpu_to_gpu: 0,
            camera_position: Vec3::ZERO,
            camera_rotation_euler: Vec3::ZERO,
            fov: 0.0,
            viewport_height: 0.0,
            viewport_width: 0.0,
            viewport_depth: -1.0,
            const_buffer_cpu: GlobalConstBuffer {
                view_projection_matrix: Mat4::IDENTITY,
            },
            const_buffer_gpu: 0,
            aspect_ratio: 0.0,
            gpu_spheres: 0,
            sphere_queue: Vec::new(),
            primitives_model: 0,
            request_reupload: false,
            gpu_lights: 0,
        };

        // Set FOV
        renderer.fov = 90.0_f32.to_radians();
        renderer.aspect_ratio = width as f32 / height as f32;
        renderer.viewport_height = (renderer.fov * 0.5).tan();
        renderer.viewport_width = renderer.viewport_height * renderer.aspect_ratio;

        // Load shaders
        renderer.fbo_shader = Some(ShaderProgram::load_shader(Path::new("assets/shaders/fbo"))
            .expect("Shader loading failed"));
        renderer.triangle_shader = Some(ShaderProgram::load_shader(Path::new("assets/shaders/lit"))
            .expect("Shader loading failed!"));
        renderer.line_shader = Some(ShaderProgram::load_shader(Path::new("assets/shaders/line"))
            .expect("Shader loading failed!"));
        renderer.raytracing_shader = Some(ShaderProgram::load_shader_compute(Path::new("assets/shaders/ray.comp"))
            .expect("Shader loading failed!"));

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

        // Create framebuffer
        let window_resolution = renderer.window.get_framebuffer_size();
        unsafe {
            // Color
            gl::GenFramebuffers(1, &mut renderer.framebuffer_object);
            gl::BindFramebuffer(gl::FRAMEBUFFER, renderer.framebuffer_object);
            gl::GenTextures(1, &mut renderer.framebuffer_texture);
            gl::BindTexture(gl::TEXTURE_2D, renderer.framebuffer_texture);
            gl::TexImage2D(
                gl::TEXTURE_2D,
                0,
                gl::RGBA16F as _,
                window_resolution.0,
                window_resolution.1,
                0,
                gl::RGBA,
                gl::FLOAT,
                null(),
            );
            gl::TexParameteri(gl::TEXTURE_2D, gl::TEXTURE_MIN_FILTER, gl::NEAREST as _);
            gl::TexParameteri(gl::TEXTURE_2D, gl::TEXTURE_MAG_FILTER, gl::NEAREST as _);
            gl::BindTexture(gl::TEXTURE_2D, 0);
            gl::FramebufferTexture2D(
                gl::FRAMEBUFFER,
                gl::COLOR_ATTACHMENT0,
                gl::TEXTURE_2D,
                renderer.framebuffer_texture,
                0,
            );

            // Depth
            gl::BindFramebuffer(gl::FRAMEBUFFER, renderer.framebuffer_object);
            gl::GenTextures(1, &mut renderer.depth_buffer_texture);
            gl::BindTexture(gl::TEXTURE_2D, renderer.depth_buffer_texture);
            gl::TexImage2D(
                gl::TEXTURE_2D,
                0,
                gl::DEPTH24_STENCIL8 as _,
                window_resolution.0,
                window_resolution.1,
                0,
                gl::DEPTH_STENCIL,
                gl::UNSIGNED_INT_24_8,
                null(),
            );
            gl::BindTexture(gl::TEXTURE_2D, 0);
            gl::FramebufferTexture2D(
                gl::FRAMEBUFFER,
                gl::DEPTH_STENCIL_ATTACHMENT,
                gl::TEXTURE_2D,
                renderer.depth_buffer_texture,
                0,
            );
        }

        // Create cpu framebuffer (for cpu raytrace rendering mode)
        unsafe {
            gl::GenTextures(1, &mut renderer.framebuffer_cpu_to_gpu);
            gl::BindTexture(gl::TEXTURE_2D, renderer.framebuffer_cpu_to_gpu);
            gl::TexImage2D(
                gl::TEXTURE_2D,
                0,
                gl::DEPTH24_STENCIL8 as _,
                window_resolution.0,
                window_resolution.1,
                0,
                gl::DEPTH_STENCIL,
                gl::UNSIGNED_INT_24_8,
                null(),
            );
            gl::BindTexture(gl::TEXTURE_2D, 0);
        }

        // Create screen quad
        unsafe {
            let quad = vec![
                // Vertices
                1.0f32, 1.0, -1.0, -1.0, -1.0, 1.0, 1.0, 1.0, 1.0, -1.0, -1.0, -1.0,
                // Texcoords
                1.0, 1.0, 0.0, 0.0, 0.0, 1.0, 1.0, 1.0, 1.0, 0.0, 0.0, 0.0,
            ];
            gl::GenVertexArrays(1, &mut renderer.quad_vao);
            gl::GenBuffers(1, &mut renderer.quad_vbo);
            gl::BindVertexArray(renderer.quad_vao);
            gl::BindBuffer(gl::ARRAY_BUFFER, renderer.quad_vbo);
            gl::BufferData(
                gl::ARRAY_BUFFER,
                (quad.len() * size_of::<f32>()) as isize,
                quad.as_ptr() as *const c_void,
                gl::STATIC_DRAW,
            );
            gl::EnableVertexAttribArray(0);
            gl::EnableVertexAttribArray(1);
            gl::VertexAttribPointer(0, 2, gl::FLOAT, gl::FALSE, 0, std::ptr::null::<c_void>());
            gl::VertexAttribPointer(1, 2, gl::FLOAT, gl::FALSE, 0, (12 * size_of::<f32>()) as _);
            gl::BindBuffer(gl::ARRAY_BUFFER, 0);
            gl::BindVertexArray(0);
        }

        // Create buffers for primitives
        unsafe {
            gl::GenBuffers(1, &mut renderer.gpu_spheres);
            gl::GenBuffers(1, &mut renderer.gpu_lights);
        }

        // Load sphere model for rasterizer
        renderer.primitives_model = renderer
            .load_model(&Path::new("assets/models/primitives.gltf"))
            .unwrap();

        // Return a new renderer object
        Ok(renderer)
    }

    pub fn should_close(&self) -> bool {
        self.window.should_close()
    }

    pub fn update_camera(&mut self, camera: &Camera) {
        // Update CPU-side buffer
        let view_matrix = camera.transform.view_matrix();
        let proj_matrix = Mat4::perspective_rh(self.fov, self.aspect_ratio, 0.1, 1000.0);
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

        // Update raytrace CPU buffer
        self.camera_position = camera.transform.translation;
        self.camera_rotation_euler.x = camera.pitch;
        self.camera_rotation_euler.y = camera.yaw;
    }

    pub fn begin_frame(&mut self) {
        // Clear the screen
        self.update_framebuffer_resolution();
        unsafe {
            gl::BindFramebuffer(gl::FRAMEBUFFER, self.framebuffer_object);
            gl::ClearColor(0.1, 0.1, 0.2, 1.0);
            gl::ClearDepth(1.0);
            gl::Clear(gl::COLOR_BUFFER_BIT | gl::DEPTH_BUFFER_BIT);
        }
    }

    pub fn end_frame(&mut self) {
        self.upload_when_requested();
        self.fbo_shader.as_mut().unwrap().hot_reload_on_change();
        self.line_shader.as_mut().unwrap().hot_reload_on_change();
        self.triangle_shader.as_mut().unwrap().hot_reload_on_change();
        self.raytracing_shader.as_mut().unwrap().hot_reload_on_change();

        match self.mode {
            RenderMode::None => {}
            RenderMode::Rasterized => self.end_frame_raster(),
            RenderMode::RaytracedCPU => self.end_frame_raytrace_cpu(),
            RenderMode::RaytracedGPU => self.end_frame_raytrace_gpu(),
        }

        // Swap front and back buffers
        self.window.swap_buffers();
    }

    fn upload_when_requested(&mut self) {
        if self.request_reupload {
           self.request_reupload = false;

           // Upload lights
           unsafe {
                gl::BindBuffer(gl::SHADER_STORAGE_BUFFER, self.gpu_lights);
                gl::BufferData(
                    gl::SHADER_STORAGE_BUFFER,
                    (self.light_queue.len() * std::mem::size_of::<Light>()) as isize,
                    self.light_queue.as_ptr() as _,
                    gl::STATIC_DRAW,
                );
                gl::BindBuffer(gl::SHADER_STORAGE_BUFFER, 0);
            }

            // Upload spheres
            unsafe {
                gl::BindBuffer(gl::SHADER_STORAGE_BUFFER, self.gpu_spheres);
                gl::BufferData(
                    gl::SHADER_STORAGE_BUFFER,
                    (self.sphere_queue.len() * std::mem::size_of::<Sphere>()) as isize,
                    self.sphere_queue.as_ptr() as _,
                    gl::STATIC_DRAW,
                );
                gl::BindBuffer(gl::SHADER_STORAGE_BUFFER, 0);
            }
        }
    }



    fn update_framebuffer_resolution(&mut self) {
        // Update OpenGL framebuffer resolution
        let window_resolution = self.window.get_framebuffer_size();
        let window_resolution = [window_resolution.0, window_resolution.1];
        self.aspect_ratio = window_resolution[0] as f32 / window_resolution[1] as f32;
        if window_resolution != self.window_resolution_prev {
            Self::resize_texture(
                &mut self.framebuffer_texture,
                window_resolution[0],
                window_resolution[1],
                gl::RGBA16F as _,
                gl::RGBA,
                gl::FLOAT,
            );
            Self::resize_texture(
                &mut self.depth_buffer_texture,
                window_resolution[0],
                window_resolution[1],
                gl::DEPTH24_STENCIL8 as _,
                gl::DEPTH_STENCIL,
                gl::UNSIGNED_INT_24_8,
            );
            Self::resize_texture(
                &mut self.framebuffer_cpu_to_gpu,
                window_resolution[0],
                window_resolution[1],
                gl::RGBA8 as _,
                gl::RGBA,
                gl::UNSIGNED_BYTE,
            );

            unsafe {
                gl::BindFramebuffer(gl::FRAMEBUFFER, self.framebuffer_object);
                gl::FramebufferTexture2D(
                    gl::FRAMEBUFFER,
                    gl::COLOR_ATTACHMENT0,
                    gl::TEXTURE_2D,
                    self.framebuffer_texture,
                    0,
                );
                gl::FramebufferTexture2D(
                    gl::FRAMEBUFFER,
                    gl::DEPTH_STENCIL_ATTACHMENT,
                    gl::TEXTURE_2D,
                    self.depth_buffer_texture,
                    0,
                );
            }
        }

        // Update software framebuffer resolution
        self.framebuffer_cpu.clear();
        self.framebuffer_cpu.resize(
            (window_resolution[0] / 1 * window_resolution[1] / 1) as usize,
            Pixel32 {
                r: 0,
                g: 0,
                b: 0,
                a: 0,
            },
        );

        // Set FOV
        self.fov = 90.0_f32.to_radians();
        self.viewport_height = (self.fov * 0.5).tan();
        self.viewport_width =
            self.viewport_height * (window_resolution[0] as f32 / window_resolution[1] as f32);

        self.window_resolution_prev = window_resolution;
    }

    fn resize_texture(
        texture: &mut u32,
        width: i32,
        height: i32,
        tex_format_internal: i32,
        tex_format: u32,
        component_type: u32,
    ) {
        unsafe {
            gl::DeleteTextures(1, texture);
            gl::GenTextures(1, texture);
            gl::BindTexture(gl::TEXTURE_2D, *texture);
            gl::TexImage2D(
                gl::TEXTURE_2D,
                0,
                tex_format_internal,
                width,
                height,
                0,
                tex_format,
                component_type,
                null() as *const c_void,
            );
            gl::TexParameteri(gl::TEXTURE_2D, gl::TEXTURE_MIN_FILTER, gl::NEAREST as _);
            gl::TexParameteri(gl::TEXTURE_2D, gl::TEXTURE_MAG_FILTER, gl::NEAREST as _);
            gl::BindTexture(gl::TEXTURE_2D, 0);
        }
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
            return Err(0);
        }
        let mut model_cpu = model.unwrap();

        // Upload each submesh in the model to OpenGL
        for (name, mesh, _material) in &mut model_cpu.meshes {
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

                // Upload the material - combine name to follow this scheme "test.gltf::materials/mat_name/albedo"
                let _new_name = format!("{}::materials/{}/albedo", path.display(), name); // TODO
            }
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
        for (_name, mesh,  material) in self.models.get(model_id).unwrap().meshes.clone() {
            self.mesh_queue.push(MeshQueueEntry {
                mesh: Arc::new(mesh),
                material: Arc::new(material),
            });
        }
    }

    pub fn draw_bvh(&mut self, bvh: Arc<Bvh>, color: Vec4) {
        self.draw_bvh_sub(bvh.clone(), &bvh.nodes[0], color, 0);
    }

    fn draw_bvh_sub(&mut self, bvh: Arc<Bvh>, node: &BvhNode, color: Vec4, rec_depth: i32) {
        self.draw_aabb(&node.bounds, color * rec_depth as f32 * 0.1);
        if node.count == 0 {
            self.draw_bvh_sub(
                bvh.clone(),
                &bvh.nodes[node.left_first as usize],
                color,
                rec_depth + 1,
            );
            self.draw_bvh_sub(
                bvh.clone(),
                &bvh.nodes[node.left_first as usize + 1],
                color,
                rec_depth + 1,
            );
        }
    }

    pub fn draw_line(&mut self, p1: Vec3, p2: Vec3, color: Vec4) {
        self.line_queue.push(LineQueueEntry {
            position: p1,
            color,
        });
        self.line_queue.push(LineQueueEntry {
            position: p2,
            color,
        });
    }

    pub fn draw_aabb(&mut self, aabb: &AABB, color: Vec4) {
        // Create 8 vertices
        let vertex000 = Vec3 {
            x: aabb.min.x,
            y: aabb.min.y,
            z: aabb.min.z,
        };
        let vertex001 = Vec3 {
            x: aabb.min.x,
            y: aabb.min.y,
            z: aabb.max.z,
        };
        let vertex010 = Vec3 {
            x: aabb.min.x,
            y: aabb.max.y,
            z: aabb.min.z,
        };
        let vertex011 = Vec3 {
            x: aabb.min.x,
            y: aabb.max.y,
            z: aabb.max.z,
        };
        let vertex100 = Vec3 {
            x: aabb.max.x,
            y: aabb.min.y,
            z: aabb.min.z,
        };
        let vertex101 = Vec3 {
            x: aabb.max.x,
            y: aabb.min.y,
            z: aabb.max.z,
        };
        let vertex110 = Vec3 {
            x: aabb.max.x,
            y: aabb.max.y,
            z: aabb.min.z,
        };
        let vertex111 = Vec3 {
            x: aabb.max.x,
            y: aabb.max.y,
            z: aabb.max.z,
        };

        // Draw the lines
        self.draw_line(vertex000, vertex100, color);
        self.draw_line(vertex100, vertex101, color);
        self.draw_line(vertex101, vertex001, color);
        self.draw_line(vertex001, vertex000, color);
        self.draw_line(vertex010, vertex110, color);
        self.draw_line(vertex110, vertex111, color);
        self.draw_line(vertex111, vertex011, color);
        self.draw_line(vertex011, vertex010, color);
        self.draw_line(vertex000, vertex010, color);
        self.draw_line(vertex100, vertex110, color);
        self.draw_line(vertex101, vertex111, color);
        self.draw_line(vertex001, vertex011, color);
    }

    pub fn upload_texture(&self, texture: &mut Texture) -> u32 {
        unsafe {
            gl::GenTextures(1, &mut texture.gl_id);
            gl::BindTexture(gl::TEXTURE_2D, texture.gl_id);
            gl::TexImage2D(
                gl::TEXTURE_2D,
                0,
                gl::RGBA8 as i32,
                texture.width as i32,
                texture.height as i32,
                0,
                gl::RGBA,
                gl::UNSIGNED_BYTE,
                texture.data.as_ptr() as *const _,
            );
            gl::GenerateMipmap(gl::TEXTURE_2D);
            gl::TexParameteri(gl::TEXTURE_2D, gl::TEXTURE_MIN_FILTER, gl::NEAREST as i32);
            gl::TexParameteri(gl::TEXTURE_2D, gl::TEXTURE_MAG_FILTER, gl::NEAREST as i32);
        }
        return texture.gl_id;
    }

    pub fn add_sphere(&mut self, sphere: Sphere) {
        self.sphere_queue.push(sphere);
        self.request_reupload = true;
    }

    pub fn add_light(&mut self, light: Light) {
        self.light_queue.push(light);
        self.request_reupload = true;
    }
}

extern "system" fn debug_callback(
    _source: gl::types::GLenum,
    _type: gl::types::GLenum,
    _id: gl::types::GLuint,
    _severity: gl::types::GLenum,
    _length: gl::types::GLsizei,
    message: *const gl::types::GLchar,
    _user_param: *mut std::ffi::c_void,
) {
    unsafe {
        let error_msg = std::ffi::CStr::from_ptr(message).to_string_lossy();
        println!("OpenGL Error: {}", error_msg);
    }
}