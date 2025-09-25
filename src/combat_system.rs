use crate::{entity_manager::EntityManager, enums_types::{AnimationType, Faction, PlayerState}, physics::PhysicsState};

pub fn update(
    em: &mut EntityManager,
    dt: f32,
    ps: &mut PhysicsState,
) {
    let attacking_state_ids = em.player_get_ids_for_state(PlayerState::Attacking);
    
    for player_id in attacking_state_ids {
        let active_item = em.active_items.get(player_id).unwrap();
        let rh_id = active_item.right_hand.unwrap();

        let hb_parent = em.parents.iter().find(|p| p.value().parent_id == rh_id).unwrap();
        let hb_collider_handle = em.collider_to_entity.iter().find(|c| *c.1 == hb_parent.key()).unwrap().0;

        let cyl_collider_handle = em.physics_handles.get(player_id).unwrap().collider;

        let anim = em.animators
            .get(player_id)
            .unwrap()
            .animations
            .get(&AnimationType::Slash)
            .unwrap();
        

        for (c1, c2, i) in ps.narrow_phase.intersection_pairs_with(*hb_collider_handle) {
            if i {
                if c1 == cyl_collider_handle || c2 == cyl_collider_handle { continue; }
                
                if let Some(ha) = &anim.hurtbox_activation {
                    if ha.triggered.get() {
                        println!("DOING DAMAGE??!!!!!???!!?!!?!!");
                    } else {
                        println!("Collided but not activated");
                    }
                }
            }
        }
        
    }
}
