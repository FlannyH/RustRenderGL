use glam::{Quat, Vec3, Vec4, Vec2};

use crate::{structs::{Pixel32, Vertex}, ray::{HitInfoExt, Ray}, graphics::Renderer};

impl Renderer {
    pub fn end_frame_raytrace_cpu(&mut self) {
        // Enable depth testing
        unsafe {
            gl::Enable(gl::DEPTH_TEST);
            gl::Enable(gl::CULL_FACE);
            gl::UseProgram(self.triangle_shader.as_ref().unwrap().gl_id);
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
                            if curr_hit_info.distance < hit_info.distance {
                                hit_info = curr_hit_info;
                            }
                        }
                    }
                }

                // Loop over each sphere in the sphere queue
                for entry in &self.sphere_queue {
                    if let Some(curr_hit_info) = entry.intersects(&ray) {
                        if curr_hit_info.distance < hit_info.distance
                            && curr_hit_info.distance > 0.0
                        {
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
            gl::UseProgram(self.fbo_shader.as_ref().unwrap().gl_id);
            gl::BindTexture(gl::TEXTURE_2D, self.framebuffer_cpu_to_gpu);
            gl::BindVertexArray(self.quad_vao);
            gl::DrawArrays(gl::TRIANGLES, 0, 6);
            gl::BindTexture(gl::TEXTURE_2D, 0);
        }
    }
}