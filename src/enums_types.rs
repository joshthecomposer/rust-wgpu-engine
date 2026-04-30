use std::{
    cell::Cell,
    fmt::{self, Display, Formatter},
    ops::RangeInclusive,
    str::FromStr,
};

use glam::{Quat, Vec3};
use rapier3d::{
    math::Vector,
    prelude::{ColliderHandle, RigidBodyHandle, RigidBodyType},
};
use serde::{Deserialize, Serialize};

use crate::state_machines::enemy::enemy_behavior_tree::ActionKind;

#[derive(Debug, Eq, PartialEq, Hash)]
pub enum VaoType {
    Cube,
    Skybox,
    DebugLight,
    GroundPlane,
    BaseQuad,
}

#[derive(Debug, Eq, PartialEq, Hash)]
pub enum VboType {
    Cube,
    Skybox,
    DebugLight,
    GroundPlane,
    BaseQuad,
}

#[derive(Debug, Eq, PartialEq, Hash)]
pub enum EboType {
    Cube,
    Skybox,
    DebugLight,
}

#[derive(Debug, Eq, PartialEq, Hash)]
pub enum FboType {
    DepthMap,
    HDR,
    HdrMsaa,
}

#[derive(Debug, Eq, PartialEq, Hash)]
pub enum ShaderType {
    Skybox,
    DebugLight,
    Depth,
    GroundPlane,
    DebugShadowMap,
    Model,
    Text,
    Gizmo,
    Particles,
    GameUi,
    UiOverlay,
    HDR,
    Blur,
    StaticModel,
    AnimatedModel,
    Fxaa,
    BloomUpsample,
    BloomDownsample,
}

#[derive(Clone, Debug, PartialEq, Deserialize, Serialize)]
pub enum FxaaLevels {
    Off,
    Low,
    Med,
    High,
}

/// A struct to carry some rotation state for blending between rotations smoothly
/// different than the Transform which just holds the current true simulation state
/// which might be blended between cur_rot and next_rot
#[derive(Debug)]
pub struct Rotator {
    pub cur_rot: Quat,
    pub next_rot: Quat,
    pub blend_factor: f32,
    pub blend_time: f32,
}

#[derive(Debug, Clone)]
pub struct Transform {
    pub position: Vec3,
    pub rotation: Quat,
    pub scale: Vec3,
}

#[derive(Clone, Debug)]
pub enum CellType {
    Grass,
    Tree,
    Path,
}

#[derive(Clone, Debug, PartialEq)]
pub enum TextureType {
    Diffuse,
    Specular,
    Emissive,
    NormalMap,
    Roughness,
    Metalness,
    Displacement,
    Opacity,
}

impl Display for TextureType {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        match self {
            TextureType::Diffuse => write!(f, "Diffuse"),
            TextureType::Specular => write!(f, "Specular"),
            TextureType::Emissive => write!(f, "Emissive"),
            TextureType::NormalMap => write!(f, "Normal Map"),
            TextureType::Roughness => write!(f, "Roughness"),
            TextureType::Metalness => write!(f, "Metalness"),
            TextureType::Displacement => write!(f, "Displacement"),
            TextureType::Opacity => write!(f, "Opacity"),
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum CameraState {
    Free,
    Third,
    Locked,
    Gallery,
}

#[derive(Clone, Debug)]
pub struct Size3 {
    pub w: f32,
    pub h: f32,
    pub d: f32,
}

pub struct Parent {
    pub parent_id: usize,
}

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum LocoState {
    Init,
    Idle,
    Running,
    Airborne,
    Jumping,
}

#[derive(Debug, PartialEq)]
pub enum LifeState {
    Alive,
    Dying,
    Dead,
}

#[derive(Debug, PartialEq, Clone, Copy)]
pub enum ControlState {
    Player,
    Combat,
    Locked,
}

#[derive(Debug, PartialEq, Clone, Copy, Eq)]
pub enum CombatState {
    Basic,
    Defensive,
    Skill1,
    Skill2,
    Evade,
    Ultimate,
}

#[derive(Debug, PartialEq, Clone, Copy)]
pub struct BufferedAction {
    pub action: u32,
    pub ttl: f32,
}

#[derive(Debug)]
pub struct PlayerController {
    pub loco_state: LocoState,
    pub loco_time: f32,

    pub combat_state: Option<CombatState>,
    pub combat_time: f32,

    pub life_state: LifeState,
    pub control_state: ControlState,

    pub jump_command_issued: bool,
    pub particle_cmd_issued: bool,
    pub impulse_cmd_issued: bool,

