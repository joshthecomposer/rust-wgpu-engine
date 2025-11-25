use std::collections::HashSet;

use gl::SampleMaski;
use glam::{vec3, Mat3, Mat4, Quat, Vec2, Vec3, Vec4};
use image::{GenericImageView, Rgba};
use rand::{rng, Rng};
use rapier3d::parry::utils::hashmap::HashMap;

use crate::{camera::Camera, config::emitter_data::{EmitterBlackboard, EmitterData, UiEmitterBlackboard}, enums_types::EmitterName, gl_call, lights::Lights, shaders::Shader};

#[derive(Debug)]
pub struct Emitter {
    pub positions: Vec<Vec3>,
    pub times_alive: Vec<f32>,
    pub lifetimes: Vec<f32>,
    pub velocities: Vec<Vec3>,
    pub rotation_speeds: Vec<f32>,
    pub rotation_offsets: Vec<f32>,

    pub count: usize,
    pub alive: bool,
    pub name: String,

    pub pps: Option<usize>,
    pub emit_accumulator: f32,
    pub origin: Vec3,
    pub texture: Option<u32>,
    pub texture_has_alpha: bool,
    pub instance_vbo: u32,
    pub alpha_vbo: u32,
    pub color_vbo: u32,
    pub colors: Vec<Vec4>,
    pub gravity: f32,

    pub alphas: Vec<f32>,
    pub base_alphas: Vec<f32>,
    pub end_alphas: Vec<f32>,
    pub alpha_powers: Vec<f32>,

    pub scales: Vec<f32>,
    pub base_scales: Vec<f32>,
    pub end_scales: Vec<f32>,
    pub scale_powers: Vec<f32>,

    pub editor_blackboard: Option<EmitterBlackboard>,
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

            rotation_speeds: vec![],
            rotation_offsets: vec![],
            count: 0,
            alive: true,
            name: String::new(),

            pps: None,
            emit_accumulator: 0.0,
            origin: Vec3::splat(1.0),
            texture: None,
            texture_has_alpha: true,
            instance_vbo,
            alpha_vbo,
            color_vbo,
            colors: vec![],
            gravity: 0.0,

            alpha_powers: vec![], // Curve shape
            alphas: vec![], // Current
            base_alphas: vec![], // beginning
            end_alphas: vec![], // end goal

            scale_powers: vec![],
            scales: vec![],
            base_scales: vec![],
            end_scales: vec![],

