use glfw::Context;
use crate::animation::animation_system;
use crate::config::game_config::GameConfig;
use crate::entity_manager::EntityManager;
use crate::enums_types::{AnimationType, CameraState, EntityType, Faction, PhysicsHandle, ShaderType, Transform};
use crate::input::{self, InputState};
use crate::state_machines::state_machine_system;
use crate::ui::game_ui::{do_ui, GameUiContext};
use crate::ui::message_queue::{MessageQueue, UiMessage};
use crate::util::data_structure::{HashMapGetPair, HashMapGetPairMut};
use crate::{combat_system, grounding_solver, items, movement_system, state_machines};
use crate::physics::PhysicsState;
use crate::renderer::Renderer;
use crate::sound::sound_manager::{self, SoundManager};
use crate::time::Time;
use crate::platform::Platform;
use crate::world::World;


pub struct Game {
    platform: Platform, // OS/window/events
    time: Time, // delta time, alpha time, elapsed time
    physics: PhysicsState,
    world: World, // ECS, terrain, particles, sim
    renderer: Renderer,
    sound: SoundManager,
    input: InputState,
    ui: GameUiContext,
    paused: bool,
    message_queue: MessageQueue,
    // imgui: SpagImgui,
}

impl Game {
    pub fn new() -> Self {
        let config = GameConfig::load_from_file("config/game_config.json");

        let platform = Platform::new("Spaghetti engine", 1920, 1080, false);
        let time = Time::new(60.0, platform.glfw.get_time() as f32);
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
            paused: false,
            message_queue: MessageQueue::new(),
        }
    }
    
    pub fn run(&mut self) {
        while !self.platform.window.should_close() {
            self.time.begin_frame(self.platform.glfw.get_time() as f32);

            let desired_cursor_mode = if self.paused {
                glfw::CursorMode::Normal
            } else if self.world.camera.move_state == CameraState::Locked {
                glfw::CursorMode::Normal
            } else {
                glfw::CursorMode::Disabled
            };

            self.platform.window.set_cursor_mode(desired_cursor_mode);

            while self.time.should_step() {
                self.time.begin_fixed_step();

                
                // Snapshot all transforms
                {
                    for curr in self.world.ecs.transforms.iter() {
                        self.world.ecs.prev_transforms.insert(curr.key(), curr.value().clone());
                    }
                }

                // Snapshot camera
                {
                    let cam = &mut self.world.camera;
                    cam.prev_pos     = cam.position;
                    cam.prev_forward = cam.forward;
                    cam.prev_up      = cam.up;
                    cam.prev_target  = cam.target;
                }

                // poll events and update input
                {
                    self.input.update();
                    self.platform.glfw.poll_events();
                    for (_, e) in glfw::flush_messages(&self.platform.events) {
                        match e {
                            glfw::WindowEvent::CursorPos(x, y) => {
                                if !self.paused {
                                    self.world.camera.process_mouse_input(&self.platform.window, &e);
                                }
                                self.input.mouse_pos_current = glam::vec2(x as f32, y as f32);
                            }
                            glfw::WindowEvent::MouseButton(b, a, _) => {
                                input::handle_mouse_input(b, a,
                                    glam::vec2(self.platform.fb_width as f32, self.platform.fb_height as f32),
                                    &self.world.camera, &mut self.world.ecs, &mut self.input, &self.physics);
                            }
                            glfw::WindowEvent::Key(k, _, a, _) => {
                                input::handle_keyboard_input(k, a, &mut self.input);
                                match (k,a) {
                                    (glfw::Key::Escape, glfw::Action::Press) => self.paused = !self.paused,
                                    _ => ()
                                }
                            }
                            _ => {}
                        }
                    }

                    if self.input.just_pressed(glfw::Key::F) {
                        self.world.camera.move_state = match self.world.camera.move_state {
                            CameraState::Free  => CameraState::Third,
                            CameraState::Third => CameraState::Locked,
                            CameraState::Locked=> CameraState::Free,
                        };
                    } 
                }
                
                let cam_basis = self.world.camera.basis_for_sim();

                if !self.paused {
                    grounding_solver::grounding_solver(
                        &mut self.world.ecs, 
                        &self.physics, 
                    );
                    state_machine_system::update(
                        &mut self.world.ecs, 
                        self.time.fixed_dt, 
                        &mut self.world.particles,
                        &self.input, 
                        &mut self.physics, 
                        &mut self.sound, 
                        &self.world.camera
                    );
                    items::update(&mut self.world.ecs, &mut self.physics);
                    animation_system::update(&mut self.world.ecs, self.time.fixed_dt);
                    combat_system::update(&mut self.world.ecs, self.time.fixed_dt, &mut self.physics, &mut self.world.particles);
                    self.world.ecs.update(&mut self.sound, &mut self.physics);

                    Self::push_weapon_kinematics_from_bones(&self.world.ecs, &mut self.physics);

                    match self.world.camera.move_state {
                        CameraState::Third | CameraState::Locked => {
                            movement_system::update(
                                &mut self.world.ecs,
                                self.time.fixed_dt,
                                &cam_basis,
                                &self.input,
                                &mut self.physics,
                            );
                        },
                        _ => (),
                    }

                    {
                        self.physics.step();
                    }
                }

                // post-physics pull RBs, handle events, snapshot current transforms
                Self::sync_transforms_from_physics(&mut self.world.ecs, &self.physics);
                Self::propagate_children(&mut self.world.ecs);


                self.time.end_fixed_step();
            }

            self.time.end_frame();

            self.update(); // Variable rate systems
            self.render(); // render uses time.alpha and interps
        }
    }

    pub fn update(&mut self) {
        self.world.camera.update(&self.world.ecs, self.time.dt, &self.physics, self.time.alpha, &self.input, self.platform.fb_width as f32 / self.platform.fb_height as f32);
        self.sound.update(&self.world.camera);
        self.world.lights.update(&self.time.dt);
        self.world.particles.update(self.time.dt);
    }

    pub fn render(&mut self) {
        self.renderer.draw(
            &self.world.ecs, 
            &mut self.world.camera, 
            &self.world.lights, 
            &mut self.sound,
            self.platform.fb_width,
            self.platform.fb_height,
            self.time.elapsed,
            true,
            &self.physics,
            self.time.alpha,
            &mut self.world.particles,
        );

        self.world.particles.render(
            self.renderer.shaders.get_mut(&ShaderType::Particles).unwrap(),
            &self.world.camera,
        );

        // Fix for mac scaled pixels garbage.
        let (win_w, win_h) = self.platform.window.get_size();                 // logical points
        let (fb_w,  fb_h)  = self.platform.window.get_framebuffer_size();     // physical pixels

        let sx = fb_w as f32 / win_w as f32;
        let sy = fb_h as f32 / win_h as f32;

        let mouse_fb = glam::vec2(self.input.mouse_pos_current.x * sx, self.input.mouse_pos_current.y * sy);

        let (ui_shader, font_shader) = self.renderer.shaders.get_pair_mut(&ShaderType::GameUi, &ShaderType::Text).unwrap();
        do_ui(
            self.platform.fb_width as f32, 
            self.platform.fb_height as f32, 
            mouse_fb,
            ui_shader,
            font_shader,
            &mut self.message_queue,
            self.paused, 
            self.platform.window.get_cursor_mode(), 
            &self.world.camera.move_state, 
            &self.input.keys_current, 
            &mut self.ui, 
            &mut false, //render_gizmos
            &self.input,
        );
        if self.message_queue.drain().contains(&UiMessage::WindowShouldClose) {
            self.platform.window.set_should_close(true)
        }
        self.platform.window.swap_buffers();
        self.platform.glfw.poll_events();
    }

    // PRIVATE //

    fn sync_transforms_from_physics(em: &mut EntityManager, ps: &PhysicsState) {
        // Collect first to avoid holding borrows to sets while mutating em.transforms
        let mut updates: Vec<(usize, glam::Vec3, glam::Quat)> = Vec::with_capacity(em.physics_handles.len());

        for ph in em.physics_handles.iter() {
            let id = ph.key();
            let PhysicsHandle { rigid_body, .. } = *ph.value();

            if let Some(rb) = ps.rigid_body_set.get(rigid_body) {
                let iso = rb.position(); // Isometry<f32>
                let pos = glam::Vec3::from_slice(iso.translation.vector.as_slice());
                // nalgebra stores a UnitQuaternion; take .coords (x,y,z,w)
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
    
    // This is purely for visual gizmos, has nothing to do with what is actually happening to
    // physics
    fn propagate_children(em: &mut EntityManager) {
        let non_weapon_joins = em.get_non_weapon_gizmo_joins();

        for (child_id, parent_id) in non_weapon_joins.iter() {
            let (pt, ct) = em.transforms.get_pair_mut(*parent_id, *child_id).unwrap();

            ct.rotation = pt.rotation;
            ct.position = pt.position;
        }

        let active_weapon_ids = em.get_active_weapon_ids();

        for id in active_weapon_ids.iter() {
            let parent_id = em.owners.get(*id).unwrap();
            let parent_animator = em.animators.get(*parent_id).unwrap();
            let current_key = parent_animator.current_animation.clone();
            let next_key = parent_animator.next_animation.clone();
            let blend_factor = parent_animator.blend_factor;
            let pt = em.transforms.get(*parent_id).unwrap();
            let pmatrix = glam::Mat4::from_scale_rotation_translation(pt.scale, pt.rotation, pt.position);
            let bone = em.skellingtons.get(*parent_id).unwrap();
            let item_bone = em.item_bones.get(*parent_id).unwrap();
            let rh_name = item_bone.rh_name.clone();

            let maybe_bone_world_model_space = if blend_factor > 0.0 && current_key != next_key {
                // SAFELY get both entries with mutable refs (no HashMap::remove)
                let (a1, a2) = {
                    match parent_animator.animations.get_pair(&current_key, &next_key) {
                        Some((a1, a2)) => (a1, a2),
                        None => {
                            eprintln!("WARNING: both animations do not exist in animator.get_pair_mut {}, {}", &current_key, &next_key);
                            return;
                        },
                    }
                };

                a1.get_raw_global_bone_transform_by_name_blended(
                    &rh_name,
                    bone,
                    pmatrix,
                    a2,
                    blend_factor,
                )
            } else {
                parent_animator.animations
                    .get(&current_key)
                    .unwrap()

                    .get_raw_global_bone_transform_by_name(
                        &rh_name,
                        bone,
                        pmatrix,
                    )
            };

            if let Some(raw_bone_transform) = maybe_bone_world_model_space {
                //let (_, rot, pos) = raw_bone_transform.to_scale_rotation_translation();

                let weapon_trans = em.transforms.get_mut(*id).unwrap();
                let corr = em.local_corrections
                    .get(*id)
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
                let final_m = raw_bone_transform * corr_m;

                let (_, rot, pos) = final_m.to_scale_rotation_translation();

                weapon_trans.position = pos;
                weapon_trans.rotation = rot;
                
                // sync the weapon's gizmo to it.
                if let Some(weapon_gizmo) = em.parents.iter().find(|p| p.value() == id) {
                    let t = em.transforms.get_mut(weapon_gizmo.key()).unwrap();

                    t.position = pos;
                    t.rotation = rot;
                }
            }

        }
    }
}
