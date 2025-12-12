use std::rc::Rc;

use glam::{Quat, Vec2, Vec3, Vec4};
use rapier3d::prelude::{point, vector, InteractionGroups, Ray};
use slint::platform::software_renderer::{MinimalSoftwareWindow, PremultipliedRgbaColor};
use slint::platform::PointerEventButton;
use slint::platform::WindowEvent as SlintWindowEvent;
use slint::{LogicalPosition, ModelRc, PhysicalSize, SharedString, VecModel};
use winit::event::WindowEvent;

use crate::camera::Camera;
use crate::config::emitter_data::EmitterBlackboard;
use crate::config::world_data::EntityInstance;
use crate::entity_manager::EntityManager;
use crate::enums_types::CameraState;
use crate::gl_call;
use crate::input::{mouse_ray_from_screen, InputState};
use crate::lights::Lights;
use crate::particles::ParticleSystem;
use crate::physics::PhysicsState;
use crate::renderer::Renderer;
use crate::some_data::GROUP_TERRAIN;
use crate::sound::sound_manager::SoundManager;
use crate::ui::slint_platform::init_slint_platform;

slint::include_modules!();

/// State for entity editor selections
pub struct EntityEditorState {
    pub entity_type_index: usize,
    pub weapon_type_index: usize,
    pub faction_index: usize,
    pub create_mode: bool,
    pub include_weapon: bool,
    pub base_speed: f32,
    pub entity_editor_initialized: bool,
    pub emitter_editor_initialized: bool,
}

impl Default for EntityEditorState {
    fn default() -> Self {
        Self {
            entity_type_index: 0,
            weapon_type_index: 0,
            faction_index: 0,
            create_mode: false,
            include_weapon: false,
            base_speed: 0.0,
            entity_editor_initialized: false,
            emitter_editor_initialized: false,
        }
    }
}

/// Manages Slint UI rendering as an overlay on top of the OpenGL scene.
/// Uses software rendering to a pixel buffer, which is then uploaded to a GL texture.
pub struct EngineUiManager {
    window: Rc<MinimalSoftwareWindow>,
    engine_ui: EngineUI,
    pub editor_state: EntityEditorState,
    pixel_buffer: Vec<PremultipliedRgbaColor>,
    width: u32,
    height: u32,
    last_cursor_pos: LogicalPosition,
    gl_texture: u32,
    needs_texture_resize: bool,
    overlay_vao: u32,
    overlay_vbo: u32,
    ui_consumed_click: bool,
    // Particle editor state (for Slint-based editor, mirroring old ImGui behavior)
    particle_timer: f32,
    particle_did_render: bool,
    particle_staged_id: Option<usize>,
    last_pe_do_render: bool,
    last_pe_save_toggle: bool,
    last_pe_use_staged_texture_toggle: bool,
}

impl EngineUiManager {
    /// Create a new EngineUiManager. Must be called BEFORE any other Slint components are created.
    pub fn new(width: u32, height: u32) -> Self {
        let window = init_slint_platform(width, height);

        let engine_ui = EngineUI::new().unwrap();

        let pixel_count = (width * height) as usize;
        let pixel_buffer = vec![PremultipliedRgbaColor::default(); pixel_count];

        // Create GL texture with RGBA format
        let gl_texture = unsafe {
            let mut tex = 0u32;
            gl_call!(gl::GenTextures(1, &mut tex));
            gl_call!(gl::BindTexture(gl::TEXTURE_2D, tex));
            gl_call!(gl::TexParameteri(
                gl::TEXTURE_2D,
                gl::TEXTURE_MIN_FILTER,
                gl::LINEAR as i32
            ));
            gl_call!(gl::TexParameteri(
                gl::TEXTURE_2D,
                gl::TEXTURE_MAG_FILTER,
                gl::LINEAR as i32
            ));
            gl_call!(gl::TexParameteri(
                gl::TEXTURE_2D,
                gl::TEXTURE_WRAP_S,
                gl::CLAMP_TO_EDGE as i32
            ));
            gl_call!(gl::TexParameteri(
                gl::TEXTURE_2D,
                gl::TEXTURE_WRAP_T,
                gl::CLAMP_TO_EDGE as i32
            ));
            gl_call!(gl::TexImage2D(
                gl::TEXTURE_2D,
                0,
                gl::RGBA as i32,
                width as i32,
                height as i32,
                0,
                gl::RGBA,
                gl::UNSIGNED_BYTE,
                std::ptr::null(),
            ));
            gl_call!(gl::BindTexture(gl::TEXTURE_2D, 0));
            tex
        };

        // create VAO/VBO for fullscreen quad overlay
        let (overlay_vao, overlay_vbo) = unsafe {
            let mut vao = 0u32;
            let mut vbo = 0u32;
            gl_call!(gl::GenVertexArrays(1, &mut vao));
            gl_call!(gl::GenBuffers(1, &mut vbo));

            // fullscreen quad vertices: position (x,y) + texcoord (u,v)
            // note: Y is flipped for texture coordinates
            let quad_vertices: [f32; 24] = [
                // pos        // uv
                -1.0, 1.0, 0.0, 0.0, // top-left
                -1.0, -1.0, 0.0, 1.0, // bottom-left
                1.0, -1.0, 1.0, 1.0, // bottom-right
                -1.0, 1.0, 0.0, 0.0, // top-left
                1.0, -1.0, 1.0, 1.0, // bottom-right
                1.0, 1.0, 1.0, 0.0, // top-right
            ];

            gl_call!(gl::BindVertexArray(vao));
            gl_call!(gl::BindBuffer(gl::ARRAY_BUFFER, vbo));
            gl_call!(gl::BufferData(
                gl::ARRAY_BUFFER,
                (quad_vertices.len() * std::mem::size_of::<f32>()) as isize,
                quad_vertices.as_ptr() as *const _,
                gl::STATIC_DRAW,
            ));

            // position attribute (location 0)
            gl_call!(gl::VertexAttribPointer(
                0,
                2,
                gl::FLOAT,
                gl::FALSE,
                4 * std::mem::size_of::<f32>() as i32,
                std::ptr::null(),
            ));
            gl_call!(gl::EnableVertexAttribArray(0));

            // texcoord attribute (location 1)
            gl_call!(gl::VertexAttribPointer(
                1,
                2,
                gl::FLOAT,
                gl::FALSE,
                4 * std::mem::size_of::<f32>() as i32,
                (2 * std::mem::size_of::<f32>()) as *const _,
            ));
            gl_call!(gl::EnableVertexAttribArray(1));

            gl_call!(gl::BindVertexArray(0));
            (vao, vbo)
        };

        Self {
            window,
            engine_ui,
            editor_state: EntityEditorState::default(),
            pixel_buffer,
            width,
            height,
            last_cursor_pos: LogicalPosition::new(0.0, 0.0),
            gl_texture,
            needs_texture_resize: false,
            overlay_vao,
            overlay_vbo,
            ui_consumed_click: false,
            particle_timer: 0.0,
            particle_did_render: false,
            particle_staged_id: None,
            last_pe_do_render: false,
            last_pe_save_toggle: false,
            last_pe_use_staged_texture_toggle: false,
        }
    }

    /// Check if the cursor is over any visible UI panel
    fn is_cursor_over_ui(&self) -> bool {
        let x = self.last_cursor_pos.x;
        let y = self.last_cursor_pos.y;

        let player_panel_expanded = self.engine_ui.get_player_panel_expanded();
        let player_panel_height = if player_panel_expanded { 140.0 } else { 30.0 };
        let player_panel = (10.0, 10.0, 330.0, player_panel_height);

        if x >= player_panel.0
            && x <= player_panel.0 + player_panel.2
            && y >= player_panel.1
            && y <= player_panel.1 + player_panel.3
        {
            return true;
        }

        if self.engine_ui.get_editor_visible() {
            let editor_y = if player_panel_expanded { 160.0 } else { 50.0 };
            let editor_panel = (10.0, editor_y, 390.0, 500.0);

            if x >= editor_panel.0
                && x <= editor_panel.0 + editor_panel.2
                && y >= editor_panel.1
                && y <= editor_panel.1 + editor_panel.3
            {
                return true;
            }
        }

        // Right-side particle editor panel
        if self.engine_ui.get_editor_visible() {
            let particle_panel_expanded = self.engine_ui.get_particle_panel_expanded();
            let panel_height = if particle_panel_expanded {
                self.height as f32 - 20.0
            } else {
                30.0
            };
            // Matches Slint: x: parent.width - 430px; panel-width: 420px
            let panel_x = self.width as f32 - 430.0;
            let particle_panel = (panel_x, 10.0, 420.0, panel_height);

            if x >= particle_panel.0
                && x <= particle_panel.0 + particle_panel.2
                && y >= particle_panel.1
                && y <= particle_panel.1 + particle_panel.3
            {
                return true;
            }
        }

        false
    }

    /// Handle a winit window event. Returns true if Slint consumed the event.
    /// Also sets input.ui_consumed_click when mouse clicks are over UI elements.
    pub fn handle_window_event(&mut self, event: &WindowEvent, input: &mut InputState) -> bool {
        let slint_event = match event {
            WindowEvent::CursorMoved { position, .. } => {
                self.last_cursor_pos = LogicalPosition::new(position.x as f32, position.y as f32);
                Some(SlintWindowEvent::PointerMoved {
                    position: self.last_cursor_pos,
                })
            }
            WindowEvent::MouseInput { state, button, .. } => {
                let btn = match button {
                    winit::event::MouseButton::Left => PointerEventButton::Left,
                    winit::event::MouseButton::Right => PointerEventButton::Right,
                    winit::event::MouseButton::Middle => PointerEventButton::Middle,
                    _ => return false,
                };

                if *button == winit::event::MouseButton::Left {
                    if *state == winit::event::ElementState::Pressed {
                        let over_ui = self.is_cursor_over_ui();
                        self.ui_consumed_click = over_ui;
                        input.ui_consumed_click = over_ui;
                    } else {
                        input.ui_consumed_click = false;
                    }
                }

                Some(match state {
                    winit::event::ElementState::Pressed => SlintWindowEvent::PointerPressed {
                        position: self.last_cursor_pos,
                        button: btn,
                    },
                    winit::event::ElementState::Released => SlintWindowEvent::PointerReleased {
                        position: self.last_cursor_pos,
                        button: btn,
                    },
                })
            }
            WindowEvent::MouseWheel { delta, .. } => {
                let (dx, dy) = match delta {
                    winit::event::MouseScrollDelta::LineDelta(x, y) => (*x * 20.0, *y * 20.0),
                    winit::event::MouseScrollDelta::PixelDelta(pos) => (pos.x as f32, pos.y as f32),
                };
                Some(SlintWindowEvent::PointerScrolled {
                    position: self.last_cursor_pos,
                    delta_x: dx,
                    delta_y: dy,
                })
            }
            WindowEvent::Resized(size) => {
                self.resize(size.width, size.height);
                None // we handle resizing internally
            }
            // TODO: Add keyboard event handling
            _ => None,
        };

        if let Some(evt) = slint_event {
            self.window.dispatch_event(evt);
            self.is_cursor_over_ui()
        } else {
            false
        }
    }

    /// Resize the UI. Called automatically when WindowEvent::Resized is received.
    pub fn resize(&mut self, width: u32, height: u32) {
        if width == 0 || height == 0 {
            return;
        }

        self.width = width;
        self.height = height;

        let pixel_count = (width * height) as usize;
        self.pixel_buffer
            .resize(pixel_count, PremultipliedRgbaColor::default());

        // notify Slint about the new size
        self.window.set_size(PhysicalSize::new(width, height));

        // mark texture for resize on next render
        self.needs_texture_resize = true;
    }

    /// Update the UI each frame. Handles all ECS data extraction internally.
    /// Pass camera_state to control entity editor visibility.
    /// When create_mode is enabled and user clicks terrain, an entity is spawned.
    /// Also updates the right-side particle editor panel and its preview behavior.
    pub fn update(
        &mut self,
        em: &mut EntityManager,
        camera_state: CameraState,
        lights: &mut Lights,
        renderer: &mut Renderer,
        sound_manager: &mut SoundManager,
        input: &mut InputState,
        physics: &mut PhysicsState,
        camera: &Camera,
        screen_size: Vec2,
        particles: &mut ParticleSystem,
        dt: f32,
    ) {
        self.update_player_data(em);

        let editor_visible = camera_state != CameraState::Third;
        self.engine_ui.set_editor_visible(editor_visible);

        if editor_visible {
            let mut last_terrain_hit: Option<glam::Vec3> = None;
            if input.left_mouse_just_pressed() && !self.ui_consumed_click {
                last_terrain_hit = self.raycast_terrain(input, physics, camera, screen_size);
                if let Some(hit_pos) = last_terrain_hit {
                    self.engine_ui.set_pe_pos_x(hit_pos.x);
                    self.engine_ui.set_pe_pos_y(hit_pos.y);
                    self.engine_ui.set_pe_pos_z(hit_pos.z);
                }
            }

            self.update_entity_editor(em, lights, renderer, sound_manager);
            self.update_particle_editor(particles, input, dt);

            if self.editor_state.create_mode {
                if let Some(hit_pos) = last_terrain_hit {
                    self.spawn_entity_at_position(em, physics, hit_pos);

                    self.editor_state.create_mode = false;
                    self.engine_ui.set_create_mode(false);
                }
            }
        }

        slint::platform::update_timers_and_animations();
    }

    /// Raycast from mouse position to find terrain hit point
    fn raycast_terrain(
        &self,
        input: &InputState,
        physics: &PhysicsState,
        camera: &Camera,
        screen_size: Vec2,
    ) -> Option<glam::Vec3> {
        let cursor_pos = input.mouse_pos_current;
        let (ray_origin, ray_dir) = mouse_ray_from_screen(cursor_pos, screen_size, camera);

        let ray = Ray::new(
            point![ray_origin.x, ray_origin.y, ray_origin.z],
            vector![ray_dir.x, ray_dir.y, ray_dir.z],
        );

        let query_pipeline = physics.query_pipeline.as_ref()?;
        let colliders = &physics.collider_set;
        let bodies = &physics.rigid_body_set;

        let max_toi = 1000.0;
        let solid = true;

        if let Some((handle, toi)) = query_pipeline.cast_ray(
            bodies,
            colliders,
            &ray,
            max_toi,
            solid,
            InteractionGroups::all().into(),
        ) {
            let collider = physics.collider_set.get(handle)?;
            let groups = collider.collision_groups();

            if groups.memberships & GROUP_TERRAIN.into() != 0.into() {
                let hit_point = ray.point_at(toi);
                println!("[EntityEditor] Terrain hit at: {:?}", hit_point);
                return Some(glam::vec3(hit_point.x, hit_point.y, hit_point.z));
            }
        }

        None
    }

    /// Spawn an entity at the given position based on current editor state
    fn spawn_entity_at_position(
        &self,
        em: &mut EntityManager,
        physics: &mut PhysicsState,
        position: glam::Vec3,
    ) {
        let entity_types: Vec<String> = em.entity_type_register.keys().cloned().collect();
        let factions: Vec<String> = em.faction_register.iter().cloned().collect();

        let selected_type = entity_types
            .get(self.editor_state.entity_type_index)
            .cloned()
            .unwrap_or_default();
        let selected_faction = factions
            .get(self.editor_state.faction_index)
            .cloned()
            .unwrap_or_default();

        if selected_type.is_empty() || selected_faction.is_empty() {
            eprintln!("[EntityEditor] Cannot create entity: type or faction not selected");
            return;
        }

        let weapons = if self.editor_state.include_weapon {
            let weapon_types: Vec<String> = em.entity_type_register.keys().cloned().collect();
            weapon_types
                .get(self.editor_state.weapon_type_index)
                .map(|wt| {
                    vec![EntityInstance {
                        entity_type: wt.clone(),
                        faction: selected_faction.clone(),
                        position: glam::Vec3::ZERO,
                        rotation: Quat::IDENTITY,
                        weapons: None,
                        base_speed: None,
                        health: None,
                        jump_height: None,
                        cleanup_timer: None,
                    }]
                })
        } else {
            None
        };

        let instance = EntityInstance {
            entity_type: selected_type.clone(),
            faction: selected_faction,
            position,
            rotation: Quat::IDENTITY,
            weapons,
            base_speed: Some(self.editor_state.base_speed),
            jump_height: Some(1.0),
            health: Some(100.0),
            cleanup_timer: None,
        };

        println!(
            "[EntityEditor] Creating entity '{}' at {:?}",
            selected_type, position
        );

        let parent_id = em.create_entity(&instance, physics);
        em.populate_inventory(parent_id, &instance, physics);
    }

    fn update_player_data(&self, em: &EntityManager) {
        let maybe_player = em.factions.iter().find(|e| *e.value() == "Player");

        if let Some(player_entry) = maybe_player {
            let player_id = player_entry.key();

            let position = em
                .transforms
                .get(player_id)
                .map(|t| t.position)
                .unwrap_or(glam::Vec3::ZERO);

            let (player_state, attack_state) = em
                .player_controllers
                .get(player_id)
                .map(|pc| (pc.state.to_string(), pc.attack_state.to_string()))
                .unwrap_or(("N/A".to_string(), "N/A".to_string()));

            let current_animation = em
                .animators
                .get(player_id)
                .map(|a| a.current_animation.to_string())
                .unwrap_or("N/A".to_string());

            self.engine_ui.set_position_text(SharedString::from(format!(
                "x: {:.2} y: {:.2} z: {:.2}",
                position.x, position.y, position.z
            )));
            self.engine_ui
                .set_player_state_text(SharedString::from(player_state));
            self.engine_ui
                .set_attack_state_text(SharedString::from(attack_state));
            self.engine_ui
                .set_current_animation_text(SharedString::from(current_animation));
        }
    }

    fn update_entity_editor(
        &mut self,
        em: &EntityManager,
        lights: &mut Lights,
        renderer: &mut Renderer,
        sound_manager: &mut SoundManager,
    ) {
        let dir_x = self.engine_ui.get_dir_light_x();
        let dir_y = self.engine_ui.get_dir_light_y();
        let dir_z = self.engine_ui.get_dir_light_z();
        let dir_dist = self.engine_ui.get_dir_light_distance();

        lights.dir_light.direction = glam::vec3(dir_x, dir_y, dir_z).normalize();
        lights.dir_light.distance = dir_dist;

        renderer.shadow_debug = self.engine_ui.get_shadow_debug();

        lights.near = self.engine_ui.get_ortho_near();
        lights.far = self.engine_ui.get_ortho_far();
        lights.bounds = self.engine_ui.get_bounds();
        lights.bias_scalar = self.engine_ui.get_bias_scalar();

        // TODO: fix this
        let volume = self.engine_ui.get_master_volume();
        sound_manager.master_volume = volume;

        if !self.editor_state.entity_editor_initialized {
            let entity_types: Vec<SharedString> = em
                .entity_type_register
                .keys()
                .map(|k| SharedString::from(k.clone()))
                .collect();

            let factions: Vec<SharedString> = em
                .faction_register
                .iter()
                .map(|f| SharedString::from(f.clone()))
                .collect();

            self.engine_ui
                .set_entity_types(ModelRc::new(VecModel::from(entity_types)));
            self.engine_ui
                .set_factions(ModelRc::new(VecModel::from(factions)));

            self.editor_state.entity_editor_initialized = true;
        }

        self.editor_state.entity_type_index = self.engine_ui.get_entity_type_index() as usize;
        self.editor_state.faction_index = self.engine_ui.get_faction_index() as usize;
        self.editor_state.weapon_type_index = self.engine_ui.get_weapon_type_index() as usize;
        self.editor_state.include_weapon = self.engine_ui.get_include_weapon();
        self.editor_state.create_mode = self.engine_ui.get_create_mode();
    }

    /// Build an EmitterBlackboard payload from the current Slint particle editor properties.
    /// This mirrors the data that the old ImGui ParticleEditor produced per UiEmitterBlackboard.
    fn build_particle_payload_from_ui(&self) -> EmitterBlackboard {
        let name = self.engine_ui.get_pe_emitter_name().to_string();

        let angle_rand = Vec2::new(
            self.engine_ui.get_pe_angle_min(),
            self.engine_ui.get_pe_angle_max(),
        );

        let radius_rand = Vec2::new(
            self.engine_ui.get_pe_radius_min(),
            self.engine_ui.get_pe_radius_max(),
        );

        let gravity = self.engine_ui.get_pe_gravity();

        // velocity was used in the original data format but is no longer consumed by the
        // still populating this bad boy for compatibility
        let velocity = vec![
            Vec2::new(0.0, 0.0),
            Vec2::new(1.0, 2.0),
            Vec2::new(0.0, 0.0),
        ];

        let particle_lifetime = Vec2::new(
            self.engine_ui.get_pe_lifetime_min(),
            self.engine_ui.get_pe_lifetime_max(),
        );

        // legacy field
        let particle_scale = Vec2::new(0.0, 0.0);

        let particle_count = self.engine_ui.get_pe_particle_count().max(1.0).round() as usize;

        let color = Vec4::new(
            self.engine_ui.get_pe_color_r(),
            self.engine_ui.get_pe_color_g(),
            self.engine_ui.get_pe_color_b(),
            self.engine_ui.get_pe_color_a(),
        );
        let colors = vec![color];

        let texture_path_str = self.engine_ui.get_pe_texture_path().to_string();
        let texture_path = if texture_path_str.is_empty() {
            None
        } else {
            Some(texture_path_str)
        };

        let radial_speed = Vec2::new(
            self.engine_ui.get_pe_radial_speed_min(),
            self.engine_ui.get_pe_radial_speed_max(),
        );

        let up_speed = Vec2::new(
            self.engine_ui.get_pe_up_speed_min(),
            self.engine_ui.get_pe_up_speed_max(),
        );

        let jitter = Vec2::new(
            self.engine_ui.get_pe_jitter_min(),
            self.engine_ui.get_pe_jitter_max(),
        );

        let base_alpha = Vec2::new(
            self.engine_ui.get_pe_base_alpha_min(),
            self.engine_ui.get_pe_base_alpha_max(),
        );

        let alpha_multiplier = self.engine_ui.get_pe_alpha_multiplier();
        let alpha_power = self.engine_ui.get_pe_alpha_power();

        let base_scale = Vec2::new(
            self.engine_ui.get_pe_base_scale_min(),
            self.engine_ui.get_pe_base_scale_max(),
        );

        let scale_multiplier = self.engine_ui.get_pe_scale_multiplier();
        let scale_power = self.engine_ui.get_pe_scale_power();

        let direction = Vec3::new(
            self.engine_ui.get_pe_dir_x(),
            self.engine_ui.get_pe_dir_y(),
            self.engine_ui.get_pe_dir_z(),
        );

        let pps_value = self.engine_ui.get_pe_pps();
        let pps = if pps_value > 0.0 {
            Some(pps_value as usize)
        } else {
            None
        };

        EmitterBlackboard {
            name,
            angle_rand,
            radius_rand,
            gravity,
            velocity,
            particle_lifetime,
            particle_scale,
            particle_count,
            colors,
            texture_path,
            texture_idx: None,
            texture_has_alpha: self.engine_ui.get_pe_texture_has_alpha(),
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
            pps,
        }
    }