            editor_blackboard: None,
        }
    }

    pub fn render(&mut self, shader: &mut Shader, camera: &Camera, vao: u32) {
        unsafe {
            gl::Enable(gl::BLEND);
            gl::BlendFunc(gl::SRC_ALPHA, gl::ONE_MINUS_SRC_ALPHA);
            gl_call!(gl::DepthMask(gl::FALSE));
        }

        shader.activate();

        let mut rng = rng();

        let mut matrices = Vec::with_capacity(self.count);
        for i in 0..self.count {
            let t = self.times_alive[i];
            let t_norm = (t / self.lifetimes[i]).clamp(0.0, 1.0);

            // ====================================================
            // ALPHA OVER TIME
            // ====================================================
            let alpha_t = t_norm.powf(self.alpha_powers[i]);
            let start_alpha = self.base_alphas[i];
            let end_alpha = self.end_alphas[i];

            let a = start_alpha + (end_alpha - start_alpha) * alpha_t;
            self.alphas[i] = a.clamp(0.0, 1.0);
            // self.alphas[i] = 1.0;

            // ====================================================
            // GROWTH OVER TIME
            // ====================================================
            let scale_t = t_norm.powf(self.scale_powers[i]);
            let start_factor = self.base_scales[i];
            let end_factor   = self.end_scales[i];
            let factor = start_factor + (end_factor - start_factor) * scale_t;
            let scale = self.scales[i] * factor;

            let rotation = self.rotation_offsets[i] + self.rotation_speeds[i] * t;

            let view = camera.view;
            let view_rot = Mat3::from_cols(
                view.x_axis.truncate(),
                view.y_axis.truncate(),
                view.z_axis.truncate(),
            );
            let inv_view_rot = view_rot.transpose();
            let model_rot = Mat4::from_mat3(inv_view_rot);
            let model = Mat4::from_translation(self.positions[i]) * model_rot * Mat4::from_scale(Vec3::splat(scale));
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
            shader.set_bool("texture_has_alpha", self.texture_has_alpha);
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

// an emitter that is just being previewed in the editor
#[derive(Debug)]
pub struct StagedEmitter {
    pub id: usize,
    pub emitter: Emitter,
}

pub struct ParticleSystem {
    pub emitters: Vec<Emitter>,
    pub vao: u32,
    pub emitter_data: EmitterData,
    pub registered_textures: HashMap<String, u32>,

    pub next_staged_id: usize,
    pub staged_emitters: Vec<StagedEmitter>,
    pub render_staged_emitters: bool,
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

        let mut emitter_data = EmitterData::load_from_file(ed_file);

        let mut registered_textures = HashMap::new();

        for (k, v) in &mut emitter_data.one_shot_data {
            if let Some(path) = &v.texture_path {
                let tex = Self::load_texture(&path);
                registered_textures.insert(path.clone(), tex);
                v.texture_idx = Some(tex);
            }
        }

        Self {
            emitters: vec![],
            vao,
            emitter_data,
            registered_textures,

            next_staged_id: 0,
            staged_emitters: vec![],
            render_staged_emitters: false,
        }
    }

    fn load_texture(path: &str) -> u32 {
        let mut tex = 0;

        println!("FOUND TEXTURE {}", &path);

        unsafe {
            gl_call!(gl::GenTextures(1, &mut tex));
            gl_call!(gl::BindTexture(gl::TEXTURE_2D, tex));
            gl_call!(gl::TexParameteri(gl::TEXTURE_2D, gl::TEXTURE_MIN_FILTER, gl::LINEAR as i32));
            gl_call!(gl::TexParameteri(gl::TEXTURE_2D, gl::TEXTURE_MAG_FILTER, gl::LINEAR as i32));
            gl_call!(gl::TexParameteri(gl::TEXTURE_2D, gl::TEXTURE_WRAP_S, gl::CLAMP_TO_EDGE as i32));
            gl_call!(gl::TexParameteri(gl::TEXTURE_2D, gl::TEXTURE_WRAP_T, gl::CLAMP_TO_EDGE as i32));

            let img = match image::open(path) {
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
        }

        tex
    }

    pub fn spawn_oneshot_editor_emitter(
        &mut self, 
        ed: &EmitterBlackboard,
        origin: Vec3,
    ) -> usize {
        let mut emitter = Emitter::new();

        emitter.texture = match &ed.texture_path {
            Some(path) => {
                if let Some(tex) = self.registered_textures.get(path) {
                    Some(*tex)
                } else {
                    let tex = Self::load_texture(&path);
                    self.registered_textures.insert(path.to_string(), tex);
                    Some(tex)
                }
            },
            None => None,
        };

        emitter.origin = origin;
        emitter.gravity = ed.gravity;

        let desired_dir = if ed.direction.length_squared() > 0.0 {
            ed.direction.normalize()
        } else {
            Vec3::Y // fallback so we don't get NaNs
        };

        emitter.pps = ed.pps;

        Self::calculate_particle_data(ed, origin, Some(desired_dir), &mut emitter);

        emitter.editor_blackboard = Some(ed.clone());

        emitter.name = ed.name.clone();
        emitter.texture_has_alpha = ed.texture_has_alpha; 
        // emitter.colors = ed.colors.clone();
        let current_id = self.next_staged_id;
        let staged_emitter = StagedEmitter {
            id: current_id,
            emitter,
        };
        self.staged_emitters.push(staged_emitter);
        self.next_staged_id += 1;

        current_id
    }

    pub fn spawn_oneshot_emitter(
        &mut self, 
        emitter_name: &str,
        origin: Vec3,
        direction: Option<Vec3>,
    ) {
        let mut emitter = Emitter::new();

        let ed = match self.emitter_data.one_shot_data.get(emitter_name) {
            Some(ed) => ed,
            None => panic!("Could not find emitter type of {}", emitter_name),
        };

        emitter.texture = match &ed.texture_path {
            Some(path) => {
                if let Some(tex) = self.registered_textures.get(path) {
                    Some(*tex)
                } else {
                    let tex = Self::load_texture(&path);
                    self.registered_textures.insert(path.to_string(), tex);
                    Some(tex)
                }
            },
            None => None,
        };

        emitter.origin = origin;
        emitter.gravity = ed.gravity;
        emitter.pps = ed.pps;

        Self::calculate_particle_data(ed, origin, direction, &mut emitter);

        emitter.name = ed.name.clone();
        self.emitters.push(emitter);
    }

    pub fn update(&mut self, dt: f32) {
        let ed = &self.emitter_data;

        for e in self.emitters.iter_mut() {
            Self::update_emitter(e, dt, ed);
        }
        self.emitters.retain(|e| e.alive);

        for se in self.staged_emitters.iter_mut() {
            Self::update_emitter(&mut se.emitter, dt, ed);
        }
        self.staged_emitters.retain(|se| se.emitter.alive);
    }

    fn update_emitter(
        emitter: &mut Emitter,
        dt: f32,
        emitter_data: &EmitterData,
    ) {
        let gravity = Vec3::new(0.0, emitter.gravity, 0.0);

        let def: EmitterBlackboard = if let Some(bb) = &emitter.editor_blackboard {
            bb.clone()
        } else {
            emitter_data
                .one_shot_data
                .get(&emitter.name)
                .expect("missing emitter preset")
                .clone()
        };

        if let Some(pps) = emitter.pps {
            if pps > 0 {
                emitter.emit_accumulator += dt;
                let seconds_per_particle = 1.0 / pps as f32;

                while emitter.emit_accumulator >= seconds_per_particle {
                    emitter.emit_accumulator -= seconds_per_particle;
                    ParticleSystem::spawn_particle(emitter, &def);
                }
            }
        }

        // lifetime / gravity stuff
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
                emitter.base_alphas.swap(i, last);
                emitter.end_alphas.swap(i, last);
                emitter.alpha_powers.swap(i, last);

                emitter.scales.swap(i, last);
                emitter.base_scales.swap(i, last);
                emitter.end_scales.swap(i, last);
                emitter.scale_powers.swap(i, last);

                emitter.colors.swap(i, last);

                emitter.count -= 1;
            } else {
                emitter.times_alive[i] += dt;
                emitter.velocities[i] += gravity * dt;
                emitter.positions[i] += emitter.velocities[i] * dt;
                i += 1;
            }
        }

        if emitter.count == 0 {
            if let Some(pps) = emitter.pps {
                if pps == 0 {
                    emitter.alive = false;
                }
            } else {
                emitter.alive = false;
            }
        }
    }

    pub fn edit_staged_emitter(&mut self, id: usize, ed: &EmitterBlackboard, origin: Vec3) {
        if let Some(se) = self.staged_emitters.iter_mut().find(|se| se.id == id) {
            let emitter = &mut se.emitter;

            emitter.origin = origin;
            emitter.gravity = ed.gravity;
            emitter.name = ed.name.clone();
            emitter.texture_has_alpha = ed.texture_has_alpha;
            emitter.pps = ed.pps;
            emitter.editor_blackboard = Some(ed.clone());

            emitter.texture = match &ed.texture_path {
                Some(path) => {
                    if let Some(tex) = self.registered_textures.get(path) {
                        Some(*tex)
                    } else {
                        let tex = Self::load_texture(path);
                        self.registered_textures.insert(path.to_string(), tex);
                        Some(tex)
                    }
                }
                None => None,
            };
        }
    }

    pub fn spawn_particle(emitter: &mut Emitter, ed: &EmitterBlackboard) {
        let origin = emitter.origin;

        let desired_dir = if ed.direction.length_squared() > 0.0 {
            Some(ed.direction.normalize())
        } else {
            Some(Vec3::Y) // fallback so we don't get NaNs
        };

        Self::calculate_particle_data(ed, origin, desired_dir, emitter);
    }

    pub fn calculate_particle_data(
        ed: &EmitterBlackboard, 
        origin: Vec3, 
        direction: Option<Vec3>,
        emitter: &mut Emitter,
    ) {
        // TODO: Do we want to take the local of particles authored facing other directions or can
        // we just assume we don't need to?
        let local_up = Vec3::Y;

        let desired_dir = if let Some(d) = direction {
            d.normalize()
        } else {
            Vec3::Y
        };

        let rot = Quat::from_rotation_arc(local_up, desired_dir);

        let mut rng = rng();

        // How many particles to spawn this call?
        // - Oneshot emitters (pps == None): spawn all at once
        // - Continuous emitters (pps == Some): spawn exactly 1 per call
        let num_to_spawn = if emitter.pps.is_none() {
            ed.particle_count
        } else {
            1
        };

        for _ in 0..num_to_spawn {
            let rot = Quat::from_rotation_arc(local_up, desired_dir);

            let angle = if ed.angle_rand.x >= ed.angle_rand.y {
                ed.angle_rand.x
            } else {
                rng.random_range(ed.angle_rand.x..=ed.angle_rand.y)
            };

            let radius = if ed.radius_rand.x >= ed.radius_rand.y {
                ed.radius_rand.x
            } else { 
                rng.random_range(ed.radius_rand.x..=ed.radius_rand.y)
            };

            let local_offset = vec3(radius * angle.cos(), 0.0, radius * angle.sin());

            let world_offset = rot * local_offset;
            let position = emitter.origin + world_offset;

            let local_dir = local_offset.normalize_or_zero();

            let radial_speed = if ed.radial_speed.x >= ed.radial_speed.y {
                ed.radial_speed.x
            } else {
                rng.random_range(ed.radial_speed.x..=ed.radial_speed.y)
            };

            let up_speed = if ed.up_speed.x >= ed.up_speed.y {
                ed.up_speed.x
            } else {
                rng.random_range(ed.up_speed.x..=ed.up_speed.y)
            };

            let jitter_amount = if ed.jitter.x >= ed.jitter.y {
                ed.jitter.x
            } else {
                rng.random_range(ed.jitter.x..=ed.jitter.y)
            };

            let jitter_dir = {
                let a = rng.random_range(0.0..std::f32::consts::TAU);
                vec3(a.cos(), 0.0, a.sin())
            };
            let jitter_local = jitter_dir * jitter_amount;

            let local_velocity =
            local_dir * radial_speed +
            Vec3::Y * up_speed +
            jitter_local;

            // Rotate velocity into world space
            let velocity = rot * local_velocity;

            let lifetime = if ed.particle_lifetime.x >= ed.particle_lifetime.y {
                ed.particle_lifetime.x
            } else {
                rng.random_range(ed.particle_lifetime.x..=ed.particle_lifetime.y)
            };

            let scale = if ed.base_scale.x >= ed.base_scale.y {
                ed.base_scale.x
            } else {
                rng.random_range(ed.base_scale.x..=ed.base_scale.y)
            };

            let alpha = if ed.base_alpha.x >= ed.base_alpha.y {
                ed.base_alpha.x
            } else {
                rng.random_range(ed.base_alpha.x..=ed.base_alpha.y)
            };

            // color randomization
            let color = if ed.colors.len() > 1 {
                ed.colors[rng.random_range(0..ed.colors.len())]
            } else {
                ed.colors[0]
            };

            // TODO: Instead allocate the right size at the beginning by multiplying the particles per second by the lifetime
            if emitter.count < emitter.positions.len() {
                let i = emitter.count;
                emitter.positions[i] = position;
                emitter.velocities[i] = velocity;
                emitter.colors[i] = color;
                emitter.lifetimes[i] = lifetime;
                emitter.times_alive[i] = 0.0;
                emitter.rotation_speeds[i] = 0.0;
                emitter.rotation_offsets[i] = 0.0;

                emitter.alphas[i] = alpha;
                emitter.base_alphas[i] = alpha;
                emitter.end_alphas[i] = alpha * ed.alpha_multiplier;
                emitter.alpha_powers[i] = ed.alpha_power;

                emitter.scales[i] = scale;
                emitter.base_scales[i] = scale;
                emitter.end_scales[i] = scale * ed.scale_multiplier;
                emitter.scale_powers[i] = ed.scale_power;
            } else {
                emitter.positions.push(position);
                emitter.velocities.push(velocity);
                emitter.colors.push(color);
                emitter.lifetimes.push(lifetime);
                emitter.times_alive.push(0.0);
                emitter.rotation_speeds.push(0.0);
                emitter.rotation_offsets.push(0.0);

                emitter.alphas.push(alpha);
                emitter.base_alphas.push(alpha);
                emitter.end_alphas.push(alpha * ed.alpha_multiplier);
                emitter.alpha_powers.push(ed.alpha_power);

                emitter.scales.push(scale);
                emitter.base_scales.push(scale);
                emitter.end_scales.push(scale * ed.scale_multiplier);
                emitter.scale_powers.push(ed.scale_power);
            }

            emitter.count += 1;
        }
    }

    pub fn render(&mut self, shader: &mut Shader, camera: &Camera) {
        // for emitter in self.emitters.iter_mut() {
        //     emitter.render(shader, camera, self.vao);
        // }

        if self.render_staged_emitters {
            for se in self.staged_emitters.iter_mut() {
                se.emitter.render(shader, camera, self.vao);
            }
        }
    }

}
