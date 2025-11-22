use gl::SampleMaski;
use glam::{vec3, Mat3, Mat4, Quat, Vec2, Vec3, Vec4};
use image::{GenericImageView, Rgba};
use rand::{rng, Rng};
use rapier3d::parry::utils::hashmap::HashMap;

use crate::{camera::Camera, config::emitter_data::EmitterData, enums_types::EmitterName, gl_call, lights::Lights, shaders::Shader};

pub struct Emitter {
    pub positions: Vec<Vec3>,
    pub times_alive: Vec<f32>,
    pub lifetimes: Vec<f32>,
    pub velocities: Vec<Vec3>,
    pub scales: Vec<Vec3>,
    pub rotation_speeds: Vec<f32>,
    pub rotation_offsets: Vec<f32>,

    pub count: usize,
    pub alive: bool,
    pub emit_type: String,

    pub pps: usize,
    pub emit_accumulator: f32,
    pub origin: Vec3,
    pub texture: Option<u32>,
    pub instance_vbo: u32,
    pub alphas: Vec<f32>,
    pub alpha_vbo: u32,
    pub color_vbo: u32,
    pub colors: Vec<Vec4>,
    pub gravity: f32,
}

impl Emitter {
    pub fn new() -> Self {
        let mut instance_vbo = 0;
        let mut alpha_vbo = 0;
        let mut color_vbo = 0;
        unsafe {
            gl_call!(gl::GenBuffers(1, &mut instance_vbo));
            gl_call!(gl::GenBuffers(1, &mut alpha_vbo));
            gl_call!(gl::GenBuffers(1, &mut color_vbo));
        }
        Self {
            positions: vec![],
            times_alive: vec![],
            lifetimes: vec![],
            velocities: vec![],
            scales: vec![],

            rotation_speeds: vec![],
            rotation_offsets: vec![],
            count: 0,
            alive: true,
            emit_type: "".to_string(),

            pps: 0,
            emit_accumulator: 0.0,
            origin: Vec3::splat(1.0),
            texture: None,
            instance_vbo,
            alphas: vec![],
            alpha_vbo,
            color_vbo,
            colors: vec![],
            gravity: 0.0,
        }
    }

    pub fn render(&mut self, shader: &mut Shader, camera: &Camera, vao: u32) {
        unsafe {
            gl::Enable(gl::BLEND);
            gl::BlendFunc(gl::SRC_ALPHA, gl::ONE_MINUS_SRC_ALPHA);
            gl_call!(gl::DepthMask(gl::FALSE));
        }

        // TODO: add growth back in for smoke based ones

        shader.activate();

        let mut rng = rng();

        let mut matrices = Vec::with_capacity(self.count);

        for i in 0..self.count {
            let t = self.times_alive[i];
            let t_norm = (t / self.lifetimes[i]).clamp(0.0, 1.0);

            self.alphas[i] = (0.9 - t_norm).clamp(0.0, 1.0);
            
            // grow over time
            let growth = 1.0 + t_norm * 0.5;
            let scale = self.scales[i] * growth;
            let rotation = self.rotation_offsets[i] + self.rotation_speeds[i] * t;

            let view = camera.view;
            let view_rot = Mat3::from_cols(
                view.x_axis.truncate(),
                view.y_axis.truncate(),
                view.z_axis.truncate(),
            );
            let inv_view_rot = view_rot.transpose();
            let model_rot = Mat4::from_mat3(inv_view_rot);
            let model = Mat4::from_translation(self.positions[i]) * model_rot * Mat4::from_scale(scale);
            //let model = Mat4::from_scale_rotation_translation(scale, , self.positions[i]);

            matrices.push(model);
        }

        unsafe {
            gl_call!(gl::BindBuffer(gl::ARRAY_BUFFER, self.alpha_vbo));
            gl_call!(gl::BufferData(
                gl::ARRAY_BUFFER,
                (self.alphas.len() * std::mem::size_of::<f32>()) as isize,
                self.alphas.as_ptr() as *const _,
                gl::STREAM_DRAW,
            ));
            gl_call!(gl::BindBuffer(gl::ARRAY_BUFFER, self.color_vbo));
            gl_call!(gl::BufferData(
                gl::ARRAY_BUFFER,
                (self.colors.len() * std::mem::size_of::<Vec4>()) as isize,
                self.colors.as_ptr() as *const _,
                gl::STREAM_DRAW,
            ));
            gl_call!(gl::BindBuffer(gl::ARRAY_BUFFER, self.instance_vbo));
            gl_call!(gl::BufferData(
                gl::ARRAY_BUFFER,
                (matrices.len() * std::mem::size_of::<Mat4>()) as isize,
                matrices.as_ptr() as *const _,
                gl::STREAM_DRAW,
            ));
        }

        shader.activate();
        shader.set_mat4("view", camera.view);
        shader.set_mat4("projection", camera.projection);
        shader.set_bool("has_tex", self.texture.is_some());

        if let Some(texture) = self.texture {
            unsafe {
                gl_call!(gl::ActiveTexture(gl::TEXTURE0));
                gl_call!(gl::BindTexture(gl::TEXTURE_2D, texture));
            }
            shader.set_int("texture1", 0);
        }

        unsafe {
            gl::Enable(gl::BLEND);
            gl::BlendFunc(gl::SRC_ALPHA, gl::ONE_MINUS_SRC_ALPHA);
            gl_call!(gl::DepthMask(gl::FALSE));

            gl_call!(gl::BindVertexArray(vao));
            gl_call!(gl::BindBuffer(gl::ARRAY_BUFFER, self.instance_vbo));

            let vec4_size = std::mem::size_of::<f32>() * 4;
            for i in 0..4 {
                let location = 2 + i;
                gl_call!(gl::EnableVertexAttribArray(location));
                gl_call!(gl::VertexAttribPointer(
                    location,
                    4,
                    gl::FLOAT,
                    gl::FALSE,
                    std::mem::size_of::<Mat4>() as i32,
                    (i * vec4_size as u32) as *const std::ffi::c_void,
                ));
                gl_call!(gl::VertexAttribDivisor(location, 1));
            }

            // Setup alpha attribute (location = 6)
            gl_call!(gl::BindBuffer(gl::ARRAY_BUFFER, self.alpha_vbo));
            gl_call!(gl::EnableVertexAttribArray(6));
            gl_call!(gl::VertexAttribPointer(
                6,
                1,
                gl::FLOAT,
                gl::FALSE,
                std::mem::size_of::<f32>() as i32,
                std::ptr::null(),
            ));
            gl_call!(gl::VertexAttribDivisor(6, 1));

            // Color attribute location (location = 7)
            gl_call!(gl::BindBuffer(gl::ARRAY_BUFFER, self.color_vbo));
            gl_call!(gl::EnableVertexAttribArray(7));
            gl_call!(gl::VertexAttribPointer(
                7,
                4,
                gl::FLOAT,
                gl::FALSE,
                std::mem::size_of::<Vec4>() as i32,
                std::ptr::null(),
            ));
            gl_call!(gl::VertexAttribDivisor(7, 1));
            // gl_call!(gl::DrawArrays(gl::TRIANGLES, 0, 6));
            // gl::Disable(gl::CULL_FACE);
            // gl::Disable(gl::DEPTH_TEST);
            gl_call!(gl::DrawArraysInstanced(gl::TRIANGLES, 0, 6, self.count as i32));
            gl_call!(gl::BindVertexArray(0));
            //                     gl::Enable(gl::CULL_FACE);
            //                     gl::Enable(gl::DEPTH_TEST);

            gl_call!(gl::BindTexture(gl::TEXTURE_2D, 0));
            gl_call!(gl::DepthMask(gl::TRUE));
        }

        shader.set_bool("has_tex", false);
        unsafe {
            gl_call!(gl::BindVertexArray(0));
            gl_call!(gl::BindBuffer(gl::ARRAY_BUFFER, 0));
            gl_call!(gl::ActiveTexture(gl::TEXTURE0));
            gl_call!(gl::BindTexture(gl::TEXTURE_2D, 0));

            gl_call!(gl::Disable(gl::BLEND));
            gl_call!(gl::DepthMask(gl::TRUE));
            gl_call!(gl::DepthFunc(gl::LESS));
            gl_call!(gl::Enable(gl::DEPTH_TEST));
        }
    }
}


pub struct ParticleSystem {
    pub emitters: Vec<Emitter>,
    pub vao: u32,
    pub emitter_data: EmitterData,
}

impl ParticleSystem {
    pub fn new(ed_file: &str) -> Self {
        let mut vao = 0;
        let mut vbo = 0;

        let quad_vertices: [f32; 30] = [
            // coords            // tex_coords
            -1.0,  1.0, 0.0,     0.0, 1.0,
            -1.0, -1.0, 0.0,     0.0, 0.0,
            1.0, -1.0, 0.0,     1.0, 0.0,

            -1.0,  1.0, 0.0,     0.0, 1.0,
            1.0, -1.0, 0.0,     1.0, 0.0,
            1.0,  1.0, 0.0,     1.0, 1.0,
        ];

        unsafe {
            gl_call!(gl::GenVertexArrays(1, &mut vao));
            gl_call!(gl::GenBuffers(1, &mut vbo));
            gl_call!(gl::BindVertexArray(vao));

            gl_call!(gl::BindBuffer(gl::ARRAY_BUFFER, vbo));
            gl_call!(gl::BufferData(
                gl::ARRAY_BUFFER,
                (quad_vertices.len() * std::mem::size_of::<f32>()) as isize,
                quad_vertices.as_ptr() as *const _,
                gl::STATIC_DRAW,
            ));

            let stride = (5 * std::mem::size_of::<f32>()) as i32;

            gl_call!(gl::EnableVertexAttribArray(0));
            gl_call!(gl::VertexAttribPointer(0, 3, gl::FLOAT, gl::FALSE, stride, std::ptr::null()));

            gl_call!(gl::EnableVertexAttribArray(1));
            gl_call!(gl::VertexAttribPointer(1, 2, gl::FLOAT, gl::FALSE, stride, (3 * std::mem::size_of::<f32>()) as *const _));

            // Unbind
            gl_call!(gl::BindBuffer(gl::ARRAY_BUFFER, 0));
            gl_call!(gl::BindVertexArray(0));
        }

        let emitter_data = EmitterData::load_from_file(ed_file);

        Self {
            emitters: Vec::new(),
            vao,
            emitter_data,
        }
    }

    pub fn spawn_oneshot_emitter(
        &mut self, 
        emitter_name: &str,
        origin: Vec3,
    ) {
        let mut rng = rng();

        let ed = match self.emitter_data.one_shot_data.get(emitter_name) {
            Some(ed) => ed,
            None     => { 
                eprintln!("WARNING: no emitter found for type {}", emitter_name);
                return;
            }
        };

        let mut emitter = Emitter::new();

        emitter.texture = ed.texture;

        emitter.origin = origin;
        emitter.gravity = ed.gravity;

        for _ in 0..ed.particle_count {
            let angle = rng.random_range(0.0..std::f32::consts::TAU);
            let radius = if ed.radius_rand.x == ed.radius_rand.y {
                ed.radius_rand.x
            } else { 
                rng.random_range(ed.radius_rand.x..ed.radius_rand.y)
            };

            let x = radius * angle.cos();
            let z = radius * angle.sin();

            let position = origin + vec3(x, 0.0, z);

            let outward = vec3(x, 0.0, z).normalize();
            let upward = vec3(0.0, rng.random_range(0.0..2.0), 0.0);
            let velocity = (outward + upward).normalize() * rng.random_range(1.0..3.0);

            let lifetime = rng.random_range(ed.particle_lifetime.x..=ed.particle_lifetime.y);
            let scale = Vec3::splat(rng.random_range(ed.particle_scale.x..=ed.particle_scale.y));

            // color randomization
            let color = if ed.colors.len() > 1 {
                ed.colors[rng.random_range(0..ed.colors.len())]
            } else {
                ed.colors[0]
            };

            emitter.positions.push(position);
            emitter.velocities.push(velocity);
            emitter.colors.push(color);
            emitter.lifetimes.push(lifetime);
            emitter.scales.push(scale);
            emitter.times_alive.push(0.0);
            emitter.rotation_speeds.push(0.0);
            emitter.rotation_offsets.push(0.0);
            emitter.alphas.push(1.0);
        }

        emitter.count = ed.particle_count;
        // emitter.colors = ed.colors.clone();
        self.emitters.push(emitter);
    }

    pub fn spawn_continuous_emitter(&mut self, pps: usize, origin: Vec3, emit_type: &str, texture_path: Option<&str>) {
        let mut emitter = Emitter::new();

        unsafe {
            if let Some(texture_path) = texture_path {
                let mut tex = 0;

                gl_call!(gl::GenTextures(1, &mut tex));
                gl_call!(gl::BindTexture(gl::TEXTURE_2D, tex));
                gl_call!(gl::TexParameteri(gl::TEXTURE_2D, gl::TEXTURE_MIN_FILTER, gl::LINEAR as i32));
                gl_call!(gl::TexParameteri(gl::TEXTURE_2D, gl::TEXTURE_MAG_FILTER, gl::LINEAR as i32));
                gl_call!(gl::TexParameteri(gl::TEXTURE_2D, gl::TEXTURE_WRAP_S, gl::CLAMP_TO_EDGE as i32));
                gl_call!(gl::TexParameteri(gl::TEXTURE_2D, gl::TEXTURE_WRAP_T, gl::CLAMP_TO_EDGE as i32));

                let img = match image::open(texture_path) {
                    Ok(img) => img,
                    _ => panic!("error opening smoke texture"),
                };

                let (img_width, img_height) = img.dimensions();
                let rgba = img.to_rgba8();
                let raw = rgba.as_raw();

                gl_call!(gl::TexImage2D(
                    gl::TEXTURE_2D,
                    0,
                    gl::RGBA8 as i32,
                    img_width as i32,
                    img_height as i32,
                    0,
                    gl::RGBA,
                    gl::UNSIGNED_BYTE,
                    raw.as_ptr().cast(),
                ));

                emitter.texture = Some(tex);
            }
        }

        emitter.pps = pps;
        emitter.origin = origin;
        emitter.emit_type = emit_type.to_string();
        self.emitters.push(emitter);
    }

    pub fn update(&mut self, dt: f32) {
        for emitter in self.emitters.iter_mut() {
            let gravity = Vec3::new(0.0, emitter.gravity, 0.0);
            if emitter.pps > 0 {
                emitter.emit_accumulator += dt;
                let seconds_per_particle = 1.0 / emitter.pps as f32;

                while emitter.emit_accumulator >= seconds_per_particle {
                    emitter.emit_accumulator -= seconds_per_particle;
                    Self::spawn_particle(emitter);
                }
            }
            let mut i = 0;
            while i < emitter.count {
                if emitter.times_alive[i] >= emitter.lifetimes[i] {
                    let last = emitter.count - 1;

                    emitter.positions.swap(i, last);
                    emitter.times_alive.swap(i, last);
                    emitter.lifetimes.swap(i, last);
                    emitter.velocities.swap(i, last);
                    emitter.rotation_speeds.swap(i, last);
                    emitter.rotation_offsets.swap(i, last);
                    emitter.alphas.swap(i, last);

                    emitter.count -= 1;
                } else {
                    // let t_norm = (emitter.times_alive[i] / emitter.lifetimes[i]).clamp(0.0, 1.0);

                    // let velocity_scale = 1.0 - t_norm * t_norm;
                    // emitter.velocities[i] *= velocity_scale;

                    // emitter.velocities[i] += gravity * dt;
                    // emitter.positions[i] += emitter.velocities[i] * dt;
                    // emitter.times_alive[i] += dt;

                    // if emitter.positions[i].y <= 0.0 {
                    //     emitter.positions[i].y = 0.0;
                    //     emitter.velocities[i].y *= -0.3;

                    //     let friction = 0.9; // Lower = more friction
                    //     emitter.velocities[i].x *= friction;
                    //     emitter.velocities[i].z *= friction;
                    // }

                    // i += 1;

                    let prev_t_norm = (emitter.times_alive[i] / emitter.lifetimes[i]).clamp(0.0, 1.0);
                    emitter.times_alive[i] += dt;
                    let t_norm = (emitter.times_alive[i] / emitter.lifetimes[i]).clamp(0.0, 1.0);

                    //let ease_out_cubic = |u: f32| 1.0 - (1.0 - u).powf(3.0);
                    //let delta = ease_out_cubic(t_norm) - ease_out_cubic(prev_t_norm);

                    emitter.velocities[i] += gravity * dt;
                    emitter.positions[i] += emitter.velocities[i] * dt;
                    //emitter.positions[i] += emitter.velocities[i] * dt;
                i += 1;
                }
            }

            if emitter.count == 0 && emitter.pps == 0 {
                emitter.alive = false;
            }
        }

        self.emitters.retain(|e| e.alive);
    }

    pub fn spawn_particle(emitter: &mut Emitter) {
        let mut rng = rng();
        let angle = rng.random_range(0.0..std::f32::consts::TAU);
        let radius = rng.random_range(0.2..30.0);

        let x = radius * angle.cos();
        let z = radius * angle.sin();
        let position = emitter.origin;

        let outward = vec3(x, 0.0, z).normalize_or_zero();
        let upward = vec3(0.0, rng.random_range(-20.0..20.0), 0.0);
        let velocity = outward * rng.random_range(2.0..30.0) + upward;

        let lifetime = rng.random_range(6.0..15.0);
        let scale = Vec3::splat(rng.random_range(1.0..3.0));

        let rotation_speed = rng.random_range(-3.0..3.0); // Radians per second
        let rotation_offset = rng.random_range(0.0..std::f32::consts::TAU);

        // TODO: Instead allocate the right size at the beginning by multiplying the particles per second by the lifetimes
        if emitter.count < emitter.positions.len() {
            let i = emitter.count;
            emitter.positions[i] = position;
            emitter.velocities[i] = velocity;
            emitter.lifetimes[i] = lifetime;
            emitter.scales[i] = scale;
            emitter.times_alive[i] = 0.0;
            emitter.rotation_speeds[i] = rotation_speed;
            emitter.rotation_offsets[i] = rotation_offset;
            emitter.alphas[i] = 0.4;
        } else {
            emitter.positions.push(position);
            emitter.velocities.push(velocity);
            emitter.lifetimes.push(lifetime);
            emitter.scales.push(scale);
            emitter.times_alive.push(0.0);
            emitter.rotation_speeds.push(rotation_speed);
            emitter.rotation_offsets.push(rotation_offset);
            emitter.alphas.push(0.4);
        }
        emitter.count += 1;
    }

    pub fn render(&mut self, shader: &mut Shader, camera: &Camera) {
        for emitter in self.emitters.iter_mut() {
            emitter.render(shader, camera, self.vao);
        }
    }

}