    /// Update the particle editor panel (right side) and drive particle preview/save
    /// behavior, mirroring the old ImGui ParticleEditor.
    fn update_particle_editor(
        &mut self,
        particles: &mut ParticleSystem,
        _input: &mut InputState,
        dt: f32,
    ) {
        let mut emitter_types: Vec<String> = particles
            .emitter_data
            .one_shot_data
            .keys()
            .cloned()
            .collect();
        emitter_types.sort_unstable();

        if !self.editor_state.emitter_editor_initialized {
            let emitter_model: Vec<SharedString> = emitter_types
                .iter()
                .cloned()
                .map(SharedString::from)
                .collect();
            self.engine_ui
                .set_emitter_types(ModelRc::new(VecModel::from(emitter_model)));

            self.editor_state.emitter_editor_initialized = true;
        }

        let staged_texture = self.engine_ui.get_staged_texture().to_string();
        let use_staged_toggle = self.engine_ui.get_pe_use_staged_texture_toggle();
        if use_staged_toggle != self.last_pe_use_staged_texture_toggle && use_staged_toggle {
            if !staged_texture.is_empty() {
                self.engine_ui
                    .set_pe_texture_path(SharedString::from(staged_texture.clone()));
            }
        }
        self.last_pe_use_staged_texture_toggle = use_staged_toggle;

        let mut payload = self.build_particle_payload_from_ui();

        let do_render = self.engine_ui.get_pe_do_render();
        let just_enabled = do_render && !self.last_pe_do_render;
        particles.render_staged_emitters = do_render;

        if do_render {
            let origin = Vec3::new(
                self.engine_ui.get_pe_pos_x(),
                self.engine_ui.get_pe_pos_y(),
                self.engine_ui.get_pe_pos_z(),
            );

            if payload.pps.is_some() {
                // CONTINUOUS EMITTER PREVIEW
                if let Some(id) = self.particle_staged_id {
                    particles.edit_staged_emitter(id, &payload, origin);
                } else if self.particle_timer >= 1.0 || just_enabled {
                    // first time spawning
                    let id = particles.spawn_oneshot_editor_emitter(&payload, origin);
                    self.particle_staged_id = Some(id);
                    self.particle_did_render = true;
                }
            } else {
                // ONESHOT PREVIEW
                if self.particle_timer >= 1.0 || just_enabled {
                    particles.spawn_oneshot_editor_emitter(&payload, origin);
                    self.particle_did_render = true;
                }
            }
        }

        if self.particle_did_render {
            self.particle_timer -= self.particle_timer;
            self.particle_did_render = false;
        }
        self.particle_timer += dt;

        let save_toggle = self.engine_ui.get_pe_save_toggle();
        if save_toggle != self.last_pe_save_toggle && save_toggle {
            let name = self.engine_ui.get_pe_emitter_name().to_string();
            if !name.is_empty() {
                if !emitter_types.contains(&name) {
                    payload.name = name.clone();
                    particles
                        .emitter_data
                        .one_shot_data
                        .insert(name.clone(), payload.clone());
                    particles
                        .emitter_data
                        .write_to_file("config/particle_emitters.toml");
                } else {
                    eprintln!(
                        "[ParticleEditor] emitter not saved, name '{}' already exists",
                        name
                    );
                }
            } else {
                eprintln!("[ParticleEditor] cannot save emitter: name is empty");
            }
        }
        self.last_pe_save_toggle = save_toggle;
        self.last_pe_do_render = do_render;
    }

