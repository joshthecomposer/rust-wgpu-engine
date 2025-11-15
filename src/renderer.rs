#![allow(dead_code, clippy::too_many_arguments)]
use std::{collections::{HashMap, HashSet}, ffi::c_void, mem, ptr::null_mut};

use gl::CULL_FACE;
use glam::{vec3, vec4, Mat4, Vec3, Vec4};
use image::GenericImageView;

use crate::{camera::Camera, entity_manager::EntityManager, enums_types::{AnimationType, EmitterName, EntityType, Faction, FboType, ShaderType, Transform, VaoType}, gl_call, grid::Grid, lights::Lights, particles::ParticleSystem, physics::PhysicsState, shaders::Shader, some_data::{FACES_CUBEMAP, POINT_LIGHT_POSITIONS, SHADOW_HEIGHT, SHADOW_WIDTH, SKYBOX_INDICES, SKYBOX_VERTICES, UNIT_CUBE_VERTICES}, sound::{fmod::{FMOD_Studio_EventInstance_Set3DAttributes, FMOD_3D_ATTRIBUTES, FMOD_VECTOR}, sound_manager::SoundManager}};

pub struct DefaultTextures {
    pub white: u32,
    pub black: u32,
    pub opaque: u32,
}

pub struct Renderer {
    pub shaders: HashMap<ShaderType, Shader>,
    pub vaos: HashMap<VaoType, u32>,
    pub fbos: HashMap<FboType, u32>,
    pub defaults: DefaultTextures,
    pub depth_map: u32,
    pub cubemap_texture: u32,

    pub shadow_debug: bool,
    pub render_gizmos: bool,
}

impl Renderer {
    pub fn new() -> Self {
        // =============================================================
        // Setup Shaders
        // =============================================================
        let mut shaders = HashMap::new();
        let mut vaos = HashMap::new();
        let mut fbos = HashMap::new();

        let skybox_shader = Shader::new("resources/shaders/skybox.glsl");
        let debug_light_shader = Shader::new("resources/shaders/point_light.glsl");
        let depth_shader = Shader::new("resources/shaders/depth_shader.glsl");
        let text_shader = Shader::new("resources/shaders/text.glsl");
        text_shader.activate();
        let loc = unsafe { gl::GetUniformLocation(text_shader.id, b"textTexture\0".as_ptr() as *const _) };
        unsafe {
            gl::Uniform1i(loc, 1); 
        }
        let model_shader = Shader::new("resources/shaders/color_for_texture.glsl");
        model_shader.activate();
        model_shader.set_int("material.Diffuse",  1);
        model_shader.set_int("material.Specular", 2);
        model_shader.set_int("material.Emissive", 3);
        model_shader.set_int("material.Opacity",  4);
        model_shader.set_int("shadow_map",        7);
        model_shader.set_int("skybox",           10);
        let gizmo_shader = Shader::new("resources/shaders/gizmo.glsl");
        let particle_shader = Shader::new("resources/shaders/particles.glsl");
        let game_ui_shader = Shader::new("resources/shaders/game_ui.glsl");

        let mut vao = 0;
        let mut vbo = 0;
        let mut ebo = 0;
        let mut cubemap_texture = 0;

        // =============================================================
        // Skybox memes
        // =============================================================
        unsafe {
            skybox_shader.activate();
            gl_call!(gl::GenVertexArrays(1, &mut vao));
            gl_call!(gl::GenBuffers(1, &mut vbo));
            gl_call!(gl::GenBuffers(1, &mut ebo));

            vaos.insert(VaoType::Skybox, vao);

            println!("vao skybox: {}", vao);

            gl_call!(gl::BindVertexArray(vao));

            gl_call!(gl::BindBuffer(gl::ARRAY_BUFFER, vbo));
            gl_call!(gl::BufferData(
                gl::ARRAY_BUFFER, 
                (mem::size_of::<f32>() * SKYBOX_VERTICES.len()) as isize,
                SKYBOX_VERTICES.as_ptr().cast(),
                gl::STATIC_DRAW
            ));

            gl_call!(gl::BindBuffer(gl::ELEMENT_ARRAY_BUFFER, ebo));
            gl_call!(gl::BufferData(
                gl::ELEMENT_ARRAY_BUFFER,
                (mem::size_of::<u32>() * SKYBOX_INDICES.len()) as isize,
                SKYBOX_INDICES.as_ptr().cast(),
                gl::STATIC_DRAW
            ));

            gl_call!(gl::VertexAttribPointer(
                0, 
                3, 
                gl::FLOAT, 
                gl::FALSE, 
                (3 * mem::size_of::<f32>()) as i32, 
                std::ptr::null(),
            ));
            gl_call!(gl::EnableVertexAttribArray(0));

            gl_call!(gl::BindVertexArray(0));
            gl_call!(gl::BindBuffer(gl::ARRAY_BUFFER, 0));
            gl_call!(gl::BindBuffer(gl::ELEMENT_ARRAY_BUFFER, 0));

            // SKYBOX TEXTURES
            gl_call!(gl::GenTextures(1, &mut cubemap_texture));
            gl_call!(gl::BindTexture(gl::TEXTURE_CUBE_MAP, cubemap_texture));
            gl_call!(gl::TexParameteri(gl::TEXTURE_CUBE_MAP, gl::TEXTURE_MAG_FILTER, gl::LINEAR as i32));
            gl_call!(gl::TexParameteri(gl::TEXTURE_CUBE_MAP, gl::TEXTURE_MIN_FILTER, gl::LINEAR as i32));
            // These are very important to prevent seams
            gl_call!(gl::TexParameteri(gl::TEXTURE_CUBE_MAP, gl::TEXTURE_WRAP_S, gl::CLAMP_TO_EDGE as i32));
            gl_call!(gl::TexParameteri(gl::TEXTURE_CUBE_MAP, gl::TEXTURE_WRAP_T, gl::CLAMP_TO_EDGE as i32));
            gl_call!(gl::TexParameteri(gl::TEXTURE_CUBE_MAP, gl::TEXTURE_WRAP_R, gl::CLAMP_TO_EDGE as i32));

            for i in 0..FACES_CUBEMAP.len() {
                let img = match image::open(FACES_CUBEMAP[i]) {
                    Ok(img) => img,
                    _=> panic!("Error opening {}", FACES_CUBEMAP[i]),
                };
                let (img_width, img_height) = img.dimensions();
                let rgba = img.to_rgb8();
                let raw = rgba.as_raw();

                gl_call!(gl::TexImage2D(
                    gl::TEXTURE_CUBE_MAP_POSITIVE_X + i as u32, 
                    0, 
                    gl::RGB as i32, 
                    img_width as i32, 
                    img_height as i32, 
                    0, 
                    gl::RGB, 
                    gl::UNSIGNED_BYTE, 
                    raw.as_ptr().cast()
                ));
            }
        }


        // =============================================================
        // Debug point light setup
        // =============================================================
        unsafe {
            debug_light_shader.activate();

            gl_call!(gl::GenVertexArrays(1, &mut vao));
            gl_call!(gl::GenBuffers(1, &mut vbo));


            vaos.insert(VaoType::DebugLight, vao);

            gl_call!(gl::BindVertexArray(vao));

            gl_call!(gl::BindBuffer(gl::ARRAY_BUFFER, vbo));
            gl_call!(gl::BufferData(
                gl::ARRAY_BUFFER, 
                (mem::size_of::<f32>() * UNIT_CUBE_VERTICES.len()) as isize, 
                UNIT_CUBE_VERTICES.as_ptr().cast(), 
                gl::STATIC_DRAW
            ));

            // Position 
            gl_call!(gl::VertexAttribPointer(
                0,
                3,
                gl::FLOAT,
                gl::FALSE,
                8 * mem::size_of::<f32>() as i32,
                std::ptr::null(),
            ));
            gl_call!(gl::EnableVertexAttribArray(0));
        
            // Normal
            gl_call!(gl::VertexAttribPointer(
                1,
                3,
                gl::FLOAT,
                gl::FALSE,
                8 * mem::size_of::<f32>() as i32,
                (5 * mem::size_of::<f32>()) as *const c_void
            ));
            gl_call!(gl::EnableVertexAttribArray(1));
        } 

        // =============================================================
        // Shadow Mapping
        // =============================================================
        // The general idea is that we need to create a depth map rendered 
        // from the perspective of the light source. In this case one 
        // directional light.
        // We can do this using a "framebuffer". We have been using a 
        // framebuffer all along, just the "default" one given to us.
        let mut fbo = 0;
        let mut depth_map = 0;
        unsafe {
            gl_call!(gl::GenFramebuffers(1, &mut fbo));

            fbos.insert(FboType::DepthMap, fbo);

            gl_call!(gl::GenTextures(1, &mut depth_map));
            gl_call!(gl::BindTexture(gl::TEXTURE_2D, depth_map));
            gl_call!(gl::TexImage2D(gl::TEXTURE_2D, 0, gl::DEPTH_COMPONENT as i32, SHADOW_WIDTH, SHADOW_HEIGHT, 0, gl::DEPTH_COMPONENT, gl::FLOAT, null_mut()));
            gl_call!(gl::TexParameteri(gl::TEXTURE_2D, gl::TEXTURE_MIN_FILTER, gl::NEAREST as i32));
            gl_call!(gl::TexParameteri(gl::TEXTURE_2D, gl::TEXTURE_MAG_FILTER, gl::NEAREST as i32));
            gl_call!(gl::TexParameteri(gl::TEXTURE_2D, gl::TEXTURE_WRAP_S, gl::CLAMP_TO_BORDER as i32));
            gl_call!(gl::TexParameteri(gl::TEXTURE_2D, gl::TEXTURE_WRAP_T, gl::CLAMP_TO_BORDER as i32));
            gl_call!(gl::TexParameterfv(
                gl::TEXTURE_2D, 
                gl::TEXTURE_BORDER_COLOR, 
                [1.0, 1.0, 1.0, 1.0].as_ptr().cast() 
            ));

            gl_call!(gl::BindFramebuffer(gl::FRAMEBUFFER, fbo));
            gl_call!(gl::FramebufferTexture2D(gl::FRAMEBUFFER, gl::DEPTH_ATTACHMENT, gl::TEXTURE_2D, depth_map, 0));
            gl_call!(gl::DrawBuffer(gl::NONE));
            gl_call!(gl::ReadBuffer(gl::NONE));
            gl_call!(gl::BindFramebuffer(gl::FRAMEBUFFER, 0));
        }

        let mut debug_depth_quad = Shader::new("resources/shaders/debug_depth_quad.glsl");

        debug_depth_quad.activate();
        debug_depth_quad.store_uniform_location("depth_map");
        debug_depth_quad.set_int("depth_map", 0);

        shaders.insert(ShaderType::Model, model_shader);
        shaders.insert(ShaderType::Skybox, skybox_shader);
        shaders.insert(ShaderType::DebugLight, debug_light_shader);
        shaders.insert(ShaderType::Depth, depth_shader);
        shaders.insert(ShaderType::DebugShadowMap, debug_depth_quad);
        shaders.insert(ShaderType::Text, text_shader);
        shaders.insert(ShaderType::Gizmo, gizmo_shader);
        shaders.insert(ShaderType::Particles, particle_shader);
        shaders.insert(ShaderType::GameUi, game_ui_shader);

        // DEFAULT TEXTURES
        let defaults = DefaultTextures {
            white:  Self::make_solid_texture(255,255,255,255),
            black:  Self::make_solid_texture(0,0,0,255),
            opaque: Self::make_solid_texture(255,255,255,255),
        };

        Self {
            shaders,
            vaos,
            fbos,
            depth_map,
            defaults,

            cubemap_texture,
            shadow_debug: false,
            render_gizmos: false,
        }
    }

    pub fn draw(
        &mut self, 
        em: &EntityManager, 
        camera: &mut Camera,
        light_manager: &Lights,
        sound_manager: &mut SoundManager,
        fb_width: u32,
        fb_height: u32,
        elapsed: f32,
        ps: &PhysicsState,
        alpha: f32,
        particles: &mut ParticleSystem
    ) {

        let active_weapons = em.get_active_weapon_ids();


        let non_animated_ids = em.entity_types.iter()
            .filter(|e| {
                em.skellingtons.get(e.key()).is_none()
            })
            .map(|e| e.key())
            .collect::<Vec<usize>>();

        let animated_ids = em.entity_types.iter()
            .filter(|e| {
                em.skellingtons.get(e.key()).is_some()
            })
            .map(|e| e.key())
            .collect::<Vec<usize>>();


        self.shadow_pass(em, camera, light_manager, fb_width, fb_height, ps, alpha, animated_ids, non_animated_ids);

        if self.shadow_debug {
            return;
        }

        // =============================================================
        // Render OOP-esque things
        // =============================================================
        // shadow pass must come first or you're gonna have a bad time
        self.skybox_pass(camera, fb_width, fb_height);
        // self.grid_pass(grid, camera, light_manager);
        
        // =============================================================
        // Render ECS things
        // =============================================================
        // Gizmo pass
        if self.render_gizmos {
            let gizmo_ids = em.get_gizmo_ids();
            self.gizmo_pass(camera, em, gizmo_ids, ps, alpha);
        }

        unsafe {
            gl::Enable(gl::CULL_FACE);
            gl::CullFace(gl::BACK);      // cull back faces
            gl::FrontFace(gl::CCW);      // CCW = front mesh
        }

        self.static_model_pass(camera, em, light_manager, ps, alpha, particles, sound_manager);

        unsafe {
            gl::Disable(gl::CULL_FACE);
        }
    }


    fn gizmo_pass(&mut self, camera: &mut Camera, em: &EntityManager, ids: Vec<usize>, ps: &PhysicsState, alpha: f32) {
        unsafe {
            gl_call!(gl::PolygonMode( gl::FRONT_AND_BACK, gl::LINE ));
        }

        let shader = self.shaders.get_mut(&ShaderType::Gizmo).unwrap();
        shader.activate();
        for id in ids {
            let model = match em.collider_gizmos.get(id) {
                Some(model) => model,
                None => continue,
            };
            let trans = Self::render_transform(em, id, alpha);
            let m_mat = Mat4::from_scale_rotation_translation(trans.scale, trans.rotation, trans.position);

            shader.set_mat4("model", m_mat);
            shader.set_mat4("projection", camera.projection);
            shader.set_mat4("view", camera.view);
            model.draw(shader);
        }

        unsafe {
            gl_call!(gl::PolygonMode( gl::FRONT_AND_BACK, gl::FILL ));
        }
    }

    fn make_solid_texture(r: u8, g: u8, b: u8, a: u8) -> u32 {
        let mut id = 0;
        unsafe {
            gl::GenTextures(1, &mut id);
            gl::BindTexture(gl::TEXTURE_2D, id);
            let pix = [r, g, b, a];
            gl::TexImage2D(
                gl::TEXTURE_2D, 0, gl::RGBA8 as i32, 1, 1, 0,
                gl::RGBA, gl::UNSIGNED_BYTE, pix.as_ptr() as *const _
            );
            gl::TexParameteri(gl::TEXTURE_2D, gl::TEXTURE_WRAP_S, gl::REPEAT as i32);
            gl::TexParameteri(gl::TEXTURE_2D, gl::TEXTURE_WRAP_T, gl::REPEAT as i32);
            gl::TexParameteri(gl::TEXTURE_2D, gl::TEXTURE_MIN_FILTER, gl::NEAREST as i32);
            gl::TexParameteri(gl::TEXTURE_2D, gl::TEXTURE_MAG_FILTER, gl::NEAREST as i32);
            gl::BindTexture(gl::TEXTURE_2D, 0);
        }
        id
    }

    fn ani_model_pass(&mut self, camera: &mut Camera, em: &EntityManager, light_manager: &Lights, sound_manager: &mut SoundManager, ids: Vec<usize>, elapsed: f32, ps: &PhysicsState, alpha: f32, particles: &mut ParticleSystem) {
        let shader = self.shaders.get_mut(&ShaderType::Model).unwrap();
        shader.activate();

        unsafe {
            gl::ActiveTexture(gl::TEXTURE7);
            gl::BindTexture(gl::TEXTURE_2D, self.depth_map);
                
            // Set default textures for models that don't have one
            gl::ActiveTexture(gl::TEXTURE1); gl::BindTexture(gl::TEXTURE_2D, self.defaults.white); // Diffuse default
            gl::ActiveTexture(gl::TEXTURE2); gl::BindTexture(gl::TEXTURE_2D, self.defaults.black); // Spec default
            gl::ActiveTexture(gl::TEXTURE3); gl::BindTexture(gl::TEXTURE_2D, self.defaults.black); // Emissive default
            gl::ActiveTexture(gl::TEXTURE4); gl::BindTexture(gl::TEXTURE_2D, self.defaults.opaque); // Opacity default
            gl::ActiveTexture(gl::TEXTURE10); gl::BindTexture(gl::TEXTURE_CUBE_MAP, self.cubemap_texture);
        }

        shader.set_bool("is_animated", true);
        shader.set_bool("alpha_test_pass", false);
        shader.set_float("elapsed", elapsed);
        shader.set_bool("do_reg_fresnel", false);
        for id in ids {

            if let Some(v_effect) = em.v_effects.get(id) {
                if v_effect.ttl > 0.0 {
                    shader.set_bool("flash_white", true);
                }
            }

            let is_selected = em.selected.contains(&id);
            shader.set_bool("selection_fresnel", is_selected);

            let model = em.models.get(id).unwrap();
            let trans = Self::render_transform(em, id, alpha);

            let animator = em.animators.get(id).unwrap();
            let animation = animator.get_current_animation().unwrap();

            // TODO: OneShot and FrameActivation are almost the same thing. I could likely
            // destructure this and use FrameActivation for both.
            for os in animation.one_shots.iter() {
                if animation.current_segment.get() == os.segment {
                    if !os.triggered.get() {
                        sound_manager.play_sound_3d(os.sound_type.clone(), &trans.position, id);
                        particles.spawn_oneshot_emitter(EmitterName::DesertStep, trans.position);
                        os.triggered.set(true);
                    }
                } else {
                    os.triggered.set(false);
                }
            }
            
            if let Some(fa) = &animation.hurtbox_activation {
                if fa.segment_range.contains(&animation.current_segment.get()) { 
                    if !fa.triggered.get() {
                        fa.triggered.set(true);
                    }
                } else {
                    fa.triggered.set(false);
                }
            }
            
            // TODO: Why are we updating sounds in the renderer? I get that it's probably because 
            // we are already accessing the animation, but likely the cost tradeoff is fine as
            // fetching the animation is constant time I believe. one thing is that if we render
            // and update sound at the same time the sound is probably less likely to have latency.
            for cs in animation.continuous_sounds.iter() {
                if !cs.playing.get() {
                    sound_manager.play_sound_3d(cs.sound_type.clone(), &trans.position, id);
                    cs.playing.set(true);
                } else if let Some(insts) = sound_manager.active_3d_sounds.get_mut(&id) {
                    for inst in insts.iter_mut() {
                        let attributes = FMOD_3D_ATTRIBUTES {
                            position: SoundManager::opengl_to_fmod(trans.position),
                            velocity: FMOD_VECTOR { x: 0.0, y: 0.0, z: 0.0 },
                            forward: FMOD_VECTOR { x: 0.0, y: 0.0, z: 1.0 },
                            up: FMOD_VECTOR { x: 0.0, y: 1.0, z: 0.0 }
                        };

                        unsafe {
                            let result = FMOD_Studio_EventInstance_Set3DAttributes(*inst, &attributes);

                            if result != 0 {
                                eprintln!("Failed to update 3D sound position: {}", result);
                            }
                        }
                    }
                }
            }


            let m_mat = Mat4::from_scale_rotation_translation(trans.scale, trans.rotation, trans.position);

            shader.set_mat4("model", m_mat);
            shader.set_mat4("projection", camera.projection);
            shader.set_mat4("view", camera.view);
            shader.set_mat4("light_space_mat", camera.light_space);
            shader.set_dir_light("dir_light", &light_manager.dir_light);
            shader.set_float("bias_scalar", light_manager.bias_scalar);
            shader.set_mat4_array("bone_transforms", &animation.current_pose);
            shader.set_vec3("view_position", camera.position);
            shader.set_int("skybox", 10);
            model.draw(shader);
            shader.set_bool("selection_fresnel", false);
            shader.set_bool("flash_white", false);
        }
        shader.set_bool("do_reg_fresnel", false);
    }


    fn static_model_pass(
        &mut self, camera: 
        &mut Camera, 
        em: &EntityManager, 
        light_manager: &Lights, 
        ps: &PhysicsState, 
        alpha: f32,
        particles: &mut ParticleSystem,
        sound_manager: &mut SoundManager,
    ) {
        unsafe {
            gl_call!(gl::Enable(gl::DEPTH_TEST));
            gl_call!(gl::DepthMask(gl::TRUE)); // Allow writing to depth buffer
            gl_call!(gl::Disable(gl::BLEND));


            // Set default textures for models that don't have one
            gl::ActiveTexture(gl::TEXTURE1); gl::BindTexture(gl::TEXTURE_2D, self.defaults.white); // Diffuse default
            gl::ActiveTexture(gl::TEXTURE2); gl::BindTexture(gl::TEXTURE_2D, self.defaults.black); // Spec default
            gl::ActiveTexture(gl::TEXTURE3); gl::BindTexture(gl::TEXTURE_2D, self.defaults.black); // Emissive default
            gl::ActiveTexture(gl::TEXTURE4); gl::BindTexture(gl::TEXTURE_2D, self.defaults.opaque); // Opacity default

            gl::ActiveTexture(gl::TEXTURE7); gl::BindTexture(gl::TEXTURE_2D, self.depth_map);
            gl::ActiveTexture(gl::TEXTURE10); gl::BindTexture(gl::TEXTURE_CUBE_MAP, self.cubemap_texture);
        }
        // Alpha pass
        let shader = self.shaders.get_mut(&ShaderType::Model).unwrap();
        shader.activate();
        
        // TODO: hacky way to set alpha test pass
        for i in 0..=1 {
            shader.set_bool("alpha_test_pass", i > 0);
            for id in em.entity_types.iter() {


                let is_selected = em.selected.contains(&id.key());
                shader.set_bool("selection_fresnel", is_selected);

                let model = em.models.get(id.key()).unwrap();
                let trans = Self::render_transform(em, id.key(), alpha);
                let m_mat = Mat4::from_scale_rotation_translation(trans.scale, trans.rotation, trans.position);

                match em.animators.get(id.key()) {
                    Some(animator) => {
                        let animation = animator.get_current_animation().unwrap();

                        for os in animation.one_shots.iter() {
                            if animation.current_segment.get() == os.segment {
                                if !os.triggered.get() {
                                    sound_manager.play_sound_3d(os.sound_type.clone(), &trans.position, id.key());
                                    particles.spawn_oneshot_emitter(EmitterName::DesertStep, trans.position);
                                    os.triggered.set(true);
                                }
                            } else {
                                os.triggered.set(false);
                            }
                        }

                        if let Some(fa) = &animation.hurtbox_activation {
                            if fa.segment_range.contains(&animation.current_segment.get()) { 
                                if !fa.triggered.get() {
                                    fa.triggered.set(true);
                                }
                            } else {
                                fa.triggered.set(false);
                            }
                        }


                        shader.set_bool("is_animated", true);
                        shader.set_mat4_array("bone_transforms", &animation.current_pose);

                    },
                    _ => {
                        shader.set_bool("is_animated", false);
                    },
                }

                shader.set_mat4("model", m_mat);
                shader.set_mat4("projection", camera.projection);
                shader.set_mat4("view", camera.view);
                shader.set_mat4("light_space_mat", camera.light_space);
                shader.set_dir_light("dir_light", &light_manager.dir_light);
                shader.set_float("bias_scalar", light_manager.bias_scalar);
                shader.set_vec3("view_position", camera.position);
                shader.set_int("skybox", 10);
                unsafe {
                    // gl_call!(gl::ActiveTexture(gl::TEXTURE0));
                    // gl_call!(gl::BindTexture(gl::TEXTURE_2D, self.depth_map));
                    // shader.set_int("shadow_map", 0);
                    model.draw(shader);
                    gl_call!(gl::BindTexture(gl::TEXTURE_2D, 0));
                }
                shader.set_bool("selection_fresnel", false);
            }
        }

        unsafe {
            gl_call!(gl::Disable(gl::BLEND));
            gl_call!(gl::DepthMask(gl::TRUE));
        }

        shader.set_bool("do_reg_fresnel", false);
        shader.set_bool("selection_fresnel", false);
    }

    fn grid_pass(&mut self,grid: &mut Grid, camera: &mut Camera, light_manager: &Lights) {
        unsafe {
            gl::Enable(gl::BLEND);
            gl::BlendFunc(gl::SRC_ALPHA, gl::ONE_MINUS_SRC_ALPHA);
        }
        let shader = self.shaders.get_mut(&ShaderType::Model).unwrap();
        shader.activate();

        shader.set_bool("do_reg_fresnel", false);
        shader.set_bool("selection_fresnel", false);

        shader.set_mat4("model", Mat4::IDENTITY);
        shader.set_mat4("view", camera.view);
        shader.set_mat4("projection", camera.projection);
        shader.set_mat4("light_space_mat", camera.light_space);
        shader.set_dir_light("dir_light", &light_manager.dir_light);
        shader.set_float("bias_scalar", light_manager.bias_scalar);
        shader.set_vec3("view_position", camera.position);
        shader.set_bool("is_animated", false);
        shader.set_bool("alpha_test_pass", false);
        unsafe {
            gl_call!(gl::ActiveTexture(gl::TEXTURE0));
            gl_call!(gl::BindTexture(gl::TEXTURE_2D, self.depth_map));
            shader.set_int("shadow_map", 0);

            grid.draw(shader);
            gl::Disable(gl::BLEND)
        }
    }

    fn skybox_pass(&mut self, camera: &mut Camera, fb_width: u32, fb_height: u32) {
        unsafe {
            let status = gl::CheckFramebufferStatus(gl::FRAMEBUFFER);
            if status != gl::FRAMEBUFFER_COMPLETE {
                println!("Framebuffer incomplete: {}", status);
            }
            let skybox_shader_prog = self.shaders.get(&ShaderType::Skybox).unwrap();

            gl_call!(gl::ClearColor(0.14, 0.13, 0.15, 1.0));
            gl_call!(gl::Clear(gl::COLOR_BUFFER_BIT | gl::DEPTH_BUFFER_BIT));

            let view_no_translation = Mat4 {
                x_axis: camera.view.x_axis,
                y_axis: camera.view.y_axis,
                z_axis: camera.view.z_axis,
                w_axis: vec4(0.0, 0.0, 0.0, 1.0),
            };
            gl_call!(gl::DepthFunc(gl::LEQUAL));

            skybox_shader_prog.activate();
            skybox_shader_prog.set_mat4("view", view_no_translation);
            skybox_shader_prog.set_mat4("projection", camera.projection);

            gl_call!(gl::BindVertexArray(*self.vaos.get(&VaoType::Skybox).unwrap()));
            gl_call!(gl::ActiveTexture(gl::TEXTURE1));
            gl_call!(gl::BindTexture(gl::TEXTURE_CUBE_MAP, self.cubemap_texture));
            gl_call!(gl::DrawElements(gl::TRIANGLES, 36, gl::UNSIGNED_INT, std::ptr::null(),));
            gl_call!(gl::BindVertexArray(0));

            gl_call!(gl::DepthFunc(gl::LESS));
            gl_call!(gl::BindTexture(gl::TEXTURE_CUBE_MAP, 0));
        }
    }

    fn shadow_pass(&mut self, em: &EntityManager, camera: &mut Camera, light_manager: &Lights, fb_width: u32, fb_height: u32, ps: &PhysicsState, alpha: f32, animated_ids: Vec<usize>, non_animated_ids: Vec<usize>) {
        let shader = self.shaders.get_mut(&ShaderType::Depth).unwrap();
        let near_plane = light_manager.near;
        let far_plane = light_manager.far;
        let half_bound = light_manager.bounds;

        let bound_l = -half_bound;
        let bound_r = half_bound;
        let bound_b = -half_bound;
        let bound_t = half_bound;

        // Calculate dir_light pos
        let light_dir = light_manager.dir_light.direction.normalize();
        let light_distance = light_manager.dir_light.distance;
        let camera_forward = camera.forward.normalize();
        let shadow_push = half_bound * 1.2;
        let shadow_center = camera.position + camera_forward * shadow_push;

        let light_pos = shadow_center + light_dir * light_distance;

        let light_projection = Mat4::orthographic_rh_gl(bound_l, bound_r, bound_b, bound_t, near_plane, far_plane);
        let light_view = Mat4::look_at_rh(light_pos, shadow_center, vec3(0.0, 1.0, 0.0));

        camera.light_space = light_projection * light_view;
        shader.activate();
        shader.set_mat4("light_space_mat", camera.light_space);

        unsafe {
            gl_call!(gl::Viewport(0, 0, SHADOW_WIDTH, SHADOW_HEIGHT));
            gl_call!(gl::BindFramebuffer(gl::FRAMEBUFFER, *self.fbos.get(&FboType::DepthMap).unwrap()));
            gl_call!(gl::Clear(gl::DEPTH_BUFFER_BIT));
            // Render scene
            gl_call!(gl::Enable(CULL_FACE));
            //gl_call!(gl::CullFace(gl::BACK));
            gl::CullFace(gl::FRONT);
            self.render_sample_depth(em, ps, alpha, animated_ids, non_animated_ids);
            gl_call!(gl::CullFace(gl::BACK)); 
            gl_call!(gl::Disable(CULL_FACE));
            // End render
            gl_call!(gl::BindFramebuffer(gl::FRAMEBUFFER,0));
            gl_call!(gl::Viewport(0, 0, fb_width as i32, fb_height as i32));

            gl_call!(gl::ClearColor(0.0, 0.0, 0.0, 1.0));
            gl_call!(gl::Clear(gl::COLOR_BUFFER_BIT | gl::DEPTH_BUFFER_BIT));
        }

        // Render only shadow from light point of view if true
        if self.shadow_debug {
            unsafe {
                let depth_debug_quad = self.shaders.get(&ShaderType::DebugShadowMap).unwrap();
                depth_debug_quad.activate();
                gl_call!(gl::ActiveTexture(gl::TEXTURE0));
                gl_call!(gl::BindTexture(gl::TEXTURE_2D, self.depth_map));
            }
            self.render_quad();
            return;
        }
    }

    fn render_sample_depth(&mut self, em: &EntityManager, ps: &PhysicsState, alpha: f32, animated_ids: Vec<usize>, non_animated_ids: Vec<usize>) {
        let depth_shader = self.shaders.get(&ShaderType::Depth).unwrap();
        depth_shader.activate();

        depth_shader.set_bool("is_animated", false);
        for id in non_animated_ids {
            let check = em.factions.get(id).unwrap();

            //  if !active_weapon_ids.contains(&model.key()) && check == &Faction::Item {
            //      continue;
            //  }
            // TODO:: Get rid of this. see above...
            if check == &Faction::Gizmo {
                continue;
            }
            let model = em.models.get(id).unwrap();
            let trans = Self::render_transform(em, id, alpha);
            //let trans = em.transforms.get(model.key()).unwrap();

            let model_model = Mat4::from_scale_rotation_translation(trans.scale, trans.rotation, trans.position);
            unsafe {
                gl::BindVertexArray(model.vao);
            }
            depth_shader.set_mat4("model", model_model);
            unsafe {
                gl_call!(gl::DrawElements(
                    gl::TRIANGLES, 
                    model.indices.len() as i32, 
                    gl::UNSIGNED_INT, 
                    std::ptr::null(),
                ));

                gl_call!(gl::BindVertexArray(0));
            }
        }

        depth_shader.set_bool("is_animated", true);

        for id in animated_ids {
            if let Some(animator) = em.animators.get(id) {
                let animation = animator.get_current_animation().unwrap();

                let trans = Self::render_transform(em, id, alpha);
                let model = em.models.get(id).unwrap();

                depth_shader.set_mat4_array("bone_transforms", &animation.current_pose);

                let mat = Mat4::from_scale_rotation_translation(trans.scale, trans.rotation, trans.position);
                unsafe {
                    gl::BindVertexArray(model.vao);
                }
                depth_shader.set_mat4("model", mat);

                unsafe {
                    gl_call!(gl::DrawElements(
                        gl::TRIANGLES, 
                        model.indices.len() as i32, 
                        gl::UNSIGNED_INT, 
                        std::ptr::null(),
                    ));

                    gl_call!(gl::BindVertexArray(0));
                }
            }
        }

    }

    fn debug_light_pass(&mut self, camera: &mut Camera) {
        let debug_light_shader = self.shaders.get(&ShaderType::DebugLight).unwrap();
        debug_light_shader.activate();
        debug_light_shader.set_mat4("view", camera.view);
        debug_light_shader.set_mat4("projection", camera.projection);

        unsafe {
            gl_call!(gl::BindVertexArray(*self.vaos.get(&VaoType::DebugLight).unwrap()));
            for light_pos in &POINT_LIGHT_POSITIONS {

                let mut m_mat = Mat4::IDENTITY;
                m_mat *= Mat4::from_translation(*light_pos);
                m_mat *= Mat4::from_scale(vec3(0.2, 0.2, 0.2)); 

                debug_light_shader.set_mat4("model", m_mat);
                debug_light_shader.set_vec3("LightColor", vec3(1.0, 1.0, 1.0));

                gl_call!(gl::DrawArrays(gl::TRIANGLES, 0, 36));
            }
        }
    }

    pub fn render_quad(&self) {
        let mut vao = 0;
        let mut vbo = 0;

        let quad_vertices: [f32; 30] = [
            // Positions      // Texture Coords
            -1.0,  1.0, 0.0,  0.0, 1.0,
            -1.0, -1.0, 0.0,  0.0, 0.0,
            1.0, -1.0, 0.0,  1.0, 0.0,

            -1.0,  1.0, 0.0,  0.0, 1.0,
            1.0, -1.0, 0.0,  1.0, 0.0,
            1.0,  1.0, 0.0,  1.0, 1.0
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
                gl::STATIC_DRAW
            ));

            let stride = (5 * std::mem::size_of::<f32>()) as i32;

            // Position Attribute
            gl_call!(gl::EnableVertexAttribArray(0));
            gl_call!(gl::VertexAttribPointer(0, 3, gl::FLOAT, gl::FALSE, stride, std::ptr::null()));

            // Texture Coordinate Attribute
            gl_call!(gl::EnableVertexAttribArray(1));
            gl_call!(gl::VertexAttribPointer(1, 2, gl::FLOAT, gl::FALSE, stride, (3 * std::mem::size_of::<f32>()) as *const _));

            gl_call!(gl::BindBuffer(gl::ARRAY_BUFFER, 0));
            gl_call!(gl::BindVertexArray(0));
        }

        // Draw the quad
        unsafe {
            gl_call!(gl::BindVertexArray(vao));
            gl_call!(gl::DrawArrays(gl::TRIANGLES, 0, 6));
            gl_call!(gl::BindVertexArray(0));
        }
    }

    pub fn render_transform(em: &EntityManager, id: usize, alpha: f32) -> Transform {
        let curr = em.transforms.get(id).unwrap();
        let prev = em.prev_transforms.get(id).unwrap_or(curr);
        Transform {
            position: prev.position.lerp(curr.position, alpha),
            rotation: prev.rotation.slerp(curr.rotation, alpha),
            scale:    curr.scale,
        }
    }

    //pub fn render_transform(em: &EntityManager, id: usize, alpha: f32) -> Transform {
    //    let curr = em.transforms.get(id).unwrap();
    //
    //    let is_physics_owned = em.physics_handles.get(id).is_some();
    //    if is_physics_owned {
    //        // physics drives curr; draw at curr to match bone time
    //        return curr.clone();
    //    }
    //
    //    let prev = em.prev_transforms.get(id).unwrap_or(curr);
    //    Transform {
    //        position: prev.position.lerp(curr.position, alpha),
    //        rotation: prev.rotation.slerp(curr.rotation, alpha),
    //        scale:    curr.scale,
    //    }
    //}
}
