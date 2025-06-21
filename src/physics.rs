use rapier3d::prelude::*;

pub struct PhysicsState {
    pub rigid_body_set: RigidBodySet,
    pub collider_set: ColliderSet,
}
