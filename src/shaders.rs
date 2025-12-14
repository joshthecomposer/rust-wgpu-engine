#![allow(dead_code, clippy::while_let_on_iterator, clippy::collapsible_if)]
use std::{collections::HashMap, ffi::CString, fs::read_to_string, ptr};

use gl::types::{GLint, GLuint};
use glam::{Mat4, Vec3};

use crate::{
    gl_call,
    lights::{DirLight, PointLight},
};

pub struct Shader {
    pub id: GLuint,
    pub uniform_locations: HashMap<String, GLint>,
}

impl Shader {
    pub fn new(file_path: &str) -> Self {
        let id = init_shader_program(file_path);

        let mut shader = Self {
            id,
            uniform_locations: HashMap::new(),
        };

        shader.parse_and_store_uniforms(file_path);

        shader
    }

    pub fn activate(&self) {
        unsafe { gl_call!(gl::UseProgram(self.id)) }
    }

    fn parse_and_store_uniforms(&mut self, file_path: &str) {
        let data = read_to_string(file_path).unwrap();
        let mut lines = data.lines();

        while let Some(line) = lines.next() {
            let parts: Vec<&str> = line.split_whitespace().collect();

            if parts.is_empty() {
                continue;
            }

            if parts[0] == "uniform" {
                let cleaned_part = parts[2].split('[').collect::<Vec<&str>>()[0].replace(';', "");

                match parts[1] {
                    "Material" => {
                        println!("Setting up uniform Material location: {}", cleaned_part);
                        self.store_material_location(cleaned_part.as_str());
                    }
                    "DirLight" => {
                        println!("Setting up uniform Material location: {}", cleaned_part);
                        self.store_dir_light_location(cleaned_part.as_str());
                    }
                    _ => {
                        println!("Setting up uniform location: {}", cleaned_part);
                        self.store_uniform_location(cleaned_part.as_str());
                    }
                }
            }
        }
    }

    pub fn store_uniform_location(&mut self, name: &str) {
        let c_name = CString::new(name).unwrap();
        let location = unsafe { gl_call!(gl::GetUniformLocation(self.id, c_name.as_ptr())) };
        self.uniform_locations.insert(name.to_string(), location);
    }

    pub fn store_dir_light_location(&mut self, name: &str) {
        self.store_uniform_location(format!("{}.direction", name).as_str());
        self.store_uniform_location(format!("{}.view_pos", name).as_str());
        self.store_uniform_location(format!("{}.ambient", name).as_str());
        self.store_uniform_location(format!("{}.diffuse", name).as_str());
        self.store_uniform_location(format!("{}.specular", name).as_str());
    }

    pub fn store_point_light_location(&mut self, name: &str) {
        self.store_uniform_location(format!("{}.position", name).as_str());
        self.store_uniform_location(format!("{}.ambient", name).as_str());
        self.store_uniform_location(format!("{}.diffuse", name).as_str());
        self.store_uniform_location(format!("{}.specular", name).as_str());
        self.store_uniform_location(format!("{}.constant", name).as_str());
        self.store_uniform_location(format!("{}.linear", name).as_str());
        self.store_uniform_location(format!("{}.quadratic", name).as_str());
    }

    pub fn store_material_location(&mut self, name: &str) {
        self.store_uniform_location(format!("{}.Diffuse", name).as_str());
        self.store_uniform_location(format!("{}.Specular", name).as_str());
        self.store_uniform_location(format!("{}.Emissive", name).as_str());
        self.store_uniform_location(format!("{}.Opacity", name).as_str());
    }

    pub fn get_uniform_location(&self, name: &str) -> GLint {
        *self.uniform_locations.get(name).unwrap_or(&-1)
    }

    pub fn set_vec3(&self, name: &str, value: Vec3) {
        let location = self.get_uniform_location(name);
        if location != -1 {
            unsafe { gl_call!(gl::Uniform3f(location, value.x, value.y, value.z)) }
        }
    }

    pub fn set_mat4(&self, name: &str, value: Mat4) {
        let location = self.get_uniform_location(name);
        if location != -1 {
            unsafe {
                gl_call!(gl::UniformMatrix4fv(
                    location,
                    1,
                    gl::FALSE,
                    value.to_cols_array().as_ptr()
                ))
            }
        }
    }

    pub fn set_bool(&self, name: &str, value: bool) {
        let location = self.get_uniform_location(name);
        if location != -1 {
            let int_value = if value { 1 } else { 0 };
            unsafe { gl_call!(gl::Uniform1i(location, int_value)) };
        }
    }

    pub fn set_int(&self, name: &str, value: u32) {
        let location = self.get_uniform_location(name);
        if location != -1 {
            unsafe { gl_call!(gl::Uniform1i(location, value as i32)) };
        }
    }

    pub fn set_float(&self, name: &str, value: f32) {
        let location = self.get_uniform_location(name);
        if location != -1 {
            unsafe { gl_call!(gl::Uniform1f(location, value)) };
        }
    }

    pub fn set_dir_light(&self, name: &str, value: &DirLight) {
        let direction = self.get_uniform_location(format!("{}.direction", name).as_str());
        let view_pos = self.get_uniform_location(format!("{}.view_pos", name).as_str());
        let ambient = self.get_uniform_location(format!("{}.ambient", name).as_str());
        let diffuse = self.get_uniform_location(format!("{}.diffuse", name).as_str());
        let specular = self.get_uniform_location(format!("{}.specular", name).as_str());

        if direction != -1 || view_pos != -1 || ambient != -1 || diffuse != -1 || specular != -1 {
            unsafe {
                gl_call!(gl::Uniform3f(
                    direction,
                    value.direction.x,
                    value.direction.y,
                    value.direction.z
                ));
                gl_call!(gl::Uniform3f(
                    view_pos,
                    value.view_pos.x,
                    value.view_pos.y,
                    value.view_pos.z
                ));
                gl_call!(gl::Uniform3f(
                    ambient,
                    value.ambient.x,
                    value.ambient.y,
                    value.ambient.z
                ));
                gl_call!(gl::Uniform3f(
                    diffuse,
                    value.diffuse.x,
                    value.diffuse.y,
                    value.diffuse.z
                ));
                gl_call!(gl::Uniform3f(
                    specular,
                    value.specular.x,
                    value.specular.y,
                    value.specular.z
                ));
            }
        }
    }

    pub fn set_point_light(&self, name: &str, value: &PointLight) {
        let position = self.get_uniform_location(format!("{}.position", name).as_str());
        let ambient = self.get_uniform_location(format!("{}.ambient", name).as_str());
        let diffuse = self.get_uniform_location(format!("{}.diffuse", name).as_str());
        let specular = self.get_uniform_location(format!("{}.specular", name).as_str());
        let constant = self.get_uniform_location(format!("{}.constant", name).as_str());
        let linear = self.get_uniform_location(format!("{}.linear", name).as_str());
        let quadratic = self.get_uniform_location(format!("{}.quadratic", name).as_str());

        if position != -1
            || ambient != -1
            || diffuse != -1
            || specular != -1
            || constant != -1
            || linear != -1
            || quadratic != -1
        {
            unsafe {
                gl_call!(gl::Uniform3f(
                    position,
                    value.position.x,
                    value.position.y,
                    value.position.z
                ));
                gl_call!(gl::Uniform3f(
                    ambient,
                    value.ambient.x,
                    value.ambient.y,
                    value.ambient.z
                ));
                gl_call!(gl::Uniform3f(
                    diffuse,
                    value.diffuse.x,
                    value.diffuse.y,
                    value.diffuse.z
                ));
                gl_call!(gl::Uniform3f(
                    specular,
                    value.specular.x,
                    value.specular.y,
                    value.specular.z
                ));
                gl_call!(gl::Uniform1f(constant, value.constant));
                gl_call!(gl::Uniform1f(linear, value.linear));
                gl_call!(gl::Uniform1f(quadratic, value.quadratic));
            }
        }
    }

    pub fn set_mat4_array(&self, name: &str, value: &Vec<Mat4>) {
        let location = self.get_uniform_location(name);
        if location != -1 {
            let mut float_data = Vec::with_capacity(value.len() * 16);

            for mat in value {
                float_data.extend_from_slice(&mat.to_cols_array());
            }

            unsafe {
                gl_call!(gl::UniformMatrix4fv(
                    location,
                    value.len() as i32,
                    gl::FALSE,
                    float_data.as_ptr()
                ));
            }
        }
    }
}

