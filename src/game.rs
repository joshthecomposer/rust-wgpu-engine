use std::path::Path;

use gl::ActiveTexture;
use glutin::surface::GlSurface;
use winit::event::{ElementState, KeyEvent, WindowEvent};
use winit::keyboard::{KeyCode, PhysicalKey};
use winit::window::CursorGrabMode;

use crate::animation::animation_system;
use crate::config::game_config::GameConfig;
use crate::entity_manager::EntityManager;
use crate::enums_types::{CameraState, PhysicsHandle, ShaderType, SoundType, Transform};
use crate::input::{self, InputState};
use crate::physics::PhysicsState;
use crate::platform::{CursorMode, Platform};
use crate::renderer::Renderer;
use crate::sound::sound_manager::SoundManager;
use crate::state_machines::state_machine_system;
use crate::time::Time;
use crate::ui::engine_ui_manager::{self, EngineUiManager};
use crate::ui::game_ui::{do_ui, GameUiContext};
use crate::ui::message_queue::{MessageQueue, UiMessage};
use crate::util::data_structure::{HashMapGetPair, HashMapGetPairMut};
use crate::world::World;
use crate::{combat_system, grounding_solver, items, movement_system};

pub struct Game {
    pub platform: Platform, // OS/window/events
    time: Time,             // delta time, alpha time, elapsed time
    physics: PhysicsState,
    pub world: World, // ECS, terrain, particles, sim
    renderer: Renderer,
    sound: SoundManager,
    pub input: InputState,
    ui: GameUiContext,
    // imgui_manager: ImguiManager,
    pub paused: bool,
    message_queue: MessageQueue,
    engine_ui: EngineUiManager,
}

impl Game {
    pub fn new(mut platform: Platform) -> Self {
        let config = GameConfig::load_from_file("config/game_config.json");

        let start_seconds = 0.0;
        let time = Time::new(60.0, start_seconds);

        let mut physics = PhysicsState::new();
        let mut world = World::new();

        world.ecs.populate_entity_data(&mut physics);

        let renderer = Renderer::new();
        let sound = SoundManager::new(&config);
        let ui = GameUiContext::new();
        let engine_ui = EngineUiManager::new(platform.fb_width, platform.fb_height);

        Self {
            platform,
            time,
            physics,
            world,
            renderer,
            sound,
            input: InputState::new(),
            ui,
            paused: false,
            message_queue: MessageQueue::new(),
            engine_ui,
        }
    }

    pub fn tick(&mut self, now_seconds: f32) {
        self.time.begin_frame(now_seconds);

        // Mouse lock / cursor mode
        if self.paused || self.world.camera.move_state == CameraState::Locked {
            self.platform.window.set_cursor_visible(true);
            let _ = self.platform.window.set_cursor_grab(CursorGrabMode::None);
            self.platform.cursor_mode = CursorMode::Normal;
        } else {
            self.platform.window.set_cursor_visible(false);
            let _ = self
                .platform
                .window
                .set_cursor_grab(CursorGrabMode::Confined);
            self.platform.cursor_mode = CursorMode::Hidden;
        }

        let mut stepped = false;

        while self.time.should_step() {
            stepped = true;
            self.time.begin_fixed_step();

            {
                for curr in self.world.ecs.transforms.iter() {
                    self.world
                        .ecs
                        .prev_transforms
                        .insert(curr.key(), curr.value().clone());
                }
            }

            {
                let cam = &mut self.world.camera;
                cam.prev_pos = cam.position;
                cam.prev_forward = cam.forward;
                cam.prev_up = cam.up;
                cam.prev_target = cam.target;
            }

            let cam_basis = self.world.camera.basis_for_sim();

            if !self.paused {
                grounding_solver::grounding_solver(&mut self.world.ecs, &self.physics);

                state_machine_system::update(
                    &mut self.world.ecs,
                    self.time.fixed_dt,
                    &mut self.world.particles,
                    &self.input,
                    &mut self.physics,
                    &mut self.sound,
                    &self.world.camera,
                );

                items::update(&mut self.world.ecs, &mut self.physics);
                animation_system::update(&mut self.world.ecs, self.time.fixed_dt);
                combat_system::update(
                    &mut self.world.ecs,
                    self.time.fixed_dt,
                    &mut self.physics,
                    &mut self.world.particles,
                );
                self.world.ecs.update(
                    &mut self.sound,
                    &mut self.physics,
                    &mut self.input,
                    self.time.fixed_dt,
                );

                Self::push_weapon_kinematics_from_bones(&self.world.ecs, &mut self.physics);
                Self::push_static_kinematics(&self.world.ecs, &mut self.physics);

                match self.world.camera.move_state {
                    CameraState::Third | CameraState::Locked => {
                        movement_system::update(
                            &mut self.world.ecs,
                            self.time.fixed_dt,
                            &cam_basis,
                            &self.input,
                            &mut self.physics,
                        );
                    }
                    _ => {}
                }

                self.physics.step();
            }

            Self::sync_transforms_from_physics(&mut self.world.ecs, &self.physics);

            self.time.end_fixed_step();
        }

        self.time.end_frame();

        self.update();
        self.render();

        if stepped {
            self.input.update();
        }
    }