    pub queued_action: Option<BufferedAction>,
}

impl PlayerController {
    pub fn can_loco(&self) -> bool {
        self.control_state != ControlState::Locked && self.control_state != ControlState::Combat
    }
}

impl Default for PlayerController {
    fn default() -> Self {
        Self {
            loco_state: LocoState::Init,
            loco_time: 0.0,
            combat_state: None,
            combat_time: 0.0,
            life_state: LifeState::Alive,
            control_state: ControlState::Player,
            jump_command_issued: false,
            particle_cmd_issued: false,
            impulse_cmd_issued: false,
            queued_action: None,
        }
    }
}

#[derive(Debug)]
pub struct EnemyController {
    pub took_damage: bool,
    pub taken_damage_ago: f32,
    pub taken_damage_ttl: f32,
    pub desired_action: Option<ActionKind>,
    pub current_action: ActionKind,
    pub life_state: LifeState,
    pub dying_counter: f32,
}

impl Default for EnemyController {
    fn default() -> Self {
        Self {
            took_damage: false,
            taken_damage_ago: 0.0,
            taken_damage_ttl: 5.0,
            desired_action: None,
            current_action: ActionKind::Idle,
            life_state: LifeState::Alive,
            dying_counter: 0.0,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Hash, Eq, Deserialize, Serialize, Copy)]
pub enum AnimationType {
    Run,
    Idle,
    Death,
    Dance,
    Slash,
    Slash2,
    Jump,
    Freefall,
    DashF,
    Block,
    Flinch,
    Basic1,
    Basic2,
    Basic3,
    OSBasic1,
    OSBasic2,
    OSBasic3,
    Roll,
    Stagger,
}

impl Display for AnimationType {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        match self {
            AnimationType::Run => write!(f, "Run"),
            AnimationType::Idle => write!(f, "Idle"),
            AnimationType::Death => write!(f, "Death"),
            AnimationType::Dance => write!(f, "Dance"),
            AnimationType::Slash => write!(f, "Slash"),
            AnimationType::Slash2 => write!(f, "Slash2"),
            AnimationType::Jump => write!(f, "Jump"),
            AnimationType::Freefall => write!(f, "Freefall"),
            AnimationType::DashF => write!(f, "DashF"),
            AnimationType::Block => write!(f, "Block"),
            AnimationType::Flinch => write!(f, "Flinch"),
            AnimationType::Basic1 => write!(f, "Basic1"),
            AnimationType::Basic2 => write!(f, "Basic2"),
            AnimationType::Basic3 => write!(f, "Basic3"),
            AnimationType::OSBasic1 => write!(f, "OSBasic1"),
            AnimationType::OSBasic2 => write!(f, "OSBasic2"),
            AnimationType::OSBasic3 => write!(f, "OSBasic3"),
            AnimationType::Roll => write!(f, "Roll"),
            AnimationType::Stagger => write!(f, "Stagger"),
        }
    }
}

impl AnimationType {
    pub fn from_str(input: &str) -> Option<Self> {
        match input {
            "Run" => Some(AnimationType::Run),
            "Idle" => Some(AnimationType::Idle),
            "Death" => Some(AnimationType::Death),
            "Dance" => Some(AnimationType::Dance),
            "Slash" => Some(AnimationType::Slash),
            "Jump" => Some(AnimationType::Jump),
            "Freefall" => Some(AnimationType::Freefall),
            "Slash2" => Some(AnimationType::Slash2),
            "DashF" => Some(AnimationType::DashF),
            "Block" => Some(AnimationType::Block),
            "Flinch" => Some(AnimationType::Flinch),
            "Basic1" => Some(AnimationType::Basic1),
            "Basic2" => Some(AnimationType::Basic2),
            "Basic3" => Some(AnimationType::Basic3),
            "OSBasic1" => Some(AnimationType::OSBasic1),
            "OSBasic2" => Some(AnimationType::OSBasic2),
            "OSBasic3" => Some(AnimationType::OSBasic3),
            "Roll" => Some(AnimationType::Roll),
            "Stagger" => Some(AnimationType::Stagger),
            _ => panic!("Invalid AnimationType passed in. {}", input),
        }
    }
}

pub struct VisualEffect {
    pub effect: Effect,
    pub ttl: f32, // Time in seconds
}

#[derive(Clone, Debug)]
pub enum Effect {
    Flashing,
}

pub struct ActiveItem {
    // TODO: what if more hands
    pub right_hand: Option<usize>,
    pub left_hand: Option<usize>,
}

pub struct Inventory {
    pub items: Vec<usize>,
}

pub struct PhysicsHandle {
    pub rigid_body: RigidBodyHandle,
    pub collider: ColliderHandle,

