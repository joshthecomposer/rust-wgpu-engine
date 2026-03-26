use glam::{Quat, Vec3};
use winit::keyboard::KeyCode;

use crate::{
    enums_types::{AnimationType, CombatState, SoundType},
    input::InputState,
};

#[derive(Debug, Default)]
pub struct CommandBuffer {
    pub phys: Vec<PhysCmd>,
    pub sound: Vec<SoundCmd>,
    pub anim: Vec<AnimCmd>,
    pub loco: Vec<LocoCmd>,
    pub particles: Vec<PartCmd>,
    pub combat: Vec<CombCmd>,
}

impl CommandBuffer {
    pub fn impulse(&mut self, target: usize, source: Option<usize>, kind: ImpulseKind, v: Vec3) {
        self.phys.push(PhysCmd {
            target,
            source,
            kind,
            op: PhysOp::ApplyImpulse(v),
        });
    }

    pub fn next_anim(&mut self, target: usize, anim: AnimationType, weapon: Option<usize>) {
        self.anim.push(AnimCmd {
            target,
            weapon,
            op: AnimOp::SetNextAnimation(anim),
        });
    }

    pub fn next_anim_from_lookup(
        &mut self,
        target: usize,
        anim_lookup: String,
        weapon: Option<usize>,
    ) {
        self.anim.push(AnimCmd {
            target,
            weapon,
            op: AnimOp::SetAnimFromString(anim_lookup),
        });
    }

    pub fn current_anim(&mut self, target: usize, anim: AnimationType, weapon: Option<usize>) {
        self.anim.push(AnimCmd {
            target,
            weapon,
            op: AnimOp::SetCurrentAnimation(anim),
        });
    }
    pub fn set_anim_hold(
        &mut self,
        target: usize,
        anim: AnimationType,
        do_hold: bool,
        weapon: Option<usize>,
    ) {
        match do_hold {
            true => {
                self.anim.push(AnimCmd {
                    target,
                    weapon,
                    op: AnimOp::DoHold(anim),
                });
            }
            false => {
                self.anim.push(AnimCmd {
                    target,
                    weapon,
                    op: AnimOp::StopHold(anim),
                });
            }
        }
    }

    pub fn set_linvel(&mut self, target: usize, kind: ImpulseKind, v: Vec3) {
        self.phys.push(PhysCmd {
            target,
            source: None,
            kind,
            op: PhysOp::SetLinvel(v),
        });
    }

    pub fn set_rot(&mut self, target: usize, kind: ImpulseKind, q: Quat) {
        self.phys.push(PhysCmd {
            target,
            source: None,
            kind,
            op: PhysOp::SetRotation(q),
        });
    }

    pub fn jump(&mut self, target: usize) {
        self.phys.push(PhysCmd {
            target,
            source: None,
            kind: ImpulseKind::Locomotion,
            op: PhysOp::Jump,
        });
    }

    pub fn sound3d(&mut self, position: Vec3) {
        self.sound.push(SoundCmd {
            kind: SoundKind::Sound3d(SoundType::Land, position),
        });
    }

    pub fn reset_attacks(&mut self, target: usize, weapon: Option<usize>) {
        self.anim.push(AnimCmd {
            target,
            weapon,
            op: AnimOp::ResetAttacks,
        });
    }
}

// ==================================================================================
// PHYSICS
// ==================================================================================
#[derive(Clone, Debug)]
pub enum PhysOp {
    ApplyImpulse(Vec3),
    SetLinvel(Vec3),
    SetRotation(Quat),
    Jump,
}

#[derive(Clone, Debug)]
pub struct PhysCmd {
    pub target: usize,
    pub source: Option<usize>,
    pub kind: ImpulseKind,
    pub op: PhysOp,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ImpulseKind {
    Locomotion,  // run / jump steering
    Action,      // roll, dash, attack lunge
    HitReaction, // knockback, flinch, etc.
    World,       // explosion, wind
}

impl ImpulseKind {
    // very rudimentary "what takes precedence"
    pub fn cancels(self, other: Self) -> bool {
        matches!(
            (self, other),
            (ImpulseKind::Action, ImpulseKind::HitReaction) | (ImpulseKind::World, _)
        )
    }
}

// ==================================================================================
// ANIMATION
// ==================================================================================
#[derive(Clone, Debug)]
pub struct AnimCmd {
    pub target: usize,
    pub weapon: Option<usize>,
    pub op: AnimOp,
}

#[derive(Clone, Debug)]
pub enum AnimOp {
    SetNextAnimation(AnimationType),
    SetCurrentAnimation(AnimationType),
    DoHold(AnimationType),
    StopHold(AnimationType),
    SetAnimFromString(String),
    ResetAttacks,
}

// ==================================================================================
// LOCO
// ==================================================================================
#[derive(Clone, Debug)]
pub struct LocoCmd {
    pub target: usize,
    pub intent: LocoIntent,
}

#[derive(Clone, Debug, Copy)]
pub struct LocoIntent {
    pub x: f32,
    pub z: f32,
}

impl LocoIntent {
    pub fn is_zero(&self) -> bool {
        self.x == 0.0 && self.z == 0.0
    }

    fn clamp_unit(self) -> Self {
        let len2 = self.x * self.x + self.z * self.z;

        if len2 > 1.0 {
            let inv_len = 1.0 / len2.sqrt();
            Self {
                x: self.x * inv_len,
                z: self.z * inv_len,
            }
        } else {
            Self {
                x: self.x,
                z: self.z,
            }
        }
    }

    // TODO: later we can have between 0.0 and 1.0 if a controller is used etc.
    pub fn build_loco_intent(input: &InputState) -> Self {
        let down = |k| input.keys_current.contains(&k);

        let mut x = 0.0;
        let mut z = 0.0;

        if down(KeyCode::KeyD) {
            x += 1.0;
        }
        if down(KeyCode::KeyA) {
            x -= 1.0;
        }
        if down(KeyCode::KeyW) {
            z += 1.0;
        }
        if down(KeyCode::KeyS) {
            z -= 1.0;
        }

        LocoIntent { x, z }.clamp_unit()
    }
}

// ==================================================================================
// PARTICLES
// ==================================================================================
#[derive(Clone, Debug)]
pub struct PartCmd {
    pub name: String, // name of the emitter
    pub kind: PartKind,
    pub direction: Vec3,
}

#[derive(Clone, Debug)]
pub enum PartKind {
    // id of the weapon it originates from (it will come from the end of the bounding box)
    WeaponOrigin(usize),
    EntityOrigin(usize), // for instance the feet of the player
    WorldOrigin(Vec3),   // anywhere in the world
}

// ==================================================================================
// SOUNDS
// ==================================================================================
#[derive(Clone, Debug)]
pub struct SoundCmd {
    pub kind: SoundKind,
}

#[derive(Clone, Debug)]
pub enum SoundKind {
    Sound2d(SoundType),
    Sound3d(SoundType, Vec3),
}

// ==================================================================================
// COMBAT
// ==================================================================================
#[derive(Clone, Debug)]
pub struct CombCmd {
    pub entity_id: usize,
    pub requested_state: CombatState,
}
