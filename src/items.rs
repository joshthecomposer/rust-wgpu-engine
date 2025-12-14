use crate::{
    entity_manager::EntityManager, physics::PhysicsState, util::data_structure::HashMapGetPairMut,
};
use glam::{Mat4, Vec3};
use rapier3d::prelude::RigidBodyType;

pub fn update(em: &mut EntityManager, ps: &mut PhysicsState) {
    orphaned_items_pass(em, ps);
    unequipped_items_pass(em, ps);
}

fn unequipped_items_pass(em: &mut EntityManager, ps: &mut PhysicsState) {
    let unequipped = em.get_all_unequipped_owned_ids();

    for id in unequipped.iter() {
        if let Some(ph) = em.physics_handles.get(*id) {
            let wrb = ps.rigid_body_set.get_mut(ph.rigid_body).unwrap();
            let col = ps.collider_set.get_mut(ph.collider).unwrap();

            if wrb.body_type() != RigidBodyType::KinematicPositionBased {
                wrb.set_body_type(RigidBodyType::KinematicPositionBased, false);
                wrb.enable_ccd(false);
                wrb.wake_up(true);

                col.set_sensor(false);
                col.set_enabled(false);

                wrb.set_next_kinematic_position(Vec3::splat(0.0).into());
            }
        }
    }
}

fn orphaned_items_pass(em: &mut EntityManager, ps: &mut PhysicsState) {
    let orphaned = em.get_all_orphaned_weapon_ids();

    for id in orphaned.iter() {
        if let Some(ph) = em.physics_handles.get(*id) {
            let wrb = ps.rigid_body_set.get_mut(ph.rigid_body).unwrap();
            let col = ps.collider_set.get_mut(ph.collider).unwrap();

            if wrb.body_type() != RigidBodyType::Dynamic {
                wrb.set_body_type(RigidBodyType::Dynamic, true);
                wrb.set_gravity_scale(1.0, true);
                wrb.enable_ccd(true);
                wrb.wake_up(true);

                col.set_sensor(false);
                col.set_density(50.0);
                col.set_enabled(true);
                // wrb.apply_impulse(Vec3::new(0.0, 3.0, 0.0).into(), true);
            }
        }
    }
}
