use glam::Mat3;

use crate::graphics::Renderer;

impl Renderer {
    pub fn end_frame_raytrace_gpu(&mut self) {
        // Enable depth testing
        unsafe {
            gl::Enable(gl::DEPTH_TEST);
            gl::Enable(gl::CULL_FACE);
            gl::UseProgram(self.raytracing_shader.as_ref().unwrap().gl_id);
        }

        // Calculate camera rotation matrix
        let camera_rot_mat = Mat3::from_euler(
            glam::EulerRot::XYZ,
            -self.camera_rotation_euler.x,
            -self.camera_rotation_euler.y,
            -self.camera_rotation_euler.z,
        );

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
            gl::BindTexture(gl::TEXTURE_2D, self.framebuffer_texture);
            gl::BindVertexArray(self.quad_vao);
            gl::DrawArrays(gl::TRIANGLES, 0, 6);
            gl::BindTexture(gl::TEXTURE_2D, 0);
        }
    }
}