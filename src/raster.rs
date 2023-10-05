use std::{mem::size_of, ffi::c_void};

use glam::{Quat, Vec3};
use memoffset::offset_of;

use crate::{structs::Transform, graphics::{Renderer, LineQueueEntry}};

impl Renderer {
    pub fn end_frame_raster(&mut self) {
        // Enable depth testing
        unsafe {
            gl::Enable(gl::DEPTH_TEST);
            gl::Enable(gl::CULL_FACE);
            gl::UseProgram(self.triangle_shader.as_ref().unwrap().program);
        }

        // Add spheres to render queue
        let model = self
            .models
            .get(&self.primitives_model)
            .unwrap();
        let mesh = &model.meshes[0].1;

        for sphere in &self.sphere_queue {
            unsafe {
                // Bind the vertex buffer
                gl::BindVertexArray(mesh.vao);
                gl::BindBuffer(gl::ARRAY_BUFFER, mesh.vbo);

                // Bind the constant buffer
                gl::BindBufferBase(gl::UNIFORM_BUFFER, 6, self.const_buffer_gpu);
                gl::BindBufferBase(gl::SHADER_STORAGE_BUFFER, 0, self.gpu_lights);
                
                // Bind the texture
                gl::Uniform1i(0, 0);
                gl::Uniform1i(1, 0);
                gl::Uniform1i(2, 0);
                gl::Uniform1i(3, 0);
                gl::Uniform1i(4, self.light_queue.len() as i32);

                // Create model matrix for the sphere
                let sphere_trans = Transform {
                    translation: sphere.position,
                    rotation: Quat::IDENTITY,
                    scale: Vec3::ONE * sphere.radius_squared.sqrt(),
                }.local_matrix();
                gl::UniformMatrix4fv(5, 1, gl::FALSE, sphere_trans.as_ref().as_ptr() as *const _);

                // Draw the model
                gl::DrawArrays(gl::TRIANGLES, 0, mesh.verts.len() as _);
            }
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
                gl::BindBufferBase(gl::SHADER_STORAGE_BUFFER, 0, self.gpu_lights);

                // Bind the texture
                gl::BindTexture(gl::TEXTURE0, material.tex_alb as u32);
                gl::BindTexture(gl::TEXTURE1, material.tex_nrm as u32);
                gl::BindTexture(gl::TEXTURE2, material.tex_mtl_rgh as u32);
                gl::BindTexture(gl::TEXTURE3, material.tex_emm as u32);
                gl::Uniform1i(0, material.tex_alb);
                gl::Uniform1i(1, material.tex_nrm);
                gl::Uniform1i(2, material.tex_mtl_rgh);
                gl::Uniform1i(3, material.tex_emm);
                gl::Uniform1i(4, self.light_queue.len() as i32);

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
                gl::UseProgram(self.line_shader.as_ref().unwrap().program);
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
            gl::UseProgram(self.fbo_shader.as_ref().unwrap().program);
            gl::BindTexture(gl::TEXTURE_2D, self.framebuffer_texture);
            gl::BindVertexArray(self.quad_vao);
            gl::DrawArrays(gl::TRIANGLES, 0, 6);
            gl::BindTexture(gl::TEXTURE_2D, 0);
        }
    }
}