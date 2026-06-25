use glam::{vec3, Quat, Vec3, Vec4};
use rand::{rng, Rng};

use crate::{
    command_buffer::{CommandBuffer, PartKind},
    config::{
        emitter_data::{EmitterBlackboard, EmitterData},
        Config,
    },
    entity_manager::EntityManager,
};

#[derive(Debug)]
pub struct Emitter {
    pub positions: Vec<Vec3>,
    pub times_alive: Vec<f32>,
    pub lifetimes: Vec<f32>,
    pub velocities: Vec<Vec3>,
    pub rotation_speeds: Vec<f32>,
    pub rotation_offsets: Vec<f32>,

    pub count: usize,
    pub alive: bool,
    pub name: String,

    pub pps: Option<usize>,
    pub emit_accumulator: f32,
    pub origin: Vec3,
    pub texture_path: Option<String>,
    pub texture_has_alpha: bool,
    pub colors: Vec<Vec4>,
    pub gravity: f32,

    pub alphas: Vec<f32>,
    pub base_alphas: Vec<f32>,
    pub end_alphas: Vec<f32>,
    pub alpha_powers: Vec<f32>,

    pub scales: Vec<f32>,
    pub base_scales: Vec<f32>,
    pub end_scales: Vec<f32>,
    pub scale_powers: Vec<f32>,

    pub editor_blackboard: Option<EmitterBlackboard>,
    pub has_bloom: bool,
}

impl Emitter {
    pub fn new() -> Self {
        Self {
            positions: vec![],
            times_alive: vec![],
            lifetimes: vec![],
            velocities: vec![],

            rotation_speeds: vec![],
            rotation_offsets: vec![],
            count: 0,
            alive: true,
            name: String::new(),

            pps: None,
            emit_accumulator: 0.0,
            origin: Vec3::splat(1.0),
            texture_path: None,
            texture_has_alpha: true,
            colors: vec![],
            gravity: 0.0,

            alpha_powers: vec![],
            alphas: vec![],
            base_alphas: vec![],
            end_alphas: vec![],

            scale_powers: vec![],
            scales: vec![],
            base_scales: vec![],
            end_scales: vec![],

            editor_blackboard: None,
            has_bloom: false,
        }
    }
}

// an emitter that is just being previewed in the editor
#[derive(Debug)]
pub struct StagedEmitter {
    pub id: usize,
    pub emitter: Emitter,
}

pub struct ParticleSystem {
    pub emitters: Vec<Emitter>,
    pub emitter_data: EmitterData,

    pub next_staged_id: usize,
    pub staged_emitters: Vec<StagedEmitter>,
    pub render_staged_emitters: bool,
}

impl ParticleSystem {
    pub fn new(ed_file: &str) -> Self {
        let emitter_data = EmitterData::load_from_file(ed_file);

        Self {
            emitters: vec![],
            emitter_data,

            next_staged_id: 0,
            staged_emitters: vec![],
            render_staged_emitters: false,
        }
    }

    pub fn iter_drawable(&self) -> impl Iterator<Item = &Emitter> {
        let render_staged = self.render_staged_emitters;
        let staged_iter = self
            .staged_emitters
            .iter()
            .filter(move |_| render_staged)
            .map(|s| &s.emitter);
        self.emitters.iter().chain(staged_iter)
    }

    pub fn spawn_oneshot_editor_emitter(&mut self, ed: &EmitterBlackboard, origin: Vec3) -> usize {
        let mut emitter = Emitter::new();

        emitter.texture_path = ed.texture_path.clone();

        emitter.origin = origin;
        emitter.gravity = ed.gravity;
        emitter.has_bloom = ed.has_bloom;

        let desired_dir = if ed.direction.length_squared() > 0.0 {
            ed.direction.normalize()
        } else {
            Vec3::Y
        };

        emitter.pps = ed.pps;

        Self::calculate_particle_data(ed, origin, Some(desired_dir), &mut emitter);

        emitter.editor_blackboard = Some(ed.clone());

        emitter.name = ed.name.clone();
        emitter.texture_has_alpha = ed.texture_has_alpha;
        let current_id = self.next_staged_id;
        let staged_emitter = StagedEmitter {
            id: current_id,
            emitter,
        };
        self.staged_emitters.push(staged_emitter);
        self.next_staged_id += 1;

        current_id
    }

    pub fn spawn_oneshot_emitter(
        &mut self,
        emitter_name: &str,
        origin: Vec3,
        direction: Option<Vec3>,
    ) {
        let mut emitter = Emitter::new();

        let ed = match self.emitter_data.one_shot_data.get(emitter_name) {
            Some(ed) => ed,
            None => panic!("Could not find emitter type of {}", emitter_name),
        };

        emitter.texture_path = ed.texture_path.clone();

        emitter.origin = origin;
        emitter.gravity = ed.gravity;
        emitter.pps = ed.pps;
        emitter.has_bloom = ed.has_bloom;

        Self::calculate_particle_data(ed, origin, direction, &mut emitter);

        emitter.name = ed.name.clone();
        emitter.texture_has_alpha = ed.texture_has_alpha;
        self.emitters.push(emitter);
    }

