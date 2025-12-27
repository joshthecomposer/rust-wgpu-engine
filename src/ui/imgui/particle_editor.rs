use std::borrow::Cow;

use glam::{Vec2, Vec4};
use imgui::{Drag, Ui};

use crate::{
    config::{
        emitter_data::{EmitterBlackboard, UiEmitterBlackboard},
        Config,
    },
    entity_manager::EntityManager,
    input::InputState,
    lights::Lights,
    particles::ParticleSystem,
    physics::PhysicsState,
    renderer::Renderer,
    sound::sound_manager::SoundManager,
    ui::message_queue::{MessageQueue, UiMessage},
};

pub struct ParticleEditor {
    pub new_emitters: Vec<UiEmitterBlackboard>,
    pub staged_texture: String,

    em_idx: usize,
    new_pos: [f32; 3],
    current_color: [f32; 4],
    color_intensity: f32,
    clr_idx: usize,
    do_render: bool,
    did_render: bool,
    timer: f32,
    payloads: Vec<EmitterBlackboard>,
}

impl ParticleEditor {
    pub fn draw(
        &mut self,
        ui: &mut Ui,
        _em: &mut EntityManager,
        _ps: &mut PhysicsState,
        _rdr: &mut Renderer,
        _lm: &mut Lights,
        _sm: &mut SoundManager,
        input: &mut InputState,
        size: &[f32; 2],
        particles: &mut ParticleSystem,
        dt: f32,
        message_queue: &mut MessageQueue,
    ) {
        ui.window("Particle Editor")
            .size([500.0, size[1]], imgui::Condition::FirstUseEver)
            .position([0.0, 75.0], imgui::Condition::FirstUseEver)
            .collapsed(true, imgui::Condition::FirstUseEver)
            .build(|| {
                let mut emitter_types: Vec<String> = particles
                    .emitter_data
                    .one_shot_data
                    .keys()
                    .map(|k| k.clone())
                    .collect();

                emitter_types.sort_unstable();

                ui.combo("Emitter Types", &mut self.em_idx, &emitter_types, |s| {
                    Cow::Borrowed(&s)
                });

                ui.separator();
                ui.text("Create a new Emitter ");
                ui.separator();

                ui.input_text("Staged Texture", &mut self.staged_texture)
                    .build();

                if ui.button("Add Another Emitter") {
                    self.new_emitters.push(UiEmitterBlackboard::default());
                };

                for (i, new_emitter) in self.new_emitters.iter_mut().enumerate() {
                    let id_token = ui.push_id(format!("{}", i));
                    ui.separator();

                    if input.ray_just_hit {
                        self.new_pos = input.ray_pos.into();
                        input.ray_just_hit = false;
                    }

                    if Drag::new("Emitter Position")
                        .speed(0.1)
                        .build_array(ui, &mut self.new_pos)
                    {};
                    if Drag::new("Emitter Direction")
                        .speed(0.1)
                        .build_array(ui, &mut new_emitter.direction)
                    {};

                    ui.input_text("Emitter Name", &mut new_emitter.name).build();

                    if Drag::new("Angle Range")
                        .speed(0.1)
                        .build_array(ui, &mut new_emitter.angle_rand)
                    {};
                    if Drag::new("Radius Range")
                        .speed(0.1)
                        .build_array(ui, &mut new_emitter.radius_rand)
                    {};
                    if Drag::new("Jitter")
                        .speed(0.1)
                        .build_array(ui, &mut new_emitter.jitter)
                    {};

                    if Drag::new("Gravity")
                        .speed(0.1)
                        .build(ui, &mut new_emitter.gravity)
                    {};

                    if Drag::new("Radial Speed")
                        .speed(0.1)
                        .build_array(ui, &mut new_emitter.radial_speed)
                    {};
                    if Drag::new("Upward Speed")
                        .speed(0.1)
                        .build_array(ui, &mut new_emitter.up_speed)
                    {};
                    if Drag::new("Particle Lifetime")
                        .speed(0.01)
                        .build_array(ui, &mut new_emitter.particle_lifetime)
                    {};
                    //if Drag::new("Particle Scale").speed(0.001).build_array(ui, &mut new_emitter.particle_scale) {};

                    ui.input_int("Particle Count", &mut new_emitter.particle_count)
                        .build();

                    ui.input_int("PPS (continuous emitter)", &mut new_emitter.pps)
                        .build();

                    if Drag::new("Particle Color")
                        .speed(0.01)
                        .build_array(ui, &mut self.current_color)
                    {};

                    if ui.button("Add color") {
                        let base = Vec4::from_array(self.current_color);
                        let rgb = base.truncate() * self.color_intensity; // Vec3
                        let a = base.w; // keep alpha

                        new_emitter.colors.push([rgb.x, rgb.y, rgb.z, a]);
                    }

                    let base = Vec4::from_array(self.current_color);
                    let hdr_rgb = base.truncate() * self.color_intensity; // RGB only

                    let max_c = hdr_rgb.x.max(hdr_rgb.y.max(hdr_rgb.z));
                    let scale = if max_c > 1.0 { 1.0 / max_c } else { 1.0 };
                    let preview_rgb = hdr_rgb * scale;

                    let preview = [
                        preview_rgb.x, // now in [0,1] but same hue
                        preview_rgb.y,
                        preview_rgb.z,
                        base.w, // or 1.0 if you want
                    ];

                    ui.color_button("##new color preview", preview);

                    if Drag::new("Color Intensity")
                        .speed(0.1)
                        .build(ui, &mut self.color_intensity)
                    {};

                    ui.combo("Colors", &mut self.clr_idx, &new_emitter.colors, |c| {
                        let label = format!("{:.2}, {:.2}, {:.2}, {:.2}", c[0], c[1], c[2], c[3]);
                        Cow::Owned(label)
                    });

                    if let Some(color) = new_emitter.colors.get(self.clr_idx) {
                        ui.color_button(
                            "##selected_color",
                            [color[0], color[1], color[2], color[3]],
                        );
                    }

                    if ui.button("Remove Color") {
                        new_emitter.colors.remove(self.clr_idx);
                    };

                    if ui.button("Use Staged Texture") {
                        new_emitter.texture_path = self.staged_texture.clone();
                    };

                    ui.input_text("Texture Path", &mut new_emitter.texture_path)
                        .build();

                    ui.checkbox("Texture Has Alpha", &mut new_emitter.texture_has_alpha);

                    if Drag::new("Start Alpha")
                        .speed(0.01)
                        .build_array(ui, &mut new_emitter.base_alpha)
                    {};
                    if Drag::new("Alpha Multiplier")
                        .speed(0.01)
                        .build(ui, &mut new_emitter.alpha_multiplier)
                    {};
                    ui.slider(
                        "Alpha Curve (1.0 is linear)",
                        0.0,
                        1.0,
                        &mut new_emitter.alpha_power,
                    );

                    if Drag::new("Start Scale")
                        .speed(0.001)
                        .build_array(ui, &mut new_emitter.base_scale)
                    {};
                    if Drag::new("Scale Multiplier")
                        .speed(0.001)
                        .build(ui, &mut new_emitter.scale_multiplier)
                    {};
                    ui.slider(
                        "Scale Curve (1.0 is linear)",
                        0.0,
                        1.0,
                        &mut new_emitter.scale_power,
                    );

                    id_token.pop();
                    ui.separator();
                }

                // ===========================================================
                // Gather Emitter Data
                // ===========================================================
                self.payloads.clear();

                for (i, new_emitter) in self.new_emitters.iter_mut().enumerate() {
                    let id_token = ui.push_id(format!("{}", i));

                    let final_colors: Vec<Vec4> = if new_emitter.colors.len() > 0 {
                        new_emitter
                            .colors
                            .iter()
                            .map(|arr| Vec4::from_array(*arr))
                            .collect()
                    } else {
                        let v = Vec4::from_array(self.current_color);
                        vec![Vec4::new(
                            v.x * self.color_intensity,
                            v.y * self.color_intensity,
                            v.z * self.color_intensity,
                            1.0,
                        )]
                    };

                    let texture_path = if new_emitter.texture_path.is_empty() {
                        None
                    } else {
                        Some(new_emitter.texture_path.clone())
                    };

                    let pps = if new_emitter.pps > 0 {
                        Some(new_emitter.pps as usize)
                    } else {
                        None
                    };

                    let payload = EmitterBlackboard {
                        name: new_emitter.name.clone(),
                        angle_rand: new_emitter.angle_rand.into(),
                        radius_rand: Vec2::from_array(new_emitter.radius_rand),
                        gravity: new_emitter.gravity,
                        velocity: vec![
                            Vec2::from_array(new_emitter.velocity_x),
                            Vec2::from_array(new_emitter.velocity_y),
                            Vec2::from_array(new_emitter.velocity_z),
                        ],
                        particle_lifetime: new_emitter.particle_lifetime.into(),
                        particle_scale: new_emitter.particle_scale.into(),
                        particle_count: new_emitter.particle_count as usize,
                        colors: final_colors,
                        texture_path: texture_path,
                        texture_idx: None,
                        texture_has_alpha: new_emitter.texture_has_alpha,
                        radial_speed: new_emitter.radial_speed.into(),
                        up_speed: new_emitter.up_speed.into(),
                        jitter: new_emitter.jitter.into(),

                        base_alpha: new_emitter.base_alpha.into(),
                        alpha_multiplier: new_emitter.alpha_multiplier,
                        alpha_power: new_emitter.alpha_power,

                        base_scale: new_emitter.base_scale.into(),
                        scale_multiplier: new_emitter.scale_multiplier,
                        scale_power: new_emitter.scale_power,

                        direction: new_emitter.direction.into(),
                        pps,
                    };

                    if self.do_render {
                        let origin = self.new_pos.into();

                        if payload.pps.is_some() {
                            // CONTINUOUS EMITTER PREVIEW
                            if let Some(id) = new_emitter.id {
                                // we already have an instance of this one, edit it instead
                                particles.edit_staged_emitter(id, &payload, origin);
                            } else if self.timer >= 1.0 {
                                // First time spawning
                                let id = particles.spawn_oneshot_editor_emitter(&payload, origin);
                                new_emitter.id = Some(id);
                                self.did_render = true;
                            }
                        } else {
                            // ONESHOT PREVIEW
                            if self.timer >= 1.0 {
                                particles.spawn_oneshot_editor_emitter(&payload, origin);
                                self.did_render = true;
                            }
                        }
                    }

                    self.payloads.push(payload);

                    id_token.pop();
                }

                if ui.checkbox("Render Emitters", &mut self.do_render) {
                    message_queue.send(UiMessage::RenderStagedEmitters {
                        do_it: self.do_render,
                    })
                };

                if ui.button("Save ") {
                    for payload in self.payloads.iter() {
                        if !payload.name.is_empty() {
                            if !emitter_types.contains(&payload.name) {
                                particles
                                    .emitter_data
                                    .one_shot_data
                                    .insert(payload.name.clone(), payload.clone());
                                particles
                                    .emitter_data
                                    .save_to_file("config/particle_emitters.toml");
                            } else {
                                eprintln!("[Warning] emitter not saved, name was already taken.");
                            }
                        }
                    }
                };

                if self.did_render {
                    self.timer -= self.timer;
                    self.did_render = false;
                }
                self.timer += dt;
            });
    }
}

impl Default for ParticleEditor {
    fn default() -> Self {
        Self {
            new_emitters: vec![UiEmitterBlackboard::default()],

            em_idx: 0,
            new_pos: [0.0, 0.0, 0.0],
            current_color: [1.0, 1.0, 1.0, 1.0],
            color_intensity: 1.0,
            clr_idx: 0,
            do_render: false,
            did_render: false,
            timer: 0.0,
            staged_texture: String::new(),
            payloads: vec![],
        }
    }
}
