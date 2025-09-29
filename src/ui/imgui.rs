use glam::{Mat4, Quat, Vec3};
use glfw::{Action, MouseButton, PWindow, WindowEvent};
use imgui::Drag;

use crate::{animation::animation::Animator, camera::Camera, config::world_data::{EntityInstance, WorldData}, entity_manager::EntityManager, enums_types::{CameraState, EntityType, Faction, SoundType}, gl_call, lights::Lights, renderer::Renderer, sound::sound_manager::SoundManager, util::data_structure::HashMapGetPairMut};

pub struct ImguiManager {
    pub imgui: imgui::Context,
    pub renderer: imgui_opengl_renderer::Renderer,
}

impl ImguiManager {
    pub fn new(window: &mut PWindow) -> Self {
        let mut imgui = imgui::Context::create();
        imgui.set_ini_filename(None);
        let renderer = imgui_opengl_renderer::Renderer::new(&mut imgui, |s| {
             window.get_proc_address(s) as *const _
         });
        Self {
            imgui,
            renderer,
        }
    }

    pub fn handle_imgui_event(&mut self, event: &WindowEvent) {
        let io = self.imgui.io_mut();
        match *event {
            // Mouse Buttons
            WindowEvent::MouseButton(btn, action, _) => {
                let pressed = action != Action::Release;
                match btn {
                    MouseButton::Button1 => {
                        io.mouse_down[0] = pressed;
                    },
                    MouseButton::Button2 => io.mouse_down[1] = pressed,
                    MouseButton::Button3 => io.mouse_down[2] = pressed,
                    _ => {}
                }
            }
            // Mouse Position
            WindowEvent::CursorPos(x, y) => {
                io.mouse_pos = [x as f32, y as f32];
            }
            // Scroll Wheel
            WindowEvent::Scroll(_x, scroll_y) => {
                io.mouse_wheel = scroll_y as f32;
            }
            // Text input
            WindowEvent::Char(ch) => {
                io.add_input_character(ch);
            }
            // Key press/release
            WindowEvent::Key(_key, _, action, _mods) => {
                let _pressed = action != Action::Release;
                // If you want to track ImGui’s internal key map, do something like:
                // io.keys_down[imgui_key_index] = pressed;
                // or handle advanced shortcuts, etc.
            }

            _ => {}
        }
    }