    pub fn update(&mut self, dt: f32, cmds: &mut CommandBuffer, em: &EntityManager) {
        // ==========================
        // evaluate commands
        // ==========================
        let partcmds = std::mem::take(&mut cmds.particles);

        for c in partcmds {
            match c.kind {
                PartKind::WeaponOrigin(id) => {
                    let Some(origin) = em.world_weapon_tips.get(id) else {
                        eprintln!("Failed to find the tip for the given weapon");
                        continue;
                    };

                    let Some(owner) = em.owners.get(id) else {
                        eprintln!("Weapon has no owner, this seems wrong...");
                        continue;
                    };

                    let yaw = em.yaws.get(*owner).unwrap();

                    let direction = vec3(yaw.sin(), 0.0, yaw.cos());

                    self.spawn_oneshot_emitter(&c.name, *origin, Some(direction));
                }
                PartKind::EntityOrigin(id) => {
                    let Some(trans) = em.transforms.get(id) else {
                        eprintln!("Failed to find the transform for the entity");
                        continue;
                    };

                    self.spawn_oneshot_emitter(&c.name, trans.position, Some(c.direction));
                }
                PartKind::WorldOrigin(pos) => {
                    self.spawn_oneshot_emitter(&c.name, pos, Some(c.direction));
                }
                _ => eprintln!("Not implemented yet in the command system"),
            }
        }

        // ==========================
        // update emitters
        // ==========================
        let ed = &self.emitter_data;

        for e in self.emitters.iter_mut() {
            Self::update_emitter(e, dt, ed);
        }
        self.emitters.retain(|e| e.alive);

        for se in self.staged_emitters.iter_mut() {
            Self::update_emitter(&mut se.emitter, dt, ed);
        }
        self.staged_emitters.retain(|se| se.emitter.alive);
    }

    fn update_emitter(emitter: &mut Emitter, dt: f32, emitter_data: &EmitterData) {
        let gravity = Vec3::new(0.0, emitter.gravity, 0.0);

        let def: EmitterBlackboard = if let Some(bb) = &emitter.editor_blackboard {
            bb.clone()
        } else {
            emitter_data
                .one_shot_data
                .get(&emitter.name)
                .expect("missing emitter preset")
                .clone()
        };

        if let Some(pps) = emitter.pps {
            if pps > 0 {
                emitter.emit_accumulator += dt;
                let seconds_per_particle = 1.0 / pps as f32;

                while emitter.emit_accumulator >= seconds_per_particle {
                    emitter.emit_accumulator -= seconds_per_particle;
                    ParticleSystem::spawn_particle(emitter, &def);
                }
            }
        }

        // lifetime / gravity stuff
        let mut i = 0;
        while i < emitter.count {
            if emitter.times_alive[i] >= emitter.lifetimes[i] {
                let last = emitter.count - 1;

                emitter.positions.swap(i, last);
                emitter.times_alive.swap(i, last);
                emitter.lifetimes.swap(i, last);
                emitter.velocities.swap(i, last);
                emitter.rotation_speeds.swap(i, last);
                emitter.rotation_offsets.swap(i, last);

                emitter.alphas.swap(i, last);
                emitter.base_alphas.swap(i, last);
                emitter.end_alphas.swap(i, last);
                emitter.alpha_powers.swap(i, last);

                emitter.scales.swap(i, last);
                emitter.base_scales.swap(i, last);
                emitter.end_scales.swap(i, last);
                emitter.scale_powers.swap(i, last);

                emitter.colors.swap(i, last);

                emitter.count -= 1;
            } else {
                emitter.times_alive[i] += dt;
                emitter.velocities[i] += gravity * dt;
                emitter.positions[i] += emitter.velocities[i] * dt;
                i += 1;
            }
        }

        if emitter.count == 0 {
            if let Some(pps) = emitter.pps {
                if pps == 0 {
                    emitter.alive = false;
                }
            } else {
                emitter.alive = false;
            }
        }
    }

    pub fn edit_staged_emitter(&mut self, id: usize, ed: &EmitterBlackboard, origin: Vec3) {
        if let Some(se) = self.staged_emitters.iter_mut().find(|se| se.id == id) {
            let emitter = &mut se.emitter;

            emitter.origin = origin;
            emitter.gravity = ed.gravity;
            emitter.name = ed.name.clone();
            emitter.texture_has_alpha = ed.texture_has_alpha;
            emitter.pps = ed.pps;
            emitter.editor_blackboard = Some(ed.clone());
            emitter.texture_path = ed.texture_path.clone();
        }
    }

    pub fn spawn_particle(emitter: &mut Emitter, ed: &EmitterBlackboard) {
        let origin = emitter.origin;

        let desired_dir = if ed.direction.length_squared() > 0.0 {
            Some(ed.direction.normalize())
        } else {
            Some(Vec3::Y)
        };

        Self::calculate_particle_data(ed, origin, desired_dir, emitter);
    }

