use std::borrow::Cow;

use glam::{Mat4, Quat, Vec2, Vec3, Vec4};
use glfw::{Action, MouseButton, PWindow, WindowEvent};
use imgui::{sys::{ImGuiKey, ImGuiKey_Backspace}, Drag, Io, Ui};

use crate::{animation::animation::Animator, camera::Camera, config::{emitter_data::{EmitterBlackboard, UiEmitterBlackboard}, entity_config::{EntityTypeHelper, UiEntityTypeHelper}, world_data::{EntityInstance, WorldData}}, entity_manager::EntityManager, enums_types::{CameraState, EntityType, Faction, SoundType}, gl_call, input::InputState, lights::Lights, particles::ParticleSystem, physics::PhysicsState, renderer::Renderer, sound::sound_manager::SoundManager, util::data_structure::HashMapGetPairMut};

pub struct ParticleEditor {
    pub new_emitter: UiEmitterBlackboard,

    em_idx: usize,
    new_pos: [f32; 3],
    current_color: [f32; 4],
    clr_idx: usize,
    render_emitter: bool,
    timer: f32,
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
        dt: f32,
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
                    self.new_pos = input.ray_pos.into();
                    input.ray_just_hit = false;
                }

                if Drag::new("Emitter Position").speed(0.1).build_array(ui, &mut self.new_pos) {};
                if Drag::new("Emitter Direction").speed(0.1).build_array(ui, &mut self.new_emitter.direction) {};

                ui.input_text("Emitter Name", &mut self.new_emitter.name)
                    .build();

                if Drag::new("Angle Range").speed(0.1).build_array(ui, &mut self.new_emitter.angle_rand) {};
                if Drag::new("Radius Range").speed(0.1).build_array(ui, &mut self.new_emitter.radius_rand) {};
                if Drag::new("Jitter").speed(0.1).build_array(ui, &mut self.new_emitter.jitter) {};
                
                if Drag::new("Gravity").speed(0.1).build(ui, &mut self.new_emitter.gravity) {};

                if Drag::new("Radial Speed").speed(0.1).build_array(ui, &mut self.new_emitter.radial_speed) {};
                if Drag::new("Upward Speed").speed(0.1).build_array(ui, &mut self.new_emitter.up_speed) {};
                if Drag::new("Particle Lifetime").speed(0.1).build_array(ui, &mut self.new_emitter.particle_lifetime) {};
                //if Drag::new("Particle Scale").speed(0.001).build_array(ui, &mut self.new_emitter.particle_scale) {};

                ui.input_int("Particle Count", &mut self.new_emitter.particle_count)
                    .build();

                if Drag::new("Particle Color").speed(0.01).build_array(ui, &mut self.current_color) {};

                if ui.button("Add color") {
                    self.new_emitter.colors.push(self.current_color.clone());
                };
                let new = self.current_color;
                ui.color_button(
                    "##new color preview",
                    [new[0], new[1], new[2], new[3]],
                );

                ui.combo(
                    "Colors",
                    &mut self.clr_idx,
                    &self.new_emitter.colors,
                    |c| {
                        let label = format!("{:.2}, {:.2}, {:.2}, {:.2}", c[0], c[1], c[2], c[3]);
                        Cow::Owned(label)
                    },
                );
                
                if let Some(color) = self.new_emitter.colors.get(self.clr_idx) {
                    ui.color_button(
                        "##selected_color",
                        [color[0], color[1], color[2], color[3]],
                    );
                }

                if ui.button("Remove Color") {
                    self.new_emitter.colors.remove(self.clr_idx);
                };

                ui.input_text("Texture Path", &mut self.new_emitter.texture_path)
                    .build();

                if Drag::new("Start Alpha").speed(0.01).build_array(ui, &mut self.new_emitter.base_alpha) {};
                if Drag::new("Alpha Multiplier").speed(0.01).build(ui, &mut self.new_emitter.alpha_multiplier) {};
                ui.slider("Alpha Curve (1.0 is linear)", 0.0, 1.0, &mut self.new_emitter.alpha_power);

                if Drag::new("Start Scale").speed(0.001).build_array(ui, &mut self.new_emitter.base_scale) {};
                if Drag::new("Scale Multiplier").speed(0.001).build(ui, &mut self.new_emitter.scale_multiplier) {};
                ui.slider("Scale Curve (1.0 is linear)", 0.0, 1.0, &mut self.new_emitter.scale_power);

                ui.checkbox("Render Emitter", &mut self.render_emitter);

                if self.render_emitter && self.timer >= 1.0 {
                    let final_colors: Vec<Vec4> = if self.new_emitter.colors.len() > 0 {
                        self.new_emitter.colors.iter().map(|arr| Vec4::from_array(*arr)).collect()
                    } else {
                        vec![ self.current_color.into() ]
                    };

                    let texture_path = if self.new_emitter.texture_path.is_empty() {
                        None
                    } else {
                        Some(self.new_emitter.texture_path.clone())
                    };

                    let payload = EmitterBlackboard {
                        name: self.new_emitter.name.clone(),
                        angle_rand: self.new_emitter.angle_rand.into(),
                        radius_rand: Vec2::from_array(self.new_emitter.radius_rand),
                        gravity: self.new_emitter.gravity,
                        velocity: vec![
                            Vec2::from_array(self.new_emitter.velocity_x),
                            Vec2::from_array(self.new_emitter.velocity_y),
                            Vec2::from_array(self.new_emitter.velocity_z),
                        ],
                        particle_lifetime: self.new_emitter.particle_lifetime.into(),
                        particle_scale: self.new_emitter.particle_scale.into(),
                        particle_count: self.new_emitter.particle_count as usize,
                        colors: final_colors,
                        texture_path: texture_path,
                        texture_idx: None,
                        radial_speed: self.new_emitter.radial_speed.into(),
                        up_speed: self.new_emitter.up_speed.into(),
                        jitter: self.new_emitter.jitter.into(),

                        base_alpha: self.new_emitter.base_alpha.into(),
                        alpha_multiplier: self.new_emitter.alpha_multiplier,
                        alpha_power: self.new_emitter.alpha_power,

                        base_scale: self.new_emitter.base_scale.into(),
                        scale_multiplier: self.new_emitter.scale_multiplier,
                        scale_power: self.new_emitter.scale_power,

                        direction: self.new_emitter.direction.into(),
                    };

                    particles.spawn_oneshot_editor_emitter(payload, self.new_pos.into());

                    self.timer -= self.timer;
                }

                self.timer += dt;

            });
    }
}

impl Default for ParticleEditor {
    fn default() -> Self {
        Self {
            new_emitter: UiEmitterBlackboard::default(),

            em_idx: 0,
            new_pos: [0.0, 0.0, 0.0],
            current_color: [1.0, 1.0, 1.0, 1.0],
            clr_idx: 0,
            render_emitter: false,
            timer: 0.0,
        }
    }
}