    pub fn handle_window_event(&mut self, event: &WindowEvent) {
        // Let Slint handle the event first
        let ui_consumed = self.engine_ui.handle_window_event(&event);

        // Process events for the game (keyboard, mouse, etc.)
        // Note: Currently we always process game events regardless of UI consumption.
        // If Slint UI has focused elements (e.g., text input), you may want to skip game input.
        match event {
            WindowEvent::Resized(size) => {
                self.platform.fb_width = size.width;
                self.platform.fb_height = size.height;
            }

            WindowEvent::CloseRequested => {}

            WindowEvent::CursorMoved { position, .. } => {
                if !self.paused {
                    self.world.camera.process_mouse_input_movement(*position);
                }
                self.input.mouse_pos_current = glam::vec2(position.x as f32, position.y as f32);
            }

            WindowEvent::MouseInput { state, button, .. } => match state {
                ElementState::Pressed => {
                    self.input.mouse_current.insert(*button);
                }
                ElementState::Released => {
                    self.input.mouse_current.remove(button);
                }
            },

            WindowEvent::KeyboardInput {
                event:
                    KeyEvent {
                        physical_key,
                        state,
                        ..
                    },
                ..
            } => {
                if let PhysicalKey::Code(code) = physical_key {
                    let keycode: KeyCode = *code;

                    match state {
                        ElementState::Pressed => {
                            self.input.keys_current.insert(keycode);
                        }
                        ElementState::Released => {
                            self.input.keys_current.remove(&keycode);
                        }
                    }

                    if keycode == KeyCode::Escape && *state == ElementState::Pressed {
                        self.paused = !self.paused;
                    }

                    // F toggles camera mode (Free <-> Third <-> Locked)
                    if keycode == KeyCode::KeyF && *state == ElementState::Pressed {
                        let maybe_player_id = self
                            .world
                            .ecs
                            .factions
                            .iter()
                            .find(|e| *e.value() == "Player");

                        self.world.camera.move_state = match self.world.camera.move_state {
                            CameraState::Free => {
                                if maybe_player_id.is_none() {
                                    CameraState::Locked
                                } else {
                                    CameraState::Third
                                }
                            }
                            CameraState::Third => CameraState::Locked,
                            CameraState::Locked => CameraState::Free,
                        };
                    }
                }
            }

            _ => {}
        }
    }

