use glam::Mat4;
use crate::{entity_manager::EntityManager, util::data_structure::HashMapGetPairMut};

pub fn update(em: &mut EntityManager) {
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

        let rh_parent = em.parents.iter().find( |p|
            p.value().parent_id == rh_weapon_id &&
            (em.cuboids.get(p.key()).is_some())
        ).unwrap();

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
