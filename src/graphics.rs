use gl::types::GLenum;
use glam::{Mat3, Mat4, Quat, Vec3, Vec4, Vec2};
use glfw::{Context, Glfw, Window, WindowEvent};
use memoffset::offset_of;
use std::hash::Hash;
use std::sync::Arc;
use std::{
    collections::{hash_map::DefaultHasher, HashMap},
    ffi::c_void,
    fs::File,
    hash::Hasher,
    io::Read,
    mem::size_of,
    path::Path,
    ptr::null,
    sync::mpsc::Receiver,
};

use crate::aabb::AABB;
use crate::bvh::{Bvh, BvhNode};
use crate::material::Material;
use crate::mesh::Mesh;
use crate::ray::{HitInfoExt, Ray};
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
    glfw: Glfw,
    window: Window,
    events: Receiver<(f64, WindowEvent)>,
    depth_buffer_texture: u32,
    framebuffer_texture: u32,
    framebuffer_object: u32,
    quad_vbo: u32,
    quad_vao: u32,
    fbo_shader: u32,
    window_resolution_prev: [i32; 2],
    pub mode: RenderMode,

    // Resources
    pub models: HashMap<u64, Model>,

    // Mesh render queue
    mesh_queue: Vec<MeshQueueEntry>,
    line_queue: Vec<LineQueueEntry>,

    // Main triangle shader
    triangle_shader: u32,
    line_shader: u32,

    // Primitives
    gpu_spheres: u32,
    sphere_queue: Vec<Sphere>,

    // Raytracing stuff
    raytracing_shader: u32,
    framebuffer_cpu: Vec<Pixel32>,
    framebuffer_cpu_to_gpu: u32,

    // Camera
    camera_position: Vec3,
    camera_rotation_euler: Vec3,
    fov: f32, // in radians
    aspect_ratio: f32,
    viewport_height: f32,
    viewport_width: f32,
    viewport_depth: f32,

    // Constant buffers
    const_buffer_cpu: GlobalConstBuffer,
    const_buffer_gpu: u32,
}

#[derive(Clone)]
pub struct MeshQueueEntry {
    mesh: Arc<Mesh>,
    material: Arc<Material>,
}

#[derive(Clone)]
pub struct LineQueueEntry {
    position: Vec3,
    color: Vec4,
}