    pub fn calculate_particle_data(
        ed: &EmitterBlackboard,
        _origin: Vec3,
        direction: Option<Vec3>,
        emitter: &mut Emitter,
    ) {
        let local_up = Vec3::Y;

        let desired_dir = if let Some(d) = direction {
            d.normalize()
        } else {
            Vec3::Y
        };

        let mut rng = rng();

        // How many particles to spawn this call?
        // - Oneshot emitters (pps == None): spawn all at once
        // - Continuous emitters (pps == Some): spawn exactly 1 per call
        let num_to_spawn = if emitter.pps.is_none() {
            ed.particle_count
        } else {
            1
        };

        for _ in 0..num_to_spawn {
            let rot = Quat::from_rotation_arc(local_up, desired_dir);

            let angle = if ed.angle_rand.x >= ed.angle_rand.y {
                ed.angle_rand.x
            } else {
                rng.random_range(ed.angle_rand.x..=ed.angle_rand.y)
            };

            let radius = if ed.radius_rand.x >= ed.radius_rand.y {
                ed.radius_rand.x
            } else {
                rng.random_range(ed.radius_rand.x..=ed.radius_rand.y)
            };

            let local_offset = vec3(radius * angle.cos(), 0.0, radius * angle.sin());

            let world_offset = rot * local_offset;
            let position = emitter.origin + world_offset;

            let local_dir = local_offset.normalize_or_zero();

            let radial_speed = if ed.radial_speed.x >= ed.radial_speed.y {
                ed.radial_speed.x
            } else {
                rng.random_range(ed.radial_speed.x..=ed.radial_speed.y)
            };

            let up_speed = if ed.up_speed.x >= ed.up_speed.y {
                ed.up_speed.x
            } else {
                rng.random_range(ed.up_speed.x..=ed.up_speed.y)
            };

            let jitter_amount = if ed.jitter.x >= ed.jitter.y {
                ed.jitter.x
            } else {
                rng.random_range(ed.jitter.x..=ed.jitter.y)
            };

            let jitter_dir = {
                let a = rng.random_range(0.0..std::f32::consts::TAU);
                vec3(a.cos(), 0.0, a.sin())
            };
            let jitter_local = jitter_dir * jitter_amount;

            let local_velocity = local_dir * radial_speed + Vec3::Y * up_speed + jitter_local;

            let velocity = rot * local_velocity;

            let lifetime = if ed.particle_lifetime.x >= ed.particle_lifetime.y {
                ed.particle_lifetime.x
            } else {
                rng.random_range(ed.particle_lifetime.x..=ed.particle_lifetime.y)
            };

            let scale = if ed.base_scale.x >= ed.base_scale.y {
                ed.base_scale.x
            } else {
                rng.random_range(ed.base_scale.x..=ed.base_scale.y)
            };

            let alpha = if ed.base_alpha.x >= ed.base_alpha.y {
                ed.base_alpha.x
            } else {
                rng.random_range(ed.base_alpha.x..=ed.base_alpha.y)
            };

            let color = if ed.colors.len() > 1 {
                ed.colors[rng.random_range(0..ed.colors.len())]
            } else {
                ed.colors[0]
            };

            // TODO: Instead allocate the right size at the beginning by multiplying the particles per second by the lifetime
            if emitter.count < emitter.positions.len() {
                let i = emitter.count;
                emitter.positions[i] = position;
                emitter.velocities[i] = velocity;
                emitter.colors[i] = color;
                emitter.lifetimes[i] = lifetime;
                emitter.times_alive[i] = 0.0;
                emitter.rotation_speeds[i] = 0.0;
                emitter.rotation_offsets[i] = 0.0;

                emitter.alphas[i] = alpha;
                emitter.base_alphas[i] = alpha;
                emitter.end_alphas[i] = alpha * ed.alpha_multiplier;
                emitter.alpha_powers[i] = ed.alpha_power;

                emitter.scales[i] = scale;
                emitter.base_scales[i] = scale;
                emitter.end_scales[i] = scale * ed.scale_multiplier;
                emitter.scale_powers[i] = ed.scale_power;
            } else {
                emitter.positions.push(position);
                emitter.velocities.push(velocity);
                emitter.colors.push(color);
                emitter.lifetimes.push(lifetime);
                emitter.times_alive.push(0.0);
                emitter.rotation_speeds.push(0.0);
                emitter.rotation_offsets.push(0.0);

                emitter.alphas.push(alpha);
                emitter.base_alphas.push(alpha);
                emitter.end_alphas.push(alpha * ed.alpha_multiplier);
                emitter.alpha_powers.push(ed.alpha_power);

                emitter.scales.push(scale);
                emitter.base_scales.push(scale);
                emitter.end_scales.push(scale * ed.scale_multiplier);
                emitter.scale_powers.push(ed.scale_power);
            }

            emitter.count += 1;
        }
    }
}
