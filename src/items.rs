use glam::{Mat4, Vec3};
use rapier3d::prelude::RigidBodyType;
use crate::{entity_manager::EntityManager, physics::PhysicsState, util::data_structure::HashMapGetPairMut};

pub fn update(em: &mut EntityManager, ps: &mut PhysicsState) {
    active_items_pass(em);
    orphaned_items_pass(em, ps);
}

fn active_items_pass(em: &mut EntityManager) {
    for a in em.active_items.iter_mut() {
        let owner_id = a.key();

        let owner_trans = em.transforms.get(owner_id).unwrap();
        let owner_model_trans = Mat4::from_scale_rotation_translation(
            owner_trans.scale,
            owner_trans.rotation,
            owner_trans.position,
        );

        let owner_skellington = em.skellingtons.get(owner_id).unwrap();
        let animator = em.animators.get_mut(owner_id).unwrap();
        let blend_factor = animator.blend_factor;

        let owner_item_bones = em.item_bones.get(owner_id).unwrap();

        let current_key = animator.current_animation.clone();
        let next_key = animator.next_animation.clone();
        let rh_name = owner_item_bones.rh_name.as_str();

        let rh_weapon_id = a.value().right_hand.unwrap();

        let maybe_bone_world_model_space = if blend_factor > 0.0 && current_key != next_key {
            // SAFELY get both entries with mutable refs (no HashMap::remove)
            let (a1, a2) = {
                match animator.animations.get_pair_mut(&current_key, &next_key) {
                    Some((a1, a2)) => (a1, a2),
                    None => {
                        eprintln!("WARNING: both animations do not exist in animator.get_pair_mut {}, {}", &current_key, &next_key);
                        return;
                    },
                }
            };

            a1.get_raw_global_bone_transform_by_name_blended(
                rh_name,
                owner_skellington,
                Mat4::IDENTITY,
                a2,
                blend_factor,
            )
        } else {
            animator.animations
                .get_mut(&current_key)
                .unwrap()
                .get_raw_global_bone_transform_by_name(
                    rh_name,
                    owner_skellington,
                    Mat4::IDENTITY,
                )
        };

        if let Some(bone_world_model_space) = maybe_bone_world_model_space {
            let bone_world_space = owner_model_trans * bone_world_model_space;
            let (_, rot, pos) = bone_world_space.to_scale_rotation_translation();

            let weapon_trans = em.transforms.get_mut(rh_weapon_id).unwrap();
            weapon_trans.position = pos;
            weapon_trans.rotation = rot * weapon_trans.original_rotation;
        }
    }
}

fn orphaned_items_pass(em: &mut EntityManager, ps: &mut PhysicsState) {

    let orphaned = em.get_all_orphaned_weapon_ids();

    for id in orphaned.iter() {
        if let Some(ph) = em.physics_handles.get(*id){
            let wrb = ps.rigid_body_set.get_mut(ph.rigid_body).unwrap();
            let col = ps.collider_set.get_mut(ph.collider).unwrap();

            if wrb.body_type() != RigidBodyType::Dynamic {
                wrb.set_body_type(RigidBodyType::Dynamic, true);
                wrb.set_gravity_scale(1.0, true);
                wrb.enable_ccd(false); // TODO: Why does enabling ccd slow this down so much?
                wrb.wake_up(true);

                col.set_sensor(false);
                col.set_density(50.0);
                col.set_enabled(true);

                wrb.apply_impulse(Vec3::new(0.0, 3.0, 0.0).into(), true);
            }
        }
    }
}
