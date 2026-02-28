#![allow(dead_code)]
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

#[derive(Clone, Debug, PartialEq, Deserialize, Hash, Eq, Serialize)]
pub enum EntityType {
    Donut,
    TreeFoliage,
    TreeTrunk,
    MooseMan,
    YRobot,
    Terrain,
    Cylinder,
    Stump,
    OrcSword,
    DoubleAxe,
    TrashGuy,
    Cuboid,
    Cactus1,
    Cactus2,
    Rock1,
    Pill,
    BareBush1,
}

impl Display for EntityType {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        match self {
            EntityType::Donut => write!(f, "Donut"),
            EntityType::TreeFoliage => write!(f, "TreeFoliage"),
            EntityType::TreeTrunk => write!(f, "TreeTrunk"),
            EntityType::MooseMan => write!(f, "MooseMan"),
            EntityType::YRobot => write!(f, "YRobot"),
            EntityType::Terrain => write!(f, "Terrain"),
            EntityType::Cylinder => write!(f, "Cylinder"),
            EntityType::Stump => write!(f, "Stump"),
            EntityType::OrcSword => write!(f, "OrcSword"),
            EntityType::DoubleAxe => write!(f, "DoubleAxe"),
            EntityType::TrashGuy => write!(f, "TrashGuy"),
            EntityType::Cuboid => write!(f, "Cuboid"),
            EntityType::Cactus1 => write!(f, "Cactus1"),
            EntityType::Cactus2 => write!(f, "Cactus2"),
            EntityType::Rock1 => write!(f, "Rock1"),
            EntityType::Pill => write!(f, "Pill"),
            EntityType::BareBush1 => write!(f, "BareBush1"),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Deserialize, Serialize)]
pub enum Faction {
    Enemy,
    Static,
    World,
    Player,
    Gizmo,
    Item,
}

impl Display for Faction {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        match self {
            Faction::Enemy => write!(f, "Enemy"),
            Faction::Static => write!(f, "Static"),
            Faction::World => write!(f, "World"),
            Faction::Player => write!(f, "Player"),
            Faction::Gizmo => write!(f, "Gizmo"),
            Faction::Item => write!(f, "Item"),
        }
    }
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

#[derive(Clone, Debug, PartialEq)]
pub enum SimState {
    Init,
    Waiting,
    Aggro,
    Dancing,
    Dying,
    Dead,
    Combat,
    Flinching,
    Blocking,
}

pub struct SimStateController {
    pub state: SimState,
    pub attack_state: AttackState,
    pub time_in_state: f32,
    pub target_time: f32,
}

impl Default for SimStateController {
    fn default() -> Self {
        Self {
            state: SimState::Init,
            attack_state: AttackState::Attack1,
            time_in_state: 0.0,
            target_time: 0.0,
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub enum PlayerState {
    Init,
    Idle,
    Running,
    Dying,
    Dead,
    Utility,
    Basic,
    Defensive,
    Skill1,
    Skill2,
    Ultimate,
    Airborne,
}

impl Display for PlayerState {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self {
            PlayerState::Init => write!(f, "Init"),
            PlayerState::Idle => write!(f, "Idle"),
            PlayerState::Running => write!(f, "Running"),
            PlayerState::Dying => write!(f, "Dying"),
            PlayerState::Dead => write!(f, "Dead"),
            PlayerState::Utility => write!(f, "Utility"),
            PlayerState::Basic => write!(f, "Basic"),
            PlayerState::Defensive => write!(f, "Defensive"),
            PlayerState::Skill1 => write!(f, "Skill1"),
            PlayerState::Skill2 => write!(f, "Skill2"),
            PlayerState::Ultimate => write!(f, "Ultimate"),
            PlayerState::Airborne => write!(f, "Airborne"),
        }
    }
}

#[derive(Debug, Copy, Clone, PartialEq)]
pub enum LocoState {
    Init,
    Idle,
    Running,
    Airborne,
    Jumping,
}

#[derive(Debug)]
pub enum LifeState {
    Alive,
    Dying,
    Dead,
}

#[derive(Debug, PartialEq)]
pub enum ControlState {
    Player,
    Combat,
    Locked,
}

#[derive(Debug, PartialEq, Clone, Copy)]
pub enum CombatState {
    Basic1,
    Basic2,
    Basic3,
    Defensive,
    Skill1,
    Skill2,
    Evade,
    Ultimate,
}

#[derive(Debug)]
pub struct PlayerController {
    pub loco_state: LocoState,
    pub loco_time: f32,

    pub combat_state: Option<CombatState>,
    pub combat_time: f32,

    pub buffered_action: Option<u32>,
    pub buffer_timer: f32,
    pub life_state: LifeState,
    pub control_state: ControlState,

    pub jump_command_issued: bool,
    pub particle_cmd_issued: bool,
}

impl PlayerController {
    pub fn can_loco(&self) -> bool {
        self.control_state != ControlState::Locked && self.control_state != ControlState::Combat
    }
}

// TODO: deprecate this
#[derive(Clone, Debug, PartialEq)]
pub enum AttackState {
    Attack1,
    Attack2,
    Attack3,
}

impl Display for AttackState {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self {
            AttackState::Attack1 => write!(f, "Attack1"),
            AttackState::Attack2 => write!(f, "Attack2"),
            AttackState::Attack3 => write!(f, "Attack3"),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Hash, Eq, Deserialize, Serialize)]
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
        }
    }
}

impl SoundType {
    pub fn from_str(input: &str) -> Option<Self> {
        match input {
            "Footstep" => Some(SoundType::Footstep),
            "MooseHuff" => Some(SoundType::MooseHuff),
            "Music" => Some(SoundType::Music),
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
