#![allow(dead_code)]
use std::{cell::Cell, fmt::{self, Display, Formatter}, ops::RangeInclusive, str::FromStr};

use glam::{Mat4, Quat, Vec3};
use rapier3d::prelude::{ColliderHandle, RigidBodyHandle};
use serde::{Deserialize, Serialize};

#[derive(Debug, Eq, PartialEq, Hash)]
pub enum VaoType {
    Cube,
    Skybox,
    DebugLight,
    GroundPlane
}

#[derive(Debug, Eq, PartialEq, Hash)]
pub enum VboType {
    Cube,
    Skybox,
    DebugLight,
    GroundPlane,
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

    pub original_rotation: Quat,
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
            Faction::Player=> write!(f, "Player"),
            Faction::Gizmo => write!(f, "Gizmo"),
            Faction::Item => write!(f, "Item"),
        }
    }
}

#[derive(Clone, Debug)]
pub enum CellType {
    Grass,
    Tree,
    Path
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

#[derive(Clone, Debug, PartialEq)]
pub enum CameraState {
    Free,
    Third,
    Locked,
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
    Aggro,
    Waiting,
    Dancing,
    Dying ,
    Dead { time: f32, target_time: f32 },
    Attacking,
    Blocking,
    Flinching,
    Dashing,
}

pub struct SimStateController {
    pub state: SimState,
    pub attack_state: AttackState,
    pub time_in_state: f32,
}

#[derive(Clone, Debug, PartialEq)]
pub enum PlayerState {
    Idle,
    Running, 
    Jumping,
    Freefalling,
    Attacking,
    Dying,
    Dead { time: f32, target_time: f32 },
    Dashing,
    Blocking,
}

pub struct PlayerController {
    pub state: PlayerState,
    pub attack_state: AttackState,
    pub time_in_state: f32,
}

#[derive(Clone, Debug, PartialEq)]
pub enum AttackState {
    Attack1,
    Attack2,
    Attack3,
}

#[derive(Clone, Debug, PartialEq, Hash, Eq, Deserialize)]
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
}

#[derive(Clone, Debug, PartialEq, Hash, Eq, Deserialize)]
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
    pub fn from_str(input: &str) -> Option<Self>  {
        match input {
            "BroadDefault" => Some(Self::BroadDefault),
            "DecalCrisp"   => Some(Self::DecalCrisp),
            "AlphaMasked"  => Some(Self::AlphaMasked),
            _              => panic!("Invalid TextureProfile passed in."),
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

pub struct PhysicsBody {

}