    pub fn update(&mut self) {
        self.world.camera.update(
            &self.world.ecs,
            self.time.dt,
            &self.physics,
            self.time.alpha,
            &self.input,
            self.platform.fb_width as f32 / self.platform.fb_height as f32,
        );
        self.sound.update(&self.world.camera);
        self.world.lights.update(&self.time.dt);
        self.world.particles.update(self.time.dt);

        let msgs = self.message_queue.drain();

        if msgs.contains(&UiMessage::ReloadWorldData) {
            let mut world = World::new();
            let mut physics = PhysicsState::new();

            world.ecs.populate_entity_data(&mut physics);

            // Cleanup 3d sounds
            {
                let keys: Vec<usize> = self.sound.active_3d_sounds.keys().cloned().collect();

                for id in keys {
                    self.sound.cleanup_entity_sounds(id);
                }
            }

            // Cleanup 2d sounds
            {
                let sounds: Vec<SoundType> = self.sound.active_sounds.keys().cloned().collect();

                for sound in sounds {
                    self.sound.stop_sound(&sound);
                }
            }

            self.world = world;
            self.physics = physics;
        }

        self.engine_ui.update();
    }

    pub fn render(&mut self) {
        // unsafe { gl::Enable(gl::FRAMEBUFFER_SRGB); }

        self.renderer.draw(
            &self.world.ecs,
            &mut self.world.camera,
            &self.world.lights,
            &mut self.sound,
            self.platform.fb_width,
            self.platform.fb_height,
            self.time.elapsed,
            &self.physics,
            self.time.alpha,
            &mut self.world.particles,
        );

        self.world.particles.render(
            self.renderer
                .shaders
                .get_mut(&ShaderType::Particles)
                .unwrap(),
            &self.world.camera,
        );

        // unsafe { gl::Disable(gl::FRAMEBUFFER_SRGB); }

        // Fix for mac scaled pixels garbage.
        let logical_size = self.platform.window.inner_size();
        let win_w = logical_size.width as f32;
        let win_h = logical_size.height as f32;

        let fb_w = self.platform.fb_width as f32;
        let fb_h = self.platform.fb_height as f32;

        let sx = fb_w as f32 / win_w as f32;
        let sy = fb_h as f32 / win_h as f32;

        let mouse_fb = glam::vec2(
            self.input.mouse_pos_current.x * sx,
            self.input.mouse_pos_current.y * sy,
        );

        let (ui_shader, font_shader) = self
            .renderer
            .shaders
            .get_pair_mut(&ShaderType::GameUi, &ShaderType::Text)
            .unwrap();

        do_ui(
            self.platform.fb_width as f32,
            self.platform.fb_height as f32,
            mouse_fb,
            ui_shader,
            font_shader,
            &mut self.message_queue,
            &mut self.paused,
            self.platform.cursor_mode,
            &self.world.camera.move_state,
            &mut self.ui,
            &mut self.renderer.render_gizmos,
            &mut self.input,
            &mut self.world.ecs,
        );

        // self.imgui_manager.draw(
        //     &mut self.platform.window,
        //     self.platform.fb_width as f32,
        //     self.platform.fb_height as f32,
        //     self.time.dt,
        //     &mut self.world.lights,
        //     &mut self.renderer,
        //     &mut self.sound,
        //     &self.world.camera,
        //     &mut self.world.ecs,
        //     &mut self.physics,
        //     &mut self.input,
        //     &mut self.world.particles,
        // );

        // Render Slint UI overlay
        self.engine_ui.render();
        let ui_overlay_shader = self.renderer.shaders.get(&ShaderType::UiOverlay).unwrap();
        self.engine_ui.draw_overlay(ui_overlay_shader);

        self.platform
            .surface
            .swap_buffers(&self.platform.gl_context)
            .expect("swap_buffers failed");
    }

    // PRIVATE //