    /// Render the UI to the internal pixel buffer and upload to GL texture.
    /// Call this after update() but before drawing the overlay.
    pub fn render(&mut self) {
        // resize GL texture if needed
        if self.needs_texture_resize {
            unsafe {
                gl_call!(gl::BindTexture(gl::TEXTURE_2D, self.gl_texture));
                gl_call!(gl::TexImage2D(
                    gl::TEXTURE_2D,
                    0,
                    gl::RGBA as i32,
                    self.width as i32,
                    self.height as i32,
                    0,
                    gl::RGBA,
                    gl::UNSIGNED_BYTE,
                    std::ptr::null(),
                ));
                gl_call!(gl::BindTexture(gl::TEXTURE_2D, 0));
            }
            self.needs_texture_resize = false;
        }

        self.window.draw_if_needed(|renderer| {
            renderer.render(&mut self.pixel_buffer, self.width as usize);
        });

        // upload to GL texture
        unsafe {
            gl::BindTexture(gl::TEXTURE_2D, self.gl_texture);
            gl::TexSubImage2D(
                gl::TEXTURE_2D,
                0,
                0,
                0,
                self.width as i32,
                self.height as i32,
                gl::RGBA,
                gl::UNSIGNED_BYTE,
                self.pixel_buffer.as_ptr() as *const _,
            );
            gl::BindTexture(gl::TEXTURE_2D, 0);
        }
    }

    /// Draw the UI overlay on screen. Call this after render() and after all other rendering.
    /// This draws a fullscreen quad with the Slint UI texture blended on top.
    ///
    /// The shader should be the UiOverlay shader with `ui_texture` uniform set to texture unit 0.
    pub fn draw_overlay(&self, shader: &crate::shaders::Shader) {
        unsafe {
            // save current state
            let mut depth_test_enabled = 0i32;
            let mut blend_enabled = 0i32;
            gl_call!(gl::GetIntegerv(gl::DEPTH_TEST, &mut depth_test_enabled));
            gl_call!(gl::GetIntegerv(gl::BLEND, &mut blend_enabled));

            // set up for 2D overlay rendering
            gl_call!(gl::Disable(gl::DEPTH_TEST));
            gl_call!(gl::Enable(gl::BLEND));
            // use premultiplied alpha blending since Slint uses PremultipliedRgbaColor
            gl_call!(gl::BlendFunc(gl::ONE, gl::ONE_MINUS_SRC_ALPHA));

            // activate shader and bind texture
            shader.activate();
            gl_call!(gl::ActiveTexture(gl::TEXTURE0));
            gl_call!(gl::BindTexture(gl::TEXTURE_2D, self.gl_texture));

            // draw the fullscreen quad
            gl_call!(gl::BindVertexArray(self.overlay_vao));
            gl_call!(gl::DrawArrays(gl::TRIANGLES, 0, 6));
            gl_call!(gl::BindVertexArray(0));
            gl_call!(gl::BindTexture(gl::TEXTURE_2D, 0));

            // restore previous state
            if depth_test_enabled != 0 {
                gl_call!(gl::Enable(gl::DEPTH_TEST));
            }
            if blend_enabled == 0 {
                gl_call!(gl::Disable(gl::BLEND));
            }
        }
    }

    #[allow(dead_code)]
    /// Get the GL texture ID for drawing as an overlay.
    pub fn texture(&self) -> u32 {
        self.gl_texture
    }

    #[allow(dead_code)]
    /// Get the current UI size.
    pub fn size(&self) -> (u32, u32) {
        (self.width, self.height)
    }
}

impl Drop for EngineUiManager {
    fn drop(&mut self) {
        unsafe {
            gl_call!(gl::DeleteTextures(1, &self.gl_texture));
            gl_call!(gl::DeleteVertexArrays(1, &self.overlay_vao));
            gl_call!(gl::DeleteBuffers(1, &self.overlay_vbo));
        }
    }
}