pub struct GlobalConstBuffer {
    view_projection_matrix: Mat4,
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
            fbo_shader: 0,
            window_resolution_prev: [0, 0],
            mode: RenderMode::RaytracedCPU,
            models: HashMap::new(),
            mesh_queue: vec![],
            line_queue: vec![],
            triangle_shader: 0,
            line_shader: 0,
            raytracing_shader: 0,
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
        };

        // Set FOV
        renderer.fov = 90.0_f32.to_radians();
        renderer.aspect_ratio = width as f32 / height as f32;
        renderer.viewport_height = (renderer.fov * 0.5).tan();
        renderer.viewport_width = renderer.viewport_height * renderer.aspect_ratio;

        // Load shaders
        renderer.fbo_shader = renderer
            .load_shader(Path::new("assets/shaders/fbo"))
            .expect("Shader loading failed");
        renderer.triangle_shader = renderer
            .load_shader(Path::new("assets/shaders/lit"))
            .expect("Shader loading failed!");
        renderer.line_shader = renderer
            .load_shader(Path::new("assets/shaders/line"))
            .expect("Shader loading failed!");
        renderer.raytracing_shader = renderer
            .load_shader_compute(Path::new("assets/shaders/ray.comp"))
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
        self.line_queue.clear();
        self.mesh_queue.clear();
        self.sphere_queue.clear();
    }

    pub fn end_frame(&mut self) {
        match self.mode {
            RenderMode::None => {}
            RenderMode::Rasterized => self.end_frame_raster(),
            RenderMode::RaytracedCPU => self.end_frame_raytrace_cpu(),
            RenderMode::RaytracedGPU => self.end_frame_raytrace_gpu(),
        }

        // Swap front and back buffers
        self.window.swap_buffers();
    }

    pub fn end_frame_raster(&mut self) {
        // Enable depth testing
        unsafe {
            gl::Enable(gl::DEPTH_TEST);
            gl::Enable(gl::CULL_FACE);
            gl::UseProgram(self.triangle_shader);
        }

        // Render mesh queue
        for entry in &self.mesh_queue {
            let mesh = &*entry.mesh;
            let material = &*entry.material;
            // Render the first mesh in the queue
            unsafe {
                // Bind the vertex buffer
                gl::BindVertexArray(mesh.vao);
                gl::BindBuffer(gl::ARRAY_BUFFER, mesh.vbo);

                // Bind the constant buffer
                gl::BindBufferBase(gl::UNIFORM_BUFFER, 0, self.const_buffer_gpu);

                // Bind the texture
                gl::BindTexture(gl::TEXTURE0, material.tex_alb as u32);
                gl::BindTexture(gl::TEXTURE1, material.tex_nrm as u32);
                gl::BindTexture(gl::TEXTURE2, material.tex_mtl_rgh as u32);
                gl::BindTexture(gl::TEXTURE3, material.tex_emm as u32);

                // Draw the model
                gl::DrawArrays(gl::TRIANGLES, 0, mesh.verts.len() as _);
            }
        }

        // Render line queue
        if !self.line_queue.is_empty() {
            unsafe {
                // Create GPU buffers
                let mut vao = 0;
                let mut vbo = 0;
                gl::UseProgram(self.line_shader);
                gl::GenVertexArrays(1, &mut vao);
                gl::GenBuffers(1, &mut vbo);

                // Bind GPU buffers
                gl::BindVertexArray(vao);
                gl::BindBuffer(gl::ARRAY_BUFFER, vbo);

                // Define vertex layout
                gl::VertexAttribPointer(
                    0,
                    3,
                    gl::FLOAT,
                    gl::FALSE,
                    size_of::<LineQueueEntry>() as i32,
                    offset_of!(LineQueueEntry, position) as *const _,
                );
                gl::VertexAttribPointer(
                    1,
                    4,
                    gl::FLOAT,
                    gl::TRUE,
                    size_of::<LineQueueEntry>() as i32,
                    offset_of!(LineQueueEntry, color) as *const _,
                );

                // Enable each attribute
                gl::EnableVertexAttribArray(0);
                gl::EnableVertexAttribArray(1);

                // Populate vertex buffer
                gl::BufferData(
                    gl::ARRAY_BUFFER,
                    (size_of::<LineQueueEntry>() * self.line_queue.len()) as isize,
                    &self.line_queue[0] as *const LineQueueEntry as *const c_void,
                    gl::STATIC_DRAW,
                );

                gl::DrawArrays(gl::LINES, 0, self.line_queue.len() as _);

                // Unbind buffer
                gl::BindVertexArray(0);
                gl::BindBuffer(gl::ARRAY_BUFFER, 0);
            }
        }

        // Render to window buffer
        unsafe {
            gl::BindFramebuffer(gl::FRAMEBUFFER, 0);
            gl::Viewport(
                0,
                0,
                self.window_resolution_prev[0],
                self.window_resolution_prev[1],
            );
            gl::Disable(gl::DEPTH_TEST);
            gl::Disable(gl::CULL_FACE);
            gl::UseProgram(self.fbo_shader);
            gl::BindTexture(gl::TEXTURE_2D, self.framebuffer_texture);
            gl::BindVertexArray(self.quad_vao);
            gl::DrawArrays(gl::TRIANGLES, 0, 6);
            gl::BindTexture(gl::TEXTURE_2D, 0);
        }
    }

    fn end_frame_raytrace_cpu(&mut self) {
        // Enable depth testing
        unsafe {
            gl::Enable(gl::DEPTH_TEST);
            gl::Enable(gl::CULL_FACE);
            gl::UseProgram(self.triangle_shader);
        }

        // Loop over every pixel
        let mut resolution = self.window.get_framebuffer_size();
        resolution.0 /= 1;
        resolution.1 /= 1;
        for y in 0..resolution.1 {
            for x in 0..resolution.0 {
                // Get UV coordinates from the X, Y position on screen
                let u = ((x as f32 / resolution.0 as f32) * 2.0) - 1.0;
                let v = ((y as f32 / resolution.1 as f32) * 2.0) - 1.0;

                // Get the ray direction from the UV coordinates
                let rot = Quat::from_euler(
                    glam::EulerRot::ZYX,
                    self.camera_rotation_euler.z,
                    self.camera_rotation_euler.y,
                    self.camera_rotation_euler.x,
                );
                let forward_vec = rot
                    .mul_vec3(Vec3 {
                        x: self.viewport_width * u,
                        y: self.viewport_height * v,
                        z: self.viewport_depth,
                    })
                    .normalize();

                // Fill the screen with the ray direction
                self.framebuffer_cpu[(x + y * resolution.0) as usize] = Pixel32 {
                    r: ((forward_vec.x) * 255.0).clamp(0.0, 255.0) as u8,
                    g: ((forward_vec.y) * 255.0).clamp(0.0, 255.0) as u8,
                    b: ((forward_vec.z) * 255.0).clamp(0.0, 255.0) as u8,
                    a: 255,
                };

                // Create a ray
                let ray = Ray::new(self.camera_position, forward_vec, None);

                let mut hit_info = HitInfoExt {
                    distance: f32::INFINITY,
                    vertex_interpolated: Vertex {
                        position: Vec3::ZERO,
                        normal: Vec3::ZERO,
                        tangent: Vec4::ZERO,
                        colour: Vec4::ZERO,
                        uv0: Vec2::ZERO,
                        uv1: Vec2::ZERO,
                    },
                };
                // Loop over each mesh in the mesh queue
                for entry in &self.mesh_queue {
                    if let Some(bvh) = &entry.mesh.bvh {
                        let bvh = bvh.as_ref();
                        if let Some(curr_hit_info) = bvh.intersects(&ray) {
                            if (curr_hit_info.distance < hit_info.distance) {
                                hit_info = curr_hit_info;
                            }
                        }
                    }
                }

                // Loop over each sphere in the sphere queue
                for entry in &self.sphere_queue {
                    if let Some(curr_hit_info) = entry.intersects(&ray) {
                        if (curr_hit_info.distance < hit_info.distance && curr_hit_info.distance > 0.0) {
                            hit_info = curr_hit_info;
                        }
                    }
                }
                
                self.framebuffer_cpu[(x + y * resolution.0) as usize] = Pixel32 {
                    r: ((hit_info.vertex_interpolated.normal.x + 1.0) * 127.0) as u8,
                    g: ((hit_info.vertex_interpolated.normal.y + 1.0) * 127.0) as u8,
                    b: ((hit_info.vertex_interpolated.normal.z + 1.0) * 127.0) as u8,
                    a: 255,
                };
            }
        }
        unsafe {
            gl::BindTexture(gl::TEXTURE_2D, self.framebuffer_cpu_to_gpu);
            gl::TexImage2D(
                gl::TEXTURE_2D,
                0,
                gl::RGBA8 as _,
                resolution.0,
                resolution.1,
                0,
                gl::RGBA,
                gl::UNSIGNED_BYTE,
                self.framebuffer_cpu.as_ptr() as _,
            );
            gl::BindTexture(gl::TEXTURE_2D, 0);
        }

        // Render to window buffer
        unsafe {
            gl::BindFramebuffer(gl::FRAMEBUFFER, 0);
            gl::Viewport(
                0,
                0,
                self.window_resolution_prev[0],
                self.window_resolution_prev[1],
            );
            gl::Disable(gl::DEPTH_TEST);
            gl::Disable(gl::CULL_FACE);
            gl::UseProgram(self.fbo_shader);
            gl::BindTexture(gl::TEXTURE_2D, self.framebuffer_cpu_to_gpu);
            gl::BindVertexArray(self.quad_vao);
            gl::DrawArrays(gl::TRIANGLES, 0, 6);
            gl::BindTexture(gl::TEXTURE_2D, 0);
        }
        println!("frame rendered succesfully");
    }

    fn end_frame_raytrace_gpu(&mut self) {
        // Upload spheres to GPU
        unsafe {
            gl::BindBuffer(gl::SHADER_STORAGE_BUFFER, self.gpu_spheres);
            gl::BufferData(gl::SHADER_STORAGE_BUFFER, (self.sphere_queue.len() * std::mem::size_of::<Sphere>()) as isize, self.sphere_queue.as_ptr() as _, gl::STATIC_DRAW);
            gl::BindBuffer(gl::SHADER_STORAGE_BUFFER, 0);
        }

        // Enable depth testing
        unsafe {
            gl::Enable(gl::DEPTH_TEST);
            gl::Enable(gl::CULL_FACE);
            gl::UseProgram(self.raytracing_shader);
        }

        // Calculate camera rotation matrix
        let camera_rot_mat = Mat3::from_euler(
            glam::EulerRot::XYZ,
            -self.camera_rotation_euler.x,
            -self.camera_rotation_euler.y,
            -self.camera_rotation_euler.z,
        );

        // Render mesh queue
        for entry in &self.mesh_queue {
            // Render the first mesh in the queue
            let mesh = entry.mesh.as_ref();
            let bvh = &**mesh.bvh.as_ref().unwrap();
            unsafe {
                gl::BindBufferBase(gl::SHADER_STORAGE_BUFFER, 0, bvh.gpu_nodes);
                gl::BindBufferBase(gl::SHADER_STORAGE_BUFFER, 1, bvh.gpu_indices);
                gl::BindBufferBase(gl::SHADER_STORAGE_BUFFER, 2, bvh.gpu_triangles);
                gl::BindBufferBase(gl::SHADER_STORAGE_BUFFER, 3, self.gpu_spheres);
                gl::BindBufferBase(gl::SHADER_STORAGE_BUFFER, 4, bvh.gpu_counts);
                gl::BindImageTexture(
                    0,
                    self.framebuffer_texture,
                    0,
                    gl::FALSE,
                    0,
                    gl::READ_WRITE,
                    gl::RGBA16F,
                );
                gl::UniformMatrix3fv(0, 1, gl::FALSE, camera_rot_mat.as_ref().as_ptr() as _);
                gl::Uniform3fv(1, 1, self.camera_position.as_ref() as _);
                gl::Uniform1f(2, self.viewport_width as _);
                gl::Uniform1f(3, self.viewport_height as _);
                gl::Uniform1f(4, self.viewport_depth as _);
                gl::Uniform1i(5, self.sphere_queue.len() as _);
                gl::DispatchCompute(
                    self.window_resolution_prev[0] as _,
                    self.window_resolution_prev[1] as _,
                    1,
                );
                gl::MemoryBarrier(gl::SHADER_IMAGE_ACCESS_BARRIER_BIT);
            }
        }
        if self.mesh_queue.len() == 0 {
            unsafe {
                gl::BindBufferBase(gl::SHADER_STORAGE_BUFFER, 3, self.gpu_spheres);
                gl::BindImageTexture(
                    0,
                    self.framebuffer_texture,
                    0,
                    gl::FALSE,
                    0,
                    gl::READ_WRITE,
                    gl::RGBA16F,
                );
                gl::UniformMatrix3fv(0, 1, gl::FALSE, camera_rot_mat.as_ref().as_ptr() as _);
                gl::Uniform3fv(1, 1, self.camera_position.as_ref() as _);
                gl::Uniform1f(2, self.viewport_width as _);
                gl::Uniform1f(3, self.viewport_height as _);
                gl::Uniform1f(4, self.viewport_depth as _);
                gl::Uniform1i(5, self.sphere_queue.len() as _);
                gl::DispatchCompute(
                    self.window_resolution_prev[0] as _,
                    self.window_resolution_prev[1] as _,
                    1,
                );
                gl::MemoryBarrier(gl::SHADER_IMAGE_ACCESS_BARRIER_BIT);
            }   
        }

        // Render compute shader test
        let resolution = self.window.get_framebuffer_size();
        unsafe {
            gl::UseProgram(self.raytracing_shader);
            gl::BindImageTexture(
                0,
                self.framebuffer_texture,
                0,
                gl::FALSE,
                0,
                gl::READ_WRITE,
                gl::RGBA16F,
            );
            gl::DispatchCompute(resolution.0 as _, resolution.1 as _, 1);
            gl::MemoryBarrier(gl::SHADER_IMAGE_ACCESS_BARRIER_BIT);
        }

        // Render to window buffer
        unsafe {
            gl::BindFramebuffer(gl::FRAMEBUFFER, 0);
            gl::Viewport(
                0,
                0,
                self.window_resolution_prev[0],
                self.window_resolution_prev[1],
            );
            gl::Disable(gl::DEPTH_TEST);
            gl::Disable(gl::CULL_FACE);
            gl::UseProgram(self.fbo_shader);
            gl::BindTexture(gl::TEXTURE_2D, self.framebuffer_texture);
            gl::BindVertexArray(self.quad_vao);
            gl::DrawArrays(gl::TRIANGLES, 0, 6);
            gl::BindTexture(gl::TEXTURE_2D, 0);
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
            let _new_name = format!("{}::materials/{}/albedo", path.display(), name);
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
        for (name, mesh) in self.models.get(model_id).unwrap().meshes.clone() {
            self.mesh_queue.push(MeshQueueEntry {
                mesh: Arc::new(mesh),
                material: Arc::new(
                    self.models
                        .get(model_id)
                        .unwrap()
                        .materials
                        .get(&name)
                        .unwrap()
                        .clone(),
                ),
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

    pub fn load_shader_compute(&mut self, path: &Path) -> Result<u32, &str> {
        let program;
        unsafe {
            program = gl::CreateProgram();
        }

        // Load and compile shader parts
        load_shader_part(
            gl::COMPUTE_SHADER,
            path.with_extension("comp").as_path(),
            program,
        );
        unsafe {
            gl::LinkProgram(program);
        }

        Ok(program)
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

    pub fn draw_sphere(&mut self, sphere: Sphere) {
        self.sphere_queue.push(sphere)
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
