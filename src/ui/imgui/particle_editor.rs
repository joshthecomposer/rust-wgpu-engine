use std::borrow::Cow;

use glam::{Mat4, Quat, Vec2, Vec3, Vec4};
use glfw::{Action, MouseButton, PWindow, WindowEvent};
use imgui::{sys::{ImGuiKey, ImGuiKey_Backspace}, Drag, Io, Ui};

use crate::{animation::animation::Animator, camera::Camera, config::{entity_config::{EntityTypeHelper, UiEntityTypeHelper}, world_data::{EntityInstance, WorldData}}, entity_manager::EntityManager, enums_types::{CameraState, EntityType, Faction, SoundType}, gl_call, input::InputState, lights::Lights, particles::ParticleSystem, physics::PhysicsState, renderer::Renderer, sound::sound_manager::SoundManager, util::data_structure::HashMapGetPairMut};

pub struct ParticleEditor {
    emitter_type: String,
    // random ranges between two floats, use the same value for no deviation
    angle_rand: Vec2,
    radius_rand: Vec2,
    gravity: f32,
    velocity: Vec<Vec2>, // random values for each
    particle_lifetime: Vec2,
    particle_scale: Vec2,
    colors: Vec<Vec4>,
    particle_count: u32,
    texture: Option<u32>,

    em_idx: usize,
    new_pos: Vec3,
}

impl ParticleEditor {
    pub fn draw(
        &mut self, 
        ui: &mut Ui, 
        em: &mut EntityManager,
        ps: &mut PhysicsState,
        rdr: &mut Renderer,
        lm: &mut Lights,
        sm: &mut SoundManager,
        input:  &mut InputState,
        size: &[f32; 2],
        particles: &mut ParticleSystem,
    ) {
        ui.window("Particle Editor")
            .size([500.0, size[1]], imgui::Condition::FirstUseEver)
            .position([size[0] - 500.0, 0.0], imgui::Condition::FirstUseEver)
            .build(|| {
                let mut emitter_types: Vec<String> = particles
                    .emitter_data
                    .one_shot_data
                    .keys()
                    .map(|k| k.clone())
                    .collect();

                emitter_types.sort_unstable();

                ui.combo(
                    "Emitter Types",
                    &mut self.em_idx,
                    &emitter_types,
                    |s| Cow::Borrowed(&s),
                );

                // ===================== Create A New Emitter =====================
                ui.separator();
                ui.text("Create a new Emitter");
                ui.separator();
                
                if input.ray_just_hit {
                    self.new_pos = input.ray_pos;
                    input.ray_just_hit = false;
                }

                let mut arr = self.new_pos.to_array();

                if Drag::new("Emitter Position").speed(0.1).build_array(ui, &mut arr) {
                    self.new_pos = Vec3::from_array(arr);
                };

            });
    }
}

impl Default for ParticleEditor {
    fn default() -> Self {
        Self {
            emitter_type: String::new(),
            angle_rand: Vec2::ZERO,
            radius_rand: Vec2::ZERO,
            gravity: -9.8,
            velocity: vec![
                Vec2::ZERO,
                Vec2::new(0.0, 1.0),
                Vec2::ZERO
            ],
            particle_lifetime: Vec2::ZERO,
            particle_scale: Vec2::ZERO,
            colors: vec![
                Vec4::ONE
            ],
            particle_count: 1,
            texture: None,

            em_idx: 0,
            new_pos: Vec3::ZERO,
        }
    }
}