    pub fn draw(&mut self, window: &mut PWindow, width: f32, height: f32, delta: f32, lm: &mut Lights, rdr: &mut Renderer, sm: &mut SoundManager, camera: &Camera, em: &mut EntityManager) {
        {
            let io = self.imgui.io_mut();
            io.display_size = [width, height];
            io.delta_time   = delta;
        }
        let ui = self.imgui.frame();

        if camera.move_state == CameraState::Locked {
            ui.window("Lights")
                .size([500.0, 200.0], imgui::Condition::FirstUseEver)
                .position([50.0, 50.0], imgui::Condition::FirstUseEver)
                .build(|| {
                    ui.text("Controls for Various Lights");
                    ui.separator();
                    // ui.input_float("Dir Light distance", &mut lm.dir_light.distance).build();
                    if ui.slider("Dir Light X", -1.0, 1.0, &mut lm.dir_light.direction.x) {
                        lm.dir_light.view_pos.x = lm.dir_light.direction.x * lm.dir_light.distance;
                    };                                                      
                    if ui.slider("Dir Light Y", -1.0, 1.0, &mut lm.dir_light.direction.y) {
                        lm.dir_light.view_pos.y = lm.dir_light.direction.y * lm.dir_light.distance;
                    };                                                      
                    if ui.slider("Dir Light Z", -1.0, 1.0, &mut lm.dir_light.direction.z) {
                        lm.dir_light.view_pos.z = lm.dir_light.direction.z * lm.dir_light.distance;
                    };
                    if ui.slider("Dir Light distance",0.0, 100.0, &mut lm.dir_light.distance) {
                    };

                    ui.checkbox("Shadow Debug",&mut rdr.shadow_debug);


                    ui.separator();

                    if ui.slider("Ortho Near", 0.0, 10.0, &mut lm.near) {
                    };
                    if ui.slider("Ortho Far", 0.0, 500.0, &mut lm.far) {
                    };
                    if ui.slider("Bounds", 0.0, 100.0, &mut lm.bounds) {
                    };

                    // if ui.slider("Bias Scalar", 0.0, 0.3, &mut lm.bias_scalar) {
                    // };

                    if Drag::new("Bias Scalar")
                        .speed(0.0001)
                        .display_format("%.6f")
                        .build(ui, &mut lm.bias_scalar) {
                    }

                    lm.dir_light.view_pos.x = lm.dir_light.direction.x * lm.dir_light.distance;
                    lm.dir_light.view_pos.y = lm.dir_light.direction.y * lm.dir_light.distance;
                    lm.dir_light.view_pos.z = lm.dir_light.direction.z * lm.dir_light.distance;


                });

            ui.window("Sound")
                .size([500.0, 200.0], imgui::Condition::FirstUseEver)
                .position([550.0, 50.0], imgui::Condition::FirstUseEver)
                .build(|| {
                    ui.text("Controls Fmod Sounds");
                    ui.separator();

                    if ui.button("Pause") {
                        sm.stop_sound(&SoundType::Music);
                    }

                    if ui.button("Play") {
                        sm.play_sound_2d(SoundType::Music);
                    }

                    if ui.slider("Volume", 0.0, 1.0, &mut sm.master_volume) {
                        sm.set_master_volume(&SoundType::Music);
                    }

                });

            ui.window("Entity Editing")
                .size([500.0, 200.0], imgui::Condition::FirstUseEver)
                .position([50.0, 250.0], imgui::Condition::FirstUseEver)
                .build(|| {
                    if ui.button("Save Entity State") {
                        let mut save_data = WorldData {
                            entities: vec![]
                        };

                        for e_type in em.entity_types.iter() {
                            match *e_type.value() {
                                EntityType::Terrain => { continue; }
                                EntityType::OrcSword => { continue; }
                                _ => ()
                            }

                            if let (
                                Some(faction),
                                Some(trans),
                            ) = (
                                em.factions.get(e_type.key()),
                                em.transforms.get(e_type.key()),
                            ) {
                                if *faction == Faction::Gizmo {
                                    continue;
                                }

                                save_data.entities.push(
                                    EntityInstance {
                                        entity_type: e_type.value().clone(),
                                        faction: faction.clone(),
                                        position: trans.position.into(),
                                        rotation: (trans.rotation).into(),
                                        weapons: vec![],
                                        // rotation: (Quat::IDENTITY).into(),
                                        base_speed: em.base_speeds.get(e_type.key()).copied(),
                                        health: *em.healths.get(e_type.key()).unwrap(),
                                    }
                                );
                            }
                        }

                        save_data.write_readable_world_data("config/world_data.toml");
                    }

                    ui.separator();

                    for i in em.selected.iter() {
                        if let Some(trans) = em.transforms.get_mut(*i) {
                            ui.text(format!("Entity: {}, Type: {}", i, em.entity_types.get(*i).unwrap()));

                            let mut position = [trans.position.x, trans.position.y, trans.position.z];
                            let mut scale = [trans.scale.x];

                            // convert quat to euler angle degrees
                            let euler_degrees = trans.rotation.to_euler(glam::EulerRot::YXZ);
                            let mut rotation_deg = [
                                euler_degrees.0.to_degrees(),
                                euler_degrees.1.to_degrees(),
                                euler_degrees.2.to_degrees(),
                            ];

                            // position
                            if Drag::new("Position").speed(0.1).build_array(ui, &mut position) {
                                trans.position = Vec3::from(position);
                            }

                            //  scale
                            if Drag::new("Scale").speed(0.001).build_array(ui, &mut scale) {
                                trans.scale = Vec3::splat(scale[0]);
                            }

                            // rotation
                            if Drag::new("Rotation").speed(0.5).build_array(ui, &mut rotation_deg) {
                                let (y, x, z) = (
                                    rotation_deg[0].to_radians(),
                                    rotation_deg[1].to_radians(),
                                    rotation_deg[2].to_radians(),
                                );
                                trans.rotation = Quat::from_euler(glam::EulerRot::YXZ, y, x, z);
                            }
                        }

                        ui.separator();
                    }

                });

        }
        /*
        ui.window("Some Info")
            .size([400.0, 150.0], imgui::Condition::FirstUseEver)
            .position([1100.0, 50.0], imgui::Condition::FirstUseEver)
            .build(|| {
                let string = format!("x: {:.3}, y: {:.3}, z: {:.3}", camera.position.x, camera.position.y, camera.position.z);
                ui.label_text("Camera Position", string);

                let string = format!("x: {:.3}, y: {:.3}, z: {:.3}", camera.forward.x, camera.forward.y, camera.forward.z);
                ui.label_text("Camera Forward", string);

                if let Some(player_entry) = em.factions.iter().find(|f| f.value() == &Faction::Player) {
                    let player_key = player_entry.key();
                    let animator = em.animators.get_mut(player_key).unwrap();

                    let player_trans = em.transforms.get(player_key).unwrap();

                    let player_mat = Mat4::from_scale_rotation_translation(player_trans.scale, player_trans.rotation, player_trans.position);

                    let current_key = &animator.current_animation;
                    let next_key = &animator.next_animation;
                    let skellington = em.skellingtons.get(player_key).unwrap();

                    if let Some((current_anim, next_anim)) = animator.animations.get_pair_mut(current_key, next_key) {
                        let bone_trans = current_anim.get_raw_global_bone_transform_by_name_blended("mixamorig:Hips", skellington, player_mat, next_anim, animator.blend_factor).unwrap();

                        let bone_pos = bone_trans.w_axis.truncate();

                        let string = format!("x: {:.3}, y: {:.3}, z: {:.3}", bone_pos.x, bone_pos.y, bone_pos.z);
                        ui.label_text("Player mixamorig:Hips Position", string);
                    } else if let Some(current_anim) = animator.animations.get_mut(current_key) {
                        let bone_trans = current_anim.get_raw_global_bone_transform_by_name("mixamorig:Hips", skellington, player_mat).unwrap();

                        let bone_pos = bone_trans.w_axis.truncate();

                        let string = format!("x: {:.3}, y: {:.3}, z: {:.3}", bone_pos.x, bone_pos.y, bone_pos.z);
                        ui.label_text("Player mixamorig:Hips Position", string);
                    };

                    let string = format!("x: {:.3}, y: {:.3}, z: {:.3}", player_trans.position.x, player_trans.position.y, player_trans.position.z);
                    ui.label_text("Player World Position", string);

                };



            });
*/

        self.renderer.render(&mut self.imgui);
    }

}

