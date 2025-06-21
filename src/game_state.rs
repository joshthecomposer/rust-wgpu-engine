#![allow(dead_code)]
use std::collections::{HashSet, VecDeque};

use gl::{AttachShader, PixelStoref};
use glam::{vec2, vec3, Quat, Vec2, Vec3};
use glfw::{Context, Glfw, GlfwReceiver, Key, PWindow, WindowEvent};
use image::GrayImage;
use rapier3d::{math::Isometry, prelude::{ColliderBuilder, RigidBodyBuilder}};
use rusttype::{point, Font, Scale};

use crate::{animation::animation_system, camera::Camera, collision_system, config::{entity_config::{self, EntityConfig}, game_config::GameConfig, world_data::WorldData}, debug::{gizmos::Cylinder, write::write_data}, entity_manager::{self, EntityManager}, enums_types::{AnimationType, CameraState, EntityType, Faction, PhysicsHandle, ShaderType, SimState, Transform}, gl_call, grid::Grid, input::{handle_keyboard_input, handle_mouse_input, InputState}, items, lights::{DirLight, Lights}, movement_system, particles::{Emitter, ParticleSystem}, physics::PhysicsState, renderer::Renderer, sound::{fmod::FMOD_Studio_System_Update, sound_manager::SoundManager}, state_machines, terrain::Terrain, ui::{font::{self, FontManager}, game_ui::{self, GameUiContext}, imgui::ImguiManager, message_queue::{MessageQueue, UiMessage}}};
// use rand::prelude::*;
// use rand_chacha::ChaCha8Rng;

pub struct GameState {
    pub delta_time: f32,
    pub last_frame: f32,
    pub elapsed: f32,
    pub camera: Camera,
    pub window_width: u32,
    pub window_height: u32,
    pub fb_width: u32,
    pub fb_height: u32,

    // GLFW context
    pub glfw: Glfw,
    pub events: GlfwReceiver<(f64, WindowEvent)>,
    pub window: PWindow,

    pub entity_manager: EntityManager,
    pub light_manager: Lights,
    pub imgui_manager: ImguiManager,


    pub paused: bool,
    pub was_paused: bool,

    pub grid: Grid,
    pub renderer: Renderer,

    pub input_state: InputState,

    pub sound_manager: SoundManager,

    pub terrain: Terrain,
    pub cursor_pos: Vec2,
    pub font_manager: FontManager,
    pub fps: u32,
    pub last_fps_update: f32,
    pub particles: ParticleSystem,

    pub message_queue: MessageQueue,
    pub game_ui_context: GameUiContext,
    pub physics_state: PhysicsState,
}

impl GameState {
    pub fn new() -> Self {
        let mut glfw = glfw::init(glfw::fail_on_errors).expect("Failed to init glfw");

        glfw.window_hint(glfw::WindowHint::ContextVersion(4, 6)); // OpenGL 3.3
        glfw.window_hint(glfw::WindowHint::OpenGlProfile(glfw::OpenGlProfileHint::Core));
        glfw.window_hint(glfw::WindowHint::Resizable(true));
        #[cfg(target_os = "macos")]
        glfw.window_hint(glfw::WindowHint::OpenGlForwardCompat(true));

        let (mut width, mut height):(i32, i32) = (1920, 1080);

        let (mut window, events) = glfw
            .create_window(width as u32, height as u32, "Hello this is window", glfw::WindowMode::Windowed)
            .expect("Failed to create GLFW window.");
        // window.set_key_polling(true);
        // window.set_sticky_keys(true); 
        window.set_cursor_mode(glfw::CursorMode::Disabled);
        window.set_all_polling(true);
        window.set_framebuffer_size_polling(true);
        window.make_current();

        glfw.with_primary_monitor(|_glfw, maybe_monitor| {
            if let Some(monitor) = maybe_monitor {
                if let Some(video_mode) = monitor.get_video_mode() {
                    // Extract the current resolution & refresh rate from the monitor
                    (width, height) = (video_mode.width as i32, video_mode.height as i32);
                    let refresh_rate    = video_mode.refresh_rate; // e.g. 60, 144, etc.

                    window.set_monitor(
                        glfw::WindowMode::Windowed,
                        // glfw::WindowMode::FullScreen(monitor),
                        100,      // X-position on that monitor
                        100,      // Y-position on that monitor
                        1920,
                        1080,
                        // width as u32,
                        // height as u32,
                        Some(refresh_rate)
                    );
                }
            }
        });

        glfw.set_swap_interval(glfw::SwapInterval::Sync(1));


        let (fb_width, fb_height) = window.get_framebuffer_size();

        println!("Framebuffer size: {}x{}", fb_width, fb_height);

        gl::load_with(|symbol| window.get_proc_address(symbol) as *const _);

        unsafe {
            gl_call!(gl::Enable(gl::BLEND));
            gl_call!(gl::BlendFunc(gl::SRC_ALPHA, gl::ONE_MINUS_SRC_ALPHA));
            gl_call!(gl::Enable(gl::TEXTURE_CUBE_MAP_SEAMLESS));
            gl_call!(gl::Viewport(0, 0, width, height));
            gl_call!(gl::Enable(gl::DEPTH_TEST));
            gl::Enable(gl::DEBUG_OUTPUT);
            gl::Enable(gl::BLEND);
            gl::BlendFunc(gl::SRC_ALPHA, gl::ONE_MINUS_SRC_ALPHA);
        }

        // =============================================================
        // Set up systems
        // =============================================================
        let mut light_manager = Lights::new(50);
        light_manager.dir_light = DirLight::default_white();

        let renderer = Renderer::new();

        let game_config = GameConfig::load_from_file("config/game_config.json");

        let sound_manager = SoundManager::new(&game_config);

        let mut entity_config = EntityConfig::load_from_file("config/entity_config.json");
        let mut world_data = WorldData::load_from_file("config/world_data.toml");
        let mut physics_state = PhysicsState::new();
        let mut entity_manager = EntityManager::new(10_000);
        entity_manager.populate_initial_entity_data(&mut entity_config, &mut world_data, &mut physics_state);

        let mut grid = Grid::new(game_config.grid_width, game_config.grid_height, game_config.cell_size);
        grid.generate();

        let imgui_manager = ImguiManager::new(&mut window);

        //TERRAIN
        let mut terrain = Terrain::from_height_map("resources/textures/grid_height.png");

        let model = terrain.into_opengl_model();

        let terrain_trans = Transform {
            position: Vec3::splat(0.0),
            rotation: Quat::IDENTITY,
            scale: Vec3::splat(1.0),
            original_rotation: Quat::IDENTITY,
        };

        entity_manager.transforms.insert(entity_manager.next_entity_id, terrain_trans.clone(), );

        entity_manager.factions.insert(entity_manager.next_entity_id, Faction::World);
        entity_manager.entity_types.insert(entity_manager.next_entity_id, EntityType::Terrain);

        // Terrain collider

        // let iso: Isometry<f32> = (terrain_trans.position, terrain_trans.rotation).into();
        // let body = RigidBodyBuilder::fixed().position(iso).build();
        // let terrain_collider = terrain.create_collider();

        // dbg!(&terrain_collider);

        // let body_handle = physics_state.rigid_body_set.insert(body);
        // let collider_handle = physics_state.collider_set.insert_with_parent(
        //     terrain_collider,
        //     body_handle,
        //     &mut physics_state.rigid_body_set,
        // );

        // entity_manager.physics_handles.insert(entity_manager.next_entity_id, PhysicsHandle {
        //     rigid_body: body_handle,
        //     collider: collider_handle,
        // });
        let terrain_trans = Transform {
            position: Vec3::new(0.0, -1.0, 0.0), // Sink it slightly so top is at y=0
            rotation: Quat::IDENTITY,
            scale: Vec3::splat(1.0),
            original_rotation: Quat::IDENTITY,
        };

        entity_manager.transforms.insert(entity_manager.next_entity_id, terrain_trans.clone());
        entity_manager.factions.insert(entity_manager.next_entity_id, Faction::World);
        entity_manager.entity_types.insert(entity_manager.next_entity_id, EntityType::Terrain);

        // Make a big static cube collider
        let iso: Isometry<f32> = (terrain_trans.position, terrain_trans.rotation).into();
        let body = RigidBodyBuilder::fixed().position(iso).build();
        let terrain_collider = ColliderBuilder::cuboid(50.0, 0.5, 50.0).build(); // Big square floor

        let body_handle = physics_state.rigid_body_set.insert(body);
        let collider_handle = physics_state.collider_set.insert_with_parent(
            terrain_collider,
            body_handle,
            &mut physics_state.rigid_body_set,
        );

        entity_manager.physics_handles.insert(entity_manager.next_entity_id, PhysicsHandle {
            rigid_body: body_handle,
            collider: collider_handle,
        });

        // entity_manager.models.insert(entity_manager.next_entity_id, model);

        entity_manager.next_entity_id += 1;

        // sound_manager.play_sound_3d("moose3D".to_string(), &vec3(0.0, 0.0, 4.0));

        // FONT MANAGER

        let mut font_manager = FontManager::new();
        font_manager.load_chars("ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789:,.!?()[]{}<> ");
        font_manager.setup_buffers();

        let particles = ParticleSystem::new();
        // particles.spawn_continuous_emitter(100, vec3(10.0, 20.0, 10.0), "Smoke", Some("resources/textures/smoke.png"));
         // particles.spawn_continuous_emitter(50, Vec3::splat(0.0), "Smoke", None);

        let ui_ctx = GameUiContext::new();


        Self {
            delta_time: 0.0,
            last_frame: 0.0,
            elapsed: 0.0,
            camera: Camera::new(),
            window_width: width as u32,
            window_height: height as u32,
            fb_width:  fb_width as u32,
            fb_height: fb_height as u32,

            glfw,
            events,
            window,

            entity_manager,
            light_manager,
            imgui_manager,

            paused: false,
            was_paused: false,

            grid,
            renderer,

            input_state: InputState::new(),
            sound_manager,

            terrain,
            cursor_pos: Vec2::new(0.0, 0.0),
            font_manager,
            fps: 0,
            last_fps_update: 0.0,
            particles,
            message_queue: MessageQueue::new(), 
            game_ui_context: ui_ctx,
            physics_state,
        }
    }

    pub fn process_events(&mut self) {
        if self.was_paused && !self.paused {
            self.camera.sync_mouse_position(&self.window);
        }
        self.was_paused = self.paused;
        self.camera.process_key_event(&self.window, self.delta_time);
        let events: Vec<(f64, glfw::WindowEvent)> = glfw::flush_messages(&self.events).collect();

        for (_, event) in events {
            self.imgui_manager.handle_imgui_event(&event);
            match event {
                glfw::WindowEvent::FramebufferSize(w, h) => {
                    self.window_width = w as u32;
                    self.window_height = h as u32;
                    unsafe {
                        gl::Viewport(0, 0, self.window_width as i32, self.window_height as i32);
                    }
                },
                glfw::WindowEvent::CursorPos(xpos, ypos) => {
                    self.cursor_pos.x = xpos as f32;
                    self.cursor_pos.y = ypos as f32;

                    if !self.paused {
                        self.camera.process_mouse_input(&self.window, &event);
                    }
                },
                glfw::WindowEvent::Key(key, _, action, _) => {
                    match key {
                        glfw::Key::G => {
                            if action == glfw::Action::Press {
                                self.particles.spawn_oneshot_emitter(50, Vec3::splat(0.0));
                            }
                        },
                        glfw::Key::Escape => {
                            if action == glfw::Action::Press {
                                self.message_queue.send(UiMessage::PauseToggle);
                            }
                        },
                        glfw::Key::Num1 => {
                            if action == glfw::Action::Press {
                                let player_id = self.entity_manager.factions.iter().filter(|f| *f.value() == Faction::Player).last().unwrap().key();
                                let active_weapon = self.entity_manager.active_items.get_mut(player_id).unwrap();
                                let curr_weapon_id = active_weapon.right_hand.unwrap();

                                let inventory = self.entity_manager.inventories.get_mut(player_id).unwrap();

                                let next_weapon_id = inventory.items.pop();

                                active_weapon.right_hand = next_weapon_id;

                                inventory.items.push(curr_weapon_id);
                            }
                        },
                        _ => {}
                    }
                    handle_keyboard_input(key, action, &mut self.input_state);
                },
                glfw::WindowEvent::MouseButton(btn, action, _) => {
                    handle_mouse_input(btn, action, self.cursor_pos, Vec2::new(self.fb_width as f32, self.fb_height as f32), &self.camera, &mut self.entity_manager, &self.input_state);
                    if btn  == glfw::MouseButtonLeft && action == glfw::Action::Press {
                        self.message_queue.send(UiMessage::LeftMouseClicked);
                    }
                },
                _ => (),
            }
        }
    }

    pub fn update(&mut self) {
        // CALC DELTA TIME
        let current_frame = self.glfw.get_time() as f32;
        self.delta_time = current_frame - self.last_frame;
        self.last_frame = current_frame;
        self.elapsed += self.delta_time;

        if self.delta_time <= 0.0 {
            return;
        }
        
        // Fps calc
        let fps_now = (1.0 / self.delta_time.max(0.0001)) as u32;
        if self.elapsed - self.last_fps_update >= 0.5 {
            self.fps = fps_now;
            self.last_fps_update = self.elapsed;
        }

        self.particles.update(self.delta_time);

        if let Some(player_entry) = self.entity_manager.factions.iter().find(|f| f.value() == &Faction::Player) {
            let player_key = player_entry.key();
            let animator = self.entity_manager.animators.get_mut(player_key).unwrap();

            if self.input_state.keys_current.contains(&glfw::Key::P) {
                animator.set_next_animation(AnimationType::Death);
            }

            if self.input_state.keys_current.contains(&glfw::Key::O) {
                animator.set_next_animation(AnimationType::Idle);
            }
        }

        if self.input_state.keys_current.contains(&glfw::Key::Delete) {
            for id in self.entity_manager.selected.iter() {
                self.entity_manager.sim_states.insert(*id, SimState::Dying);
                if let Some(parent) = self.entity_manager.parents.iter().find(|p| p.value().parent_id == *id) {
                    let cyl_id = parent.key();

                    if let Some(_cyl) = self.entity_manager.cylinders.get(cyl_id) {
                        self.entity_manager.entity_trashcan.push(cyl_id);
                    }
                }
            }
        }


        let desired_cursor_mode = if self.paused {
            // println!("Setting cursormode to normal at line 305");
            glfw::CursorMode::Normal
        } else if self.camera.move_state == CameraState::Locked {
            // println!("Setting cursormode to normal at line 308");
            glfw::CursorMode::Normal // or Disabled, based on your UI preferences
        } else {
            // println!("Setting cursormode to disabled at line 311");
            glfw::CursorMode::Disabled
        };

        self.window.set_cursor_mode(desired_cursor_mode);

        if self.paused {
            // don't update the simulation/animations if paused
            return;
        }

        // UPDATE OOP-ESQUE STRUCTS
        self.camera.update(&self.entity_manager, self.delta_time);
        self.sound_manager.update(&self.camera);
        self.light_manager.update(&self.delta_time);

        // UPDATE SYSTEMS
        movement_system::update(
            &mut self.entity_manager, &self.terrain, self.delta_time, &self.camera, &self.input_state, &mut self.physics_state
        );
        animation_system::update(&mut self.entity_manager, self.delta_time);
        state_machines::update(&mut self.entity_manager, self.delta_time, &mut self.particles);
        // collision_system::update(&mut self.entity_manager);
        self.physics_state.update();
        items::update(&mut self.entity_manager);
        self.entity_manager.update(&mut self.sound_manager, &self.physics_state);

        self.input_state.update();// likely this shoudl always be last because it just checks if we
        // are holding a key
    }

