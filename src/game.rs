use glam::{Quat, Vec3};
use glutin::surface::GlSurface;
use winit::event::{ElementState, KeyEvent, WindowEvent};
use winit::keyboard::{KeyCode, PhysicalKey};
use winit::window::CursorGrabMode;

use crate::animation::animation_system;
use crate::config::emitter_data::EmitterBlackboard;
use crate::config::game_config::GameConfig;
use crate::config::world_data::EntityInstance;
use crate::entity_manager::EntityManager;
use crate::enums_types::{CameraState, PhysicsHandle, ShaderType, SoundType, Transform};
use crate::input::{self, InputState};
use crate::state_machines::state_machine_system;
use crate::ui::deprecated_game_ui::game_ui::GameUiContext;
use crate::ui::events::UiEvent;
use crate::ui::message_queue::{MessageQueue, UiMessage};
use crate::ui::ui_manager::UiManager;
use crate::ultralight::types::ViewType;
use crate::util::data_structure::HashMapGetPair;
use crate::{combat_system, grounding_solver, items, movement_system};
use crate::physics::PhysicsState;
use crate::renderer::Renderer;
use crate::sound::sound_manager::SoundManager;
use crate::time::Time;
use crate::platform::{CursorMode, Platform};
use crate::world::World;

/// Pending entity creation info when in create mode.
#[derive(Default)]
struct PendingEntityCreate {
    enabled: bool,
    entity_type: String,
    faction: String,
    weapon: Option<String>,
    base_speed: f32,
}

pub struct Game {
    pub platform: Platform, // OS/window/events
    time: Time, // delta time, alpha time, elapsed time
    physics: PhysicsState,
    pub world: World, // ECS, terrain, particles, sim
    renderer: Renderer,
    sound: SoundManager,
    pub input: InputState,
    #[allow(dead_code)]
    ui: GameUiContext,
    /// UI Manager - handles Ultralight HTML/CSS/JS UI system
    ui_manager: UiManager,
    pub paused: bool,
    /// Flag to signal that the window should close (set by pause menu quit button)
    pub should_close: bool,
    message_queue: MessageQueue,
    /// Pending entity to create when clicking on terrain
    pending_entity_create: PendingEntityCreate,
    /// Preview emitter ID for the particle editor (None = no preview active)
    preview_emitter_id: Option<usize>,
    /// Timer for respawning preview emitters
    preview_emitter_timer: f32,
    /// Track if last mouse click was on terrain (not UI)
    last_click_was_terrain: bool,
    /// Stored emitter data for continuous preview
    preview_emitter_data: Option<(EmitterBlackboard, Vec3)>,
}