    // for moving things around or temporarily changing them for impulses etc.
    // we can set them to kinematic and then back to this when needed
    pub og_rb_type: RigidBodyType,
}

#[derive(Clone, Debug, PartialEq, Hash, Eq, Deserialize, Serialize)]
pub enum SoundType {
    Footstep,
    MooseHuff,
    Music,
    Jump,
    Land,
    StopRunning,
    Bloop,
}

impl Display for SoundType {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        match self {
            SoundType::Footstep => write!(f, "Footstep"),
            SoundType::MooseHuff => write!(f, "MooseHuff"),
            SoundType::Music => write!(f, "Music"),
            SoundType::Jump => write!(f, "Jump"),
            SoundType::Land => write!(f, "Land"),
            SoundType::StopRunning => write!(f, "StopRunning"),
            SoundType::Bloop => write!(f, "Bloop"),
        }
    }
}

impl SoundType {
    pub fn from_str(input: &str) -> Option<Self> {
        match input {
            "Footstep" => Some(SoundType::Footstep),
            "MooseHuff" => Some(SoundType::MooseHuff),
            "Music" => Some(SoundType::Music),
            "Bloop" => Some(SoundType::Bloop),
            _ => panic!("Invalid SoundType passed in."),
        }
    }
}

pub const ANIMATION_EPSILON: f32 = 0.001;

#[derive(Clone, Debug, PartialEq, Hash, Eq, Deserialize)]
pub enum TextureProfile {
    BroadDefault, // For big textures like landscape/tree trunks etc.
    DecalCrisp,   // For small details like a face etc.
    AlphaMasked,  // For foliage
}

impl TextureProfile {
    pub fn from_str(input: &str) -> Option<Self> {
        match input {
            "BroadDefault" => Some(Self::BroadDefault),
            "DecalCrisp" => Some(Self::DecalCrisp),
            "AlphaMasked" => Some(Self::AlphaMasked),
            _ => panic!("Invalid TextureProfile passed in."),
        }
    }
}

impl Display for TextureProfile {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        match self {
            Self::BroadDefault => write!(f, "BroadDefault"),
            Self::AlphaMasked => write!(f, "AlphaMasked"),
            Self::DecalCrisp => write!(f, "DecalCrisp"),
        }
    }
}

#[derive(Clone, Debug)]
pub struct FrameActivation {
    pub segment_range: RangeInclusive<u32>,
    pub triggered: Cell<bool>,
}

#[derive(Clone, Debug)]
pub struct Knockback {
    pub ttl: f32, // time remaining seconds
    pub flinch: bool,
    pub did_particles: bool,
    //pub lock_rotation: bool,
}

#[derive(Deserialize, Debug, Serialize, Hash, PartialEq, Eq)]
pub enum EmitterName {
    DesertSlide,
    DesertStep,
    DamageBlood,
    BodyPoof,
    DesertLand,
}

impl Display for EmitterName {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        match self {
            EmitterName::DesertSlide => write!(f, "DesertSlide"),
            EmitterName::DesertStep => write!(f, "DesertStep"),
            EmitterName::DamageBlood => write!(f, "DamageBlood"),
            EmitterName::BodyPoof => write!(f, "BodyPoof"),
            EmitterName::DesertLand => write!(f, "DesertLand"),
        }
    }
}

pub enum EmitterType {
    OneShot,
    Continuous,
}

#[derive(Clone, Debug, PartialEq, Deserialize, Copy, Serialize)]
#[serde(tag = "shape", rename_all = "PascalCase")]
pub enum HitboxShape {
    Cylinder { r: f32, h: f32 },
    Pill { r: f32, h: f32 },
    BoxDim { hx: f32, hy: f32, hz: f32 },
    Sphere { r: f32 },
    Mesh,        // The mesh itself is the collider
    BoundingBox, // Dynamically generated box around the mesh
}

pub struct ParseHitboxShapeError;

impl FromStr for HitboxShape {
    type Err = ParseHitboxShapeError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "Cylinder" | "cylinder" => Ok(HitboxShape::Cylinder { r: 0.5, h: 1.0 }),
            "Pill" | "pill" => Ok(HitboxShape::Pill { r: 0.5, h: 1.0 }),
            "BoxDim" | "boxdim" => Ok(HitboxShape::BoxDim {
                hx: 0.5,
                hy: 0.5,
                hz: 0.5,
            }),
            "Sphere" | "sphere" => Ok(HitboxShape::Sphere { r: 0.5 }),
            "Mesh" | "mesh" => Ok(HitboxShape::Mesh),
            "BoundingBox" | "boundingbox" => Ok(HitboxShape::BoundingBox),
            _ => Err(ParseHitboxShapeError),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Deserialize, Hash, Eq, Serialize)]
pub enum EquipSlot {
    RHand,
    LHand,
}

#[derive(Clone, Debug, Copy)]
pub struct JumpHeight {
    pub desired: f32,
    // The precalculated impulse. This is scary because if we add or
    // take away a collider we will have to recalc this
    pub precalculated: Option<Vector<rapier3d::math::Real>>,
}

#[derive(Clone, Debug)]
pub struct GroundedState {
    pub was_grounded: bool,
    pub is_grounded: bool,
    pub just_left: bool,
    pub just_landed: bool,
    pub ray_length_grounded: f32,
    pub ray_length_airborne: f32,
}