pub fn init_shader_program(file_path: &str) -> u32 {
    let (vs_source, gs_source, fs_source) = extract_shader_sources(file_path);

    let vs_cstr = CString::new(vs_source).expect("Failed to convert vs source to C string");
    let fs_cstr = CString::new(fs_source).expect("Failed to convert fs source to C string");

    unsafe {
        let shader = gl::CreateProgram();

        // Vertex Shader
        let vertex_shader = gl::CreateShader(gl::VERTEX_SHADER);
        gl::ShaderSource(vertex_shader, 1, &vs_cstr.as_ptr(), ptr::null());
        compile_shader(vertex_shader);
        gl::AttachShader(shader, vertex_shader);

        // Fragment Shader
        let fragment_shader = gl::CreateShader(gl::FRAGMENT_SHADER);
        gl::ShaderSource(fragment_shader, 1, &fs_cstr.as_ptr(), ptr::null());
        compile_shader(fragment_shader);
        gl::AttachShader(shader, fragment_shader);

        // optional geometry shader
        let geometry_shader = if let Some(gs_source) = gs_source {
            let gs_cstr = CString::new(gs_source).expect("Failed to convert gs source to C string");
            let geometry_shader = gl::CreateShader(gl::GEOMETRY_SHADER);
            gl_call!(gl::ShaderSource(
                geometry_shader,
                1,
                &gs_cstr.as_ptr(),
                ptr::null()
            ));
            compile_shader(geometry_shader);
            Some(geometry_shader)
        } else {
            None
        };

        gl_call!(gl::LinkProgram(shader));

        gl::DeleteShader(vertex_shader);
        gl::DeleteShader(fragment_shader);

        if let Some(geometry_shader) = geometry_shader {
            gl::DeleteShader(geometry_shader);
        }
        shader
    }
}

fn extract_shader_sources(file_path: &str) -> (String, Option<String>, String) {
    println!("{}", file_path);
    let data = read_to_string(file_path).unwrap();
    let mut lines = data.lines();

    let mut current_shader = None;
    let mut shader_sources = HashMap::new();

    while let Some(line) = lines.next() {
        match line.trim() {
            "// VERTEX_SHADER" => {
                println!("Located vertex shader, extracting now...");
                current_shader = Some("VERTEX_SHADER".to_string());
                shader_sources.insert("VERTEX_SHADER".to_string(), String::new());
            }
            "// FRAGMENT_SHADER" => {
                println!("Located fragment shader, extracting now...");
                current_shader = Some("FRAGMENT_SHADER".to_string());
                shader_sources.insert("FRAGMENT_SHADER".to_string(), String::new());
            }
            "// GEOMETRY_SHADER" => {
                println!("Located geometry shader shader, extracting now...");
                current_shader = Some("FRAGMENT_SHADER".to_string());
                shader_sources.insert("FRAGMENT_SHADER".to_string(), String::new());
            }
            _ => {
                if let Some(ref shader_type) = current_shader {
                    shader_sources.entry(shader_type.clone()).and_modify(|src| {
                        src.push_str(line);
                        src.push('\n');
                    });
                }
            }
        }
    }

    (
        shader_sources.remove("VERTEX_SHADER").unwrap(),
        shader_sources.remove("GEOMETRY_SHADER"),
        shader_sources.remove("FRAGMENT_SHADER").unwrap(),
    )
}

fn compile_shader(input: u32) {
    unsafe {
        gl_call!(gl::CompileShader(input));

        let mut success: i32 = 0;
        let mut info_log = vec![0u8; 512];

        gl_call!(gl::GetShaderiv(input, gl::COMPILE_STATUS, &mut success));

        if success == 0 {
            gl_call!(gl::GetShaderInfoLog(
                input,
                512,
                core::ptr::null_mut(),
                info_log.as_mut_ptr() as *mut i8
            ));
            println!(
                "Problem compiling shader: {:?}",
                String::from_utf8_lossy(&info_log)
            );
        }
    }
}