impl Game {
    pub fn new(platform: Platform, ui_manager: UiManager) -> Self {
        let config = GameConfig::load_from_file("config/game_config.json");

        let start_seconds = 0.0;
        let time = Time::new(60.0, start_seconds);

        let mut physics = PhysicsState::new();
        let mut world = World::new();

        world.ecs.populate_entity_data(&mut physics);

        let renderer = Renderer::new();
        let sound = SoundManager::new(&config);
        let ui = GameUiContext::new();

        Self {
            platform,
            time,
            physics,
            world,
            renderer,
            sound,
            input: InputState::new(),
            ui,
            ui_manager,
            paused: false,
            should_close: false,
            message_queue: MessageQueue::new(),
            pending_entity_create: PendingEntityCreate::default(),
            preview_emitter_id: None,
            preview_emitter_timer: 0.0,
            last_click_was_terrain: false,
            preview_emitter_data: None,
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
            let _ = self.platform.window.set_cursor_grab(CursorGrabMode::Confined);
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
                self.world
                    .ecs
                    .update(&mut self.sound, &mut self.physics, &mut self.input, self.time.fixed_dt);

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
        // First, route events to UI manager and check if it wants to capture them
        let (ul_wants_mouse, ul_wants_keyboard) = self.ui_manager.handle_event(event);

        match event {
            WindowEvent::Resized(size) => {
                self.platform.fb_width = size.width;
                self.platform.fb_height = size.height;

                // Resize UI views to match the new window size
                self.ui_manager.handle_resize(size.width, size.height);
            }

            WindowEvent::CloseRequested => {
            }

            WindowEvent::DroppedFile(_path) => {
                // TODO: Wire up file drops to Ultralight UI when needed
                // Previously used for ImGui entity/particle editor file drops
            }

            WindowEvent::CursorMoved { position, .. } => {
                // Only process camera movement if Ultralight doesn't want mouse
                if !ul_wants_mouse && !self.paused {
                    self.world
                        .camera
                        .process_mouse_input_movement(*position);
                }
                self.input.mouse_pos_current =
                    glam::vec2(position.x as f32, position.y as f32);
            }

            WindowEvent::MouseInput { state, button, .. } => {
                // Track if this click was on terrain (not captured by UI)
                self.last_click_was_terrain = !ul_wants_mouse;

                // Process game mouse input if UI doesn't want mouse
                // OR if we're in entity create mode (to allow terrain clicking)
                // OR if we're in editor mode (to allow setting emitter position)
                if !ul_wants_mouse || self.pending_entity_create.enabled || self.ui_manager.editor_was_visible {
                    let fb = glam::vec2(
                        self.platform.fb_width as f32,
                        self.platform.fb_height as f32,
                    );

                    input::handle_mouse_input(
                        *button,
                        *state,
                        fb,
                        &self.world.camera,
                        &mut self.world.ecs,
                        &mut self.input,
                        &mut self.physics,
                    );
                }
            }

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

                    // Always handle game keyboard input for now (F key, Escape, etc.)
                    // Only skip if Ultralight wants keyboard AND it's not a special key
                    let is_special_key = keycode == KeyCode::Escape || keycode == KeyCode::KeyF;
                    if !ul_wants_keyboard || is_special_key {
                        input::handle_keyboard_input(keycode, *state, &mut self.input);
                    }

                    // Escape toggles pause
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
        self.world.camera.update(&self.world.ecs, self.time.dt, &self.physics, self.time.alpha, &self.input, self.platform.fb_width as f32 / self.platform.fb_height as f32);
        self.sound.update(&self.world.camera);
        self.world.lights.update(&self.time.dt);
        self.world.particles.update(self.time.dt);

        // Update preview emitter timer for continuous respawning
        if self.world.particles.render_staged_emitters {
            self.preview_emitter_timer += self.time.dt;

            // Respawn one-shot emitters every 1 second for continuous preview
            if self.preview_emitter_timer >= 1.0 {
                if let Some((ref emitter, position)) = self.preview_emitter_data {
                    // For one-shot emitters (pps == 0), respawn them
                    // For continuous emitters (pps > 0), just edit them
                    if emitter.pps.is_none() || emitter.pps == Some(0) {
                        self.world.particles.spawn_oneshot_editor_emitter(emitter, position);
                    } else if let Some(id) = self.preview_emitter_id {
                        self.world.particles.edit_staged_emitter(id, emitter, position);
                    }
                }
                self.preview_emitter_timer = 0.0;
            }
        }

        // Update UI manager
        let should_show_editor = self.world.camera.move_state == CameraState::Locked;

        // Show/hide editor based on camera state
        let editor_just_shown = self.ui_manager.update_editor_visibility(should_show_editor);
        if editor_just_shown && should_show_editor {
            // Populate dropdowns when editor is first shown
            let entity_types: Vec<String> = self.world.ecs.entity_type_register
                .keys()
                .cloned()
                .collect();
            let factions: Vec<String> = self.world.ecs.faction_register
                .iter()
                .cloned()
                .collect();
            let mut emitter_types: Vec<String> = self.world.particles.emitter_data
                .one_shot_data
                .keys()
                .cloned()
                .collect();
            emitter_types.sort();
            self.ui_manager.update_editor_dropdowns(&entity_types, &factions, &emitter_types);
        }

        // Show/hide pause menu based on paused state
        self.ui_manager.update_pause_menu_visibility(self.paused, self.renderer.render_gizmos);

        // Update UI manager each frame
        self.ui_manager.update(self.time.dt);

        // Update editor with player data if editor is visible
        if should_show_editor {
            // Find the player entity and get its data
            let mut player_pos = [0.0f32, 0.0, 0.0];
            let mut player_state = "Unknown".to_string();
            let mut attack_state = "None".to_string();
            let mut animation = "Unknown".to_string();

            // Iterate through player controllers to find the player
            for pc in self.world.ecs.player_controllers.iter() {
                let id = pc.key();
                let controller = pc.value();

                // Get position
                if let Some(transform) = self.world.ecs.transforms.get(id) {
                    player_pos = [transform.position.x, transform.position.y, transform.position.z];
                }

                // Get state info
                player_state = format!("{}", controller.state);
                attack_state = format!("{}", controller.attack_state);

                // Get current animation
                if let Some(animator) = self.world.ecs.animators.get(id) {
                    animation = format!("{:?}", animator.current_animation);
                }

                break; // Only need first player
            }

            self.ui_manager.update_editor_state(
                player_pos,
                &player_state,
                &attack_state,
                &animation
            );
        }

        // Poll and process UI events
        let ui_events = self.ui_manager.poll_events();
        for event in ui_events {
            self.handle_ui_event(event);
        }

        // Check for entity creation when in create mode and clicking terrain
        let mut entity_was_placed = false;
        if self.pending_entity_create.enabled && self.input.ray_just_hit {
            // Create the entity at the clicked position
            let weapons = if let Some(weapon_type) = &self.pending_entity_create.weapon {
                Some(vec![EntityInstance {
                    entity_type: weapon_type.clone(),
                    faction: "Item".to_string(),
                    position: glam::Vec3::ZERO,
                    rotation: Quat::IDENTITY,
                    weapons: None,
                    base_speed: None,
                    jump_height: None,
                    health: None,
                    cleanup_timer: None,
                }])
            } else {
                None
            };

            let base_speed = if self.pending_entity_create.base_speed > 0.0 {
                Some(self.pending_entity_create.base_speed)
            } else {
                None
            };

            let instance = EntityInstance {
                entity_type: self.pending_entity_create.entity_type.clone(),
                faction: self.pending_entity_create.faction.clone(),
                position: self.input.ray_pos,
                rotation: Quat::IDENTITY,
                weapons,
                base_speed,
                jump_height: Some(1.0),
                health: Some(100.0),
                cleanup_timer: None,
            };

            println!("[Game] Spawning entity at {:?}: {} ({})",
                self.input.ray_pos,
                instance.entity_type,
                instance.faction);

            let parent_id = self.world.ecs.create_entity(&instance, &mut self.physics);
            self.world.ecs.populate_inventory(parent_id, &instance, &mut self.physics);

            // Reset create mode
            self.pending_entity_create.enabled = false;
            self.input.ray_just_hit = false;
            entity_was_placed = true;
        }

        // Notify UI that entity was placed
        if entity_was_placed {
            self.ui_manager.notify_entity_placed();
        }

        // Update emitter position in UI when clicking terrain in editor mode (not in create mode, not on UI)
        if !self.pending_entity_create.enabled && self.input.ray_just_hit && self.ui_manager.editor_was_visible && self.last_click_was_terrain {
            let pos = self.input.ray_pos;
            self.ui_manager.update_emitter_position([pos.x, pos.y, pos.z]);
            self.input.ray_just_hit = false;
        }

        let msgs = self.message_queue.drain();

        if msgs.contains(&UiMessage::ReloadWorldData) {
            let mut world = World::new();
            let mut physics = PhysicsState::new();

            world.ecs.populate_entity_data(&mut physics);

            // Cleanup 3d sounds
            {
                let keys: Vec<usize> = self.sound.active_3d_sounds.keys().cloned().collect();

                for id in keys  {
                    self.sound.cleanup_entity_sounds(id);
                }
            }

            // Cleanup 2d sounds
            {
                let sounds: Vec<SoundType> = self.sound.active_sounds.keys().cloned().collect();

                for sound in sounds  {
                    self.sound.stop_sound(&sound);
                }
            }

            self.world = world;
            self.physics = physics;
        }
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
            self.renderer.shaders.get_mut(&ShaderType::Particles).unwrap(),
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

        let _mouse_fb = glam::vec2(self.input.mouse_pos_current.x * sx, self.input.mouse_pos_current.y * sy);

        // DEPRECATED: Old game_ui replaced by Ultralight views (PauseMenu, Hud)
        // let (ui_shader, font_shader) = self.renderer.shaders.get_pair_mut(&ShaderType::GameUi, &ShaderType::Text).unwrap();
        // do_ui(
        //     self.platform.fb_width as f32,
        //     self.platform.fb_height as f32,
        //     mouse_fb,
        //     ui_shader,
        //     font_shader,
        //     &mut self.message_queue,
        //     &mut self.paused,
        //     self.platform.cursor_mode,
        //     &self.world.camera.move_state,
        //     &mut self.ui,
        //     &mut self.renderer.render_gizmos,
        //     &mut self.input,
        //     &mut self.world.ecs,
        // );

        // Render UI
        self.ui_manager.render(self.platform.fb_width, self.platform.fb_height);

        self.platform
            .surface
            .swap_buffers(&self.platform.gl_context)
            .expect("swap_buffers failed");
    }


    // PRIVATE //

    fn sync_transforms_from_physics(em: &mut EntityManager, ps: &PhysicsState) {
        let mut updates: Vec<(usize, glam::Vec3, glam::Quat)> = Vec::with_capacity(em.physics_handles.len());

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
                em.transforms.insert(id, Transform {
                    position: pos,
                    rotation: rot,
                    scale: glam::Vec3::splat(1.0), // or preserve a known scale (e.g., Vec3::ONE)
                });
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
            let pm = glam::Mat4::from_scale_rotation_translation(pt.scale, pt.rotation, pt.position);
            let skel = em.skellingtons.get(parent).unwrap();
            let rh = em.item_bones.get(parent).unwrap().rh_name.clone();

            let bone_m = if blend > 0.0 && cur != next {
                let (a1, a2) = animator.animations.get_pair(&cur, &next).unwrap();
                a1.get_raw_global_bone_transform_by_name_blended(&rh, skel, pm, a2, blend)
            } else {
                animator.animations.get(&cur).unwrap()
                    .get_raw_global_bone_transform_by_name(&rh, skel, pm)
            };

            if let (Some(m), Some(ph)) = (bone_m, em.physics_handles.get(wid)) {
                //let (_s, rot, pos) = m.to_scale_rotation_translation();

                let corr = em.local_corrections
                    .get(wid)
                    .cloned()
                    .unwrap_or(Transform {
                        position: glam::Vec3::ZERO,
                        rotation: glam::Quat::IDENTITY,
                        scale:    glam::Vec3::ONE,
                    });

                let corr_m = glam::Mat4::from_scale_rotation_translation(
                    corr.scale, corr.rotation, corr.position
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
                    rapier3d::na::UnitQuaternion::from_quaternion(gt.rotation.into())
                );

                rb.set_next_kinematic_position(iso);
            }
        }
    }

    /// Handle a structured UI event.
    fn handle_ui_event(&mut self, event: UiEvent) {
        match event {
            UiEvent::LightUpdate { x, y, z, distance } => {
                if let Some(x) = x {
                    self.world.lights.dir_light.direction.x = x;
                }
                if let Some(y) = y {
                    self.world.lights.dir_light.direction.y = y;
                }
                if let Some(z) = z {
                    self.world.lights.dir_light.direction.z = z;
                }
                if let Some(distance) = distance {
                    self.world.lights.dir_light.distance = distance;
                }
                // Update view_pos based on direction and distance
                let dir = self.world.lights.dir_light.direction;
                let dist = self.world.lights.dir_light.distance;
                self.world.lights.dir_light.view_pos = dir * dist;
            }
            UiEvent::ShadowDebug { enabled } => {
                self.renderer.shadow_debug = enabled;
            }
            UiEvent::OrthoUpdate { near, far, bounds, bias } => {
                if let Some(near) = near {
                    self.world.lights.near = near;
                }
                if let Some(far) = far {
                    self.world.lights.far = far;
                }
                if let Some(bounds) = bounds {
                    self.world.lights.bounds = bounds;
                }
                if let Some(bias) = bias {
                    self.world.lights.bias_scalar = bias;
                }
            }
            UiEvent::VolumeUpdate { volume } => {
                self.sound.master_volume = volume;
                self.sound.set_master_volume(&SoundType::Music);
                println!("[Game] Set master volume to {}", volume);
            }
            UiEvent::SoundToggle { paused } => {
                if paused {
                    self.sound.stop_sound(&SoundType::Music);
                } else {
                    self.sound.play_sound_2d(SoundType::Music);
                }
            }
            UiEvent::SoundPause => {
                self.sound.stop_sound(&SoundType::Music);
                println!("[Game] Paused music");
            }
            UiEvent::SoundPlay => {
                self.sound.play_sound_2d(SoundType::Music);
                println!("[Game] Playing music");
            }
            UiEvent::CreateModeToggle { enabled, entity_type, faction, weapon, base_speed } => {
                if enabled {
                    self.pending_entity_create = PendingEntityCreate {
                        enabled: true,
                        entity_type: entity_type.clone(),
                        faction: faction.clone(),
                        weapon: weapon.clone(),
                        base_speed,
                    };
                    println!("[Game] Create mode enabled - entity_type: {}, faction: {}, weapon: {:?}, base_speed: {}",
                        entity_type, faction, weapon, base_speed);
                } else {
                    self.pending_entity_create.enabled = false;
                    println!("[Game] Create mode disabled");
                }
            }
            UiEvent::RenderEmitterPreview { enabled, raw_json } => {
                self.world.particles.render_staged_emitters = enabled;

                if enabled {
                    // Parse the raw JSON to get emitter data
                    if let Ok(event) = serde_json::from_str::<serde_json::Value>(&raw_json) {
                        if let Some((emitter, position)) = Self::parse_emitter_from_json(&event) {
                            // Store emitter data for continuous respawning
                            self.preview_emitter_data = Some((emitter.clone(), position));
                            self.preview_emitter_timer = 0.0;

                            if let Some(id) = self.preview_emitter_id {
                                // Update existing preview
                                self.world.particles.edit_staged_emitter(id, &emitter, position);
                            } else {
                                // Spawn new preview
                                let id = self.world.particles.spawn_oneshot_editor_emitter(&emitter, position);
                                self.preview_emitter_id = Some(id);
                            }
                            println!("[Game] Emitter preview spawned/updated at {:?}", position);
                        }
                    }
                } else {
                    // Clear preview emitter id and stored data when disabled
                    self.preview_emitter_id = None;
                    self.preview_emitter_data = None;
                }
                println!("[Game] Emitter preview rendering: {}", enabled);
            }
            UiEvent::SaveEmitter { raw_json } => {
                // Parse the raw JSON to get emitter data
                if let Ok(event) = serde_json::from_str::<serde_json::Value>(&raw_json) {
                    let name = event.get("name").and_then(|v| v.as_str()).unwrap_or("").to_string();
                    if name.is_empty() {
                        eprintln!("[Game] Cannot save emitter: name is empty");
                        return;
                    }
                    // Check if name already exists
                    if self.world.particles.emitter_data.one_shot_data.contains_key(&name) {
                        eprintln!("[Game] Cannot save emitter: name '{}' already exists", name);
                        return;
                    }
                    if let Some((emitter, position)) = Self::parse_emitter_from_json(&event) {
                        println!("[Game] Saving emitter: {} at {:?}", emitter.name, position);
                        self.world.particles.emitter_data.one_shot_data.insert(emitter.name.clone(), emitter);
                        self.world.particles.emitter_data.write_to_file("config/particle_emitters.toml");
                        println!("[Game] Emitter saved to config/particle_emitters.toml");
                    }
                }
            }
            UiEvent::PauseResume => {
                self.paused = false;
                println!("[Game] Pause menu closed/resumed");
            }
            UiEvent::PauseToggleGizmos => {
                self.renderer.render_gizmos = !self.renderer.render_gizmos;
                println!("[Game] Gizmos toggled: {}", self.renderer.render_gizmos);
                // Update the UI to reflect the new state
                let gizmo_status = if self.renderer.render_gizmos { "true" } else { "false" };
                self.ui_manager.execute_js(ViewType::PauseMenu, &format!("updateGizmoStatus({})", gizmo_status));
            }
            UiEvent::PauseReloadWorld => {
                self.message_queue.send(UiMessage::ReloadWorldData);
                println!("[Game] Reload world data requested");
            }
            UiEvent::PauseSavePlayer => {
                self.world.ecs.serialize_entity_data("config/player_data.json");
                println!("[Game] Player data saved");
            }
            UiEvent::PauseQuit => {
                self.should_close = true;
                println!("[Game] Quit game requested");
            }
            UiEvent::Unknown { event_type } => {
                println!("[Game] Unknown UI event type: {}", event_type);
            }
        }
    }

    /// Parse emitter data from JSON event. Returns (EmitterBlackboard, position).
    fn parse_emitter_from_json(event: &serde_json::Value) -> Option<(EmitterBlackboard, Vec3)> {
        use glam::{Vec2, Vec4};

        // Helper to parse [min, max] range from JSON array
        let parse_range = |arr: Option<&serde_json::Value>| -> Vec2 {
            arr.and_then(|v| v.as_array())
                .map(|a| {
                    let min = a.get(0).and_then(|v| v.as_f64()).unwrap_or(0.0);
                    let max = a.get(1).and_then(|v| v.as_f64()).unwrap_or(min);
                    Vec2::new(min as f32, max as f32)
                })
                .unwrap_or(Vec2::ZERO)
        };

        // Helper to parse [x, y, z] from JSON array
        let parse_vec3 = |arr: Option<&serde_json::Value>| -> Vec3 {
            arr.and_then(|v| v.as_array())
                .map(|a| {
                    let x = a.get(0).and_then(|v| v.as_f64()).unwrap_or(0.0) as f32;
                    let y = a.get(1).and_then(|v| v.as_f64()).unwrap_or(0.0) as f32;
                    let z = a.get(2).and_then(|v| v.as_f64()).unwrap_or(0.0) as f32;
                    Vec3::new(x, y, z)
                })
                .unwrap_or(Vec3::ZERO)
        };

        let name = event.get("name").and_then(|v| v.as_str()).unwrap_or("preview").to_string();
        let position = parse_vec3(event.get("position"));
        let direction = parse_vec3(event.get("direction"));
        let angle_range = parse_range(event.get("angleRange"));
        let radius_range = parse_range(event.get("radiusRange"));
        let jitter_vec = parse_vec3(event.get("jitter"));
        let jitter = Vec2::new(jitter_vec.x, jitter_vec.y);
        let gravity = event.get("gravity").and_then(|v| v.as_f64()).unwrap_or(-9.8) as f32;
        let radial_speed = parse_range(event.get("radialSpeed"));
        let up_speed = parse_range(event.get("upSpeed"));
        let lifetime = parse_range(event.get("lifetime"));
        let particle_count = event.get("particleCount").and_then(|v| v.as_i64()).unwrap_or(10) as usize;
        let pps = event.get("pps").and_then(|v| v.as_i64()).unwrap_or(0) as usize;
        let texture_path = event.get("texturePath").and_then(|v| v.as_str()).unwrap_or("").to_string();
        let texture_has_alpha = event.get("textureHasAlpha").and_then(|v| v.as_bool()).unwrap_or(false);
        let base_alpha = parse_range(event.get("baseAlpha"));
        let alpha_multiplier = event.get("alphaMultiplier").and_then(|v| v.as_f64()).unwrap_or(1.0) as f32;
        let alpha_power = event.get("alphaPower").and_then(|v| v.as_f64()).unwrap_or(1.0) as f32;
        let base_scale = parse_range(event.get("baseScale"));
        let scale_multiplier = event.get("scaleMultiplier").and_then(|v| v.as_f64()).unwrap_or(1.0) as f32;
        let scale_power = event.get("scalePower").and_then(|v| v.as_f64()).unwrap_or(1.0) as f32;

        // Parse colors array (ensure at least one default color)
        let mut colors: Vec<Vec4> = event.get("colors")
            .and_then(|v| v.as_array())
            .map(|arr| {
                arr.iter()
                    .filter_map(|c| {
                        c.as_array().map(|rgba| {
                            let r = rgba.get(0).and_then(|v| v.as_f64()).unwrap_or(1.0) as f32;
                            let g = rgba.get(1).and_then(|v| v.as_f64()).unwrap_or(1.0) as f32;
                            let b = rgba.get(2).and_then(|v| v.as_f64()).unwrap_or(1.0) as f32;
                            let a = rgba.get(3).and_then(|v| v.as_f64()).unwrap_or(1.0) as f32;
                            Vec4::new(r, g, b, a)
                        })
                    })
                    .collect()
            })
            .unwrap_or_default();
        if colors.is_empty() {
            colors.push(Vec4::new(1.0, 1.0, 1.0, 1.0));
        }

        let emitter = EmitterBlackboard {
            name,
            angle_rand: angle_range,
            radius_rand: radius_range,
            gravity,
            velocity: vec![Vec2::ZERO, Vec2::ZERO, Vec2::ZERO],
            particle_lifetime: lifetime,
            particle_scale: base_scale,
            particle_count,
            colors,
            texture_path: if texture_path.is_empty() { None } else { Some(texture_path) },
            texture_idx: None,
            texture_has_alpha,
            radial_speed,
            up_speed,
            jitter,
            base_alpha,
            alpha_multiplier,
            alpha_power,
            base_scale,
            scale_multiplier,
            scale_power,
            direction,
            pps: if pps > 0 { Some(pps) } else { None },
        };

        Some((emitter, position))
    }
}
