use std::{path::Path, fs::File, io::Read};

use gl::types::GLenum;

use crate::graphics::Renderer;

impl Renderer {
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