    fn sync_transforms_from_physics(em: &mut EntityManager, ps: &PhysicsState) {
        let mut updates: Vec<(usize, glam::Vec3, glam::Quat)> =
            Vec::with_capacity(em.physics_handles.len());

        for ph in em.physics_handles.iter() {
            let id = ph.key();
            let PhysicsHandle { rigid_body, .. } = *ph.value();

            if let Some(rb) = ps.rigid_body_set.get(rigid_body) {
                let iso = rb.position();
                let pos = glam::Vec3::from_slice(iso.translation.vector.as_slice());
                let rot = {
                    let c = iso.rotation.coords;
                    glam::Quat::from_xyzw(c.x, c.y, c.z, c.w)
                };
                updates.push((id, pos, rot));
            }
        }

        // Apply to ECS transforms
        for (id, pos, rot) in updates {
            if let Some(t) = em.transforms.get_mut(id) {
                t.position = pos;
                t.rotation = rot;
                // keep existing t.scale as-is
            } else {
                // If some physics-driven entity somehow lacked a Transform, create one
                em.transforms.insert(
                    id,
                    Transform {
                        position: pos,
                        rotation: rot,
                        scale: glam::Vec3::splat(1.0), // or preserve a known scale (e.g., Vec3::ONE)
                    },
                );
            }
        }
    }

    fn push_weapon_kinematics_from_bones(em: &EntityManager, ps: &mut PhysicsState) {
        for wid in em.get_active_weapon_ids() {
            let parent = *em.owners.get(wid).unwrap();
            let animator = em.animators.get(parent).unwrap();
            let cur = animator.current_animation.clone();
            let next = animator.next_animation.clone();
            let blend = animator.blend_factor;

            let pt = em.transforms.get(parent).unwrap();
            let pm =
                glam::Mat4::from_scale_rotation_translation(pt.scale, pt.rotation, pt.position);
            let skel = em.skellingtons.get(parent).unwrap();
            let rh = em.item_bones.get(parent).unwrap().rh_name.clone();

            let bone_m = if blend > 0.0 && cur != next {
                let (a1, a2) = animator.animations.get_pair(&cur, &next).unwrap();
                a1.get_raw_global_bone_transform_by_name_blended(&rh, skel, pm, a2, blend)
            } else {
                animator
                    .animations
                    .get(&cur)
                    .unwrap()
                    .get_raw_global_bone_transform_by_name(&rh, skel, pm)
            };

            if let (Some(m), Some(ph)) = (bone_m, em.physics_handles.get(wid)) {
                //let (_s, rot, pos) = m.to_scale_rotation_translation();

                let corr = em.local_corrections.get(wid).cloned().unwrap_or(Transform {
                    position: glam::Vec3::ZERO,
                    rotation: glam::Quat::IDENTITY,
                    scale: glam::Vec3::ONE,
                });

                let corr_m = glam::Mat4::from_scale_rotation_translation(
                    corr.scale,
                    corr.rotation,
                    corr.position,
                );

                // Apply correction in bone space
                // (boneWorld * correctionLocal) -> final weapon world
                let final_m = m * corr_m;

                let (_, rot, pos) = final_m.to_scale_rotation_translation();

                if let Some(rb) = ps.rigid_body_set.get_mut(ph.rigid_body) {
                    if rb.is_kinematic() {
                        let iso = rapier3d::na::Isometry3::from_parts(
                            rapier3d::na::Translation3::new(pos.x, pos.y, pos.z),
                            rapier3d::na::UnitQuaternion::from_quaternion(
                                rapier3d::na::Quaternion::new(rot.w, rot.x, rot.y, rot.z),
                            ),
                        );
                        rb.set_next_kinematic_position(iso);
                    }
                }
            }
        }
    }

    fn push_static_kinematics(em: &EntityManager, ps: &mut PhysicsState) {
        for id in em.selected.iter() {
            if let Some(ph) = em.physics_handles.get(*id) {
                let rb = ps.rigid_body_set.get_mut(ph.rigid_body).unwrap();

                rb.wake_up(true);

                let gt = em.transforms.get(*id).unwrap();

                let iso = rapier3d::na::Isometry::from_parts(
                    rapier3d::na::Translation3::new(gt.position.x, gt.position.y, gt.position.z),
                    rapier3d::na::UnitQuaternion::from_quaternion(gt.rotation.into()),
                );

                rb.set_next_kinematic_position(iso);
            }
        }
    }
}
