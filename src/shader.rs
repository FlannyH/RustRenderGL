use std::{path::Path, fs::File, io::Read, time::SystemTime};

use gl::types::GLenum;

use crate::graphics::Renderer;

pub enum ProgramType {
    Graphics,
    Compute,
}

pub struct ShaderProgram {
    pub shaders: Vec::<ShaderStage>,
    pub path: String,
    pub gl_id: u32,
    pub program_type: ProgramType,
}

pub struct ShaderStage {
    file: File,
    last_modified: u64,
    shader_type: GLenum,
}

impl ShaderProgram {
    pub fn load_shader(path: &Path) -> Result<ShaderProgram, &str> {
        // Create shader program object
        let mut program = ShaderProgram {
            shaders: Vec::new(),
            gl_id: 0,
            path: String::from(path.to_str().unwrap()),
            program_type: ProgramType::Graphics,
        };
        unsafe {
            program.gl_id = gl::CreateProgram();
        }

        // Load and compile shader parts
        load_shader_part_from_path(
            gl::VERTEX_SHADER,
            path.with_extension("vert").as_path(),
            &mut program,
        );
        load_shader_part_from_path(
            gl::FRAGMENT_SHADER,
            path.with_extension("frag").as_path(),
            &mut program,
        );
        unsafe {
            gl::LinkProgram(program.gl_id);
        }

        Ok(program)
    }
    
    pub fn load_shader_compute(path: &Path) -> Result<ShaderProgram, &str> {
        let mut shader = ShaderProgram {
            shaders: Vec::new(),
            gl_id: 0,
            path: String::from(path.to_str().unwrap()),
            program_type: ProgramType::Compute,
        };
        unsafe {
            shader.gl_id = gl::CreateProgram();
        }

        // Load and compile shader parts
        load_shader_part_from_path(
            gl::COMPUTE_SHADER,
            path.with_extension("comp").as_path(),
            &mut shader,
        );
        unsafe {
            gl::LinkProgram(shader.gl_id);
        }

        Ok(shader)
    }

    pub fn hot_reload_on_change(&mut self) {
        let mut should_change = false;

        // Check if the file has been modified since the last time it was loaded
        for shader in &mut self.shaders {
            let curr_modified = shader.file.metadata().unwrap().modified().unwrap().duration_since(SystemTime::UNIX_EPOCH).unwrap().as_secs();
            if curr_modified != shader.last_modified {
                should_change = true;
                shader.last_modified = curr_modified;
                break;
            }
        }

        // If so, create a new shader program, and schedule the old one for deletion
        if should_change {
            let new_shader = match self.program_type {
                ProgramType::Graphics => Self::load_shader(Path::new(&self.path)),
                ProgramType::Compute => Self::load_shader_compute(Path::new(&self.path)),
            }.unwrap();

            self.shaders.clear();
            unsafe {gl::DeleteProgram(self.gl_id)} // todo: check if this is safe
            self.gl_id = new_shader.gl_id;
            self.shaders = new_shader.shaders;
        }
    }
}

impl Renderer {
}

fn load_shader_part_from_path(shader_type: GLenum, path: &Path, program: &mut ShaderProgram) {
    println!("Opening file {path:?}");
    let mut source = File::open(path).expect("Failed to open shader file");
    let last_modified = source.metadata().unwrap().modified().unwrap().duration_since(SystemTime::UNIX_EPOCH).unwrap().as_secs();
    load_shader_part_from_file(shader_type, &mut source, program);
    program.shaders.push(ShaderStage { 
        file: source, 
        last_modified,
        shader_type, 
    });
}

fn load_shader_part_from_file(shader_type: GLenum, file: &mut File, shader: &mut ShaderProgram) {
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
        gl::AttachShader(shader.gl_id, shader_part);
    }
}
