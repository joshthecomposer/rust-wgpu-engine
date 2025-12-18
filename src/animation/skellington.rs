use glam::{Mat4, Quat, Vec3};

#[derive(Debug, Clone)]
pub struct Bone {
    // id will be the position in the final bone array as well.
    pub id: u32,
    pub parent_index: Option<u32>,
    pub name: String,
    pub offset: Mat4,
    pub children: Vec<Bone>,
}

#[derive(Debug, Clone)]
pub struct BoneJoinInfo {
    pub name: String,
    // offset: Mat4,
}

#[derive(Debug, Clone)]
pub struct BoneTransformTrack {
    pub position_timestamps: Vec<f32>,
    pub rotation_timestamps: Vec<f32>,
    pub scale_timestamps: Vec<f32>,

    pub positions: Vec<Vec3>,
    pub rotations: Vec<Quat>,
    pub scales: Vec<Vec3>,
}

impl BoneTransformTrack {
    pub fn default() -> Self {
        Self {
            position_timestamps: vec![],
            rotation_timestamps: vec![],
            scale_timestamps: vec![],

            positions: vec![],
            rotations: vec![],
            scales: vec![],
        }
    }
}
