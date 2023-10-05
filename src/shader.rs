use std::{path::Path, fs::File, io::Read, collections::HashMap};

use gl::types::GLenum;

use crate::graphics::Renderer;

pub struct Shader {
    pub files: HashMap<String, File>,
    pub program: u32,
}

impl Renderer {
    pub fn load_shader(&mut self, path: &Path) -> Result<Shader, &str> {
        // Create shader program object
        let mut shader = Shader {
            files: HashMap::new(),
            program: 0,
        };
        unsafe {
            shader.program = gl::CreateProgram();
        }

        // Load and compile shader parts
        load_shader_part_from_path(
            gl::VERTEX_SHADER,
            path.with_extension("vert").as_path(),
            &mut shader,
        );
        load_shader_part_from_path(
            gl::FRAGMENT_SHADER,
            path.with_extension("frag").as_path(),
            &mut shader,
        );
        unsafe {
            gl::LinkProgram(shader.program);
        }

        Ok(shader)
    }

    pub fn load_shader_compute(&mut self, path: &Path) -> Result<Shader, &str> {
        let mut shader = Shader {
            files: HashMap::new(),
            program: 0,
        };
        unsafe {
            shader.program = gl::CreateProgram();
        }

        // Load and compile shader parts
        load_shader_part_from_path(
            gl::COMPUTE_SHADER,
            path.with_extension("comp").as_path(),
            &mut shader,
        );
        unsafe {
            gl::LinkProgram(shader.program);
        }

        Ok(shader)
    }
}

fn load_shader_part_from_path(shader_type: GLenum, path: &Path, shader: &mut Shader) {
    let mut source = File::open(path).expect("Failed to open shader file");
    load_shader_part_from_file(shader_type, &mut source, shader);
    shader.files.insert(String::from(path.to_str().unwrap()), source);
}

fn load_shader_part_from_file(shader_type: GLenum, file: &mut File, shader: &mut Shader) {
    // Load shader source
    let mut source = String::new();
    file.read_to_string(&mut source)
        .expect("Failed to read file");
    let source_len = source.len() as i32;

    unsafe {
        // Create shader part
        let shader_part = gl::CreateShader(shader_type);
        gl::ShaderSource(shader_part, 1, &source.as_bytes().as_ptr().cast(), &source_len);
        gl::CompileShader(shader_part);

        // Check for errors
        let mut result = 0;
        let mut log_length = 0;
        gl::GetShaderiv(shader_part, gl::COMPILE_STATUS, &mut result);
        gl::GetShaderiv(shader_part, gl::INFO_LOG_LENGTH, &mut log_length);
        let mut error_message: Vec<u8> = vec![0; log_length as usize];
        gl::GetShaderInfoLog(
            shader_part,
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
        gl::AttachShader(shader.program, shader_part);
    }
}