    pub fn render(&mut self) {
        // ======================================
        // Handle windowed/FullScreen
        // ======================================
        // TODO: should we abstract this out somewhere?
        if self.input_state.keys_current.contains(&glfw::Key::H) {
            self.glfw.with_primary_monitor(|_glfw, maybe_monitor| {
                if let Some(monitor) = maybe_monitor {
                    if let Some(video_mode) = monitor.get_video_mode() {
                        let refresh_rate    = video_mode.refresh_rate;

                        self.window.set_monitor(
                            glfw::WindowMode::FullScreen(monitor),
                            0,      // X-position on that monitor
                            0,      // Y-position on that monitor
                            video_mode.width,
                            video_mode.height,
                            Some(refresh_rate)
                        );
                        self.window.set_cursor_mode(glfw::CursorMode::Normal);
                    }
                }
            });
        } else if self.input_state.keys_current.contains(&glfw::Key::J) {
            self.window.set_monitor(
                glfw::WindowMode::Windowed,
                100,
                100,
                1920,
                1080,
                None,
            );
        }
        let (new_width, new_height) = self.window.get_framebuffer_size();
        if new_width as u32 != self.fb_width || new_height as u32 != self.fb_height {
            self.fb_width = new_width as u32;
            self.fb_height = new_height as u32;

            unsafe { gl::Viewport(0, 0, new_width, new_height); }

            let aspect = new_width as f32 / new_height as f32;
            self.camera.reset_matrices(aspect);
        }

        // ======================================
        // Actually draw stuff
        // ======================================
        self.renderer.draw(&self.entity_manager, &mut self.camera, &self.light_manager, &mut self.grid, &mut self.sound_manager, self.fb_width, self.fb_height, self.elapsed);

        self.particles.render(
            self.renderer.shaders.get_mut(&ShaderType::Particles).unwrap(),
            &self.camera,
        );
        
        self.imgui_manager.draw(&mut self.window, self.fb_width as f32, self.fb_height as f32, self.delta_time, &mut self.light_manager, &mut self.renderer, &mut self.sound_manager, &self.camera, &mut self.entity_manager);


        // let phrase = format!("FPS: {}", self.fps);

        // self.font_manager.render_phrase(
        //     &phrase,
        //     100.0,
        //     100.0,
        //     self.fb_width as f32,
        //     self.fb_height as f32,
        //     self.renderer.shaders.get_mut(&ShaderType::Text).unwrap(),
        //     0.7,
        // );

       game_ui::do_ui(
            self.fb_width as f32, 
            self.fb_height as f32, 
            self.cursor_pos, 
            &mut self.font_manager,
            self.renderer.shaders.get(&ShaderType::GameUi).unwrap(),
            self.renderer.shaders.get(&ShaderType::Text).unwrap(),
            &mut self.message_queue,
            self.paused,
            self.window.get_cursor_mode(),
            &self.camera.move_state,
            &self.input_state.keys_current,
            &mut self.game_ui_context,
        );

        if self.message_queue.queue.contains(&UiMessage::WindowShouldClose) {
            self.window.set_should_close(true);
        }

        if self.message_queue.queue.contains(&UiMessage::PauseToggle) {
            self.paused = !self.paused;
        }

        self.window.swap_buffers();
        self.glfw.poll_events();
        self.message_queue.drain();
    }
}
