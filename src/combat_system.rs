use glam::{vec3, Vec3};

use crate::{entity_manager::EntityManager, enums_types::{AnimationType, Faction, Knockback, PlayerState, SimState}, physics::PhysicsState};

pub fn update(
    em: &mut EntityManager,
    dt: f32,
    ps: &mut PhysicsState,
) {
    handle_player_to_enemy(em, ps);
    handle_enemy_to_player(em, ps);
}

fn handle_player_to_enemy(em: &mut EntityManager, ps: &mut PhysicsState) {
    let attacking_player_ids = em.player_get_ids_for_state(PlayerState::Attacking);

    for player_id in attacking_player_ids {
        let yaw = em.yaws.get(player_id).unwrap();
        let active_item = em.active_items.get(player_id).unwrap();
        let rh_id = active_item.right_hand.unwrap(); // Only doing right hand weeapons right now.
        let hitset = em.hitsets.get_mut(rh_id).unwrap();

        let rh_parent = em.parents.iter().find(|p| p.value().parent_id == rh_id).unwrap();
        let rh_w_col_handle = em.collider_to_entity.iter().find(|c| *c.1 == rh_parent.key()).unwrap().0;


        let player_cyl_handle = em.physics_handles.get(player_id).unwrap().collider;
        
        let anim = em.animators
            .get(player_id)
            .unwrap()
            .animations
            .get(&AnimationType::Slash)
            .unwrap();

        let active = anim
            .hurtbox_activation
            .as_ref()
            .map_or(false, |ha| ha.triggered.get());

        if !active {
            hitset.clear();               // important: reset when inactive
            return;                       // skip this frame
        }

        for (c1, c2, i) in ps.narrow_phase.intersection_pairs_with(*rh_w_col_handle) {
            if i {
                if c1 == player_cyl_handle || c2 == player_cyl_handle { continue; }

                let other = if c1 == *rh_w_col_handle { c2 } else { c1 };
                let target_id = *em.collider_to_entity.get(&other).unwrap();


                if !hitset.insert(other) { continue };

                if let Some(ph) = em.physics_handles.get(target_id) {
                    if let Some(rb) = ps.rigid_body_set.get_mut(ph.rigid_body) {
                        if let Some(h) = em.healths.get_mut(target_id) { *h -= 50.0 };
                        let dir = vec3(yaw.sin(), 1.0, yaw.cos()).normalize();
                        let strength = 4.0;
                        rb.apply_impulse((dir * strength).into(), true);
                        em.knockbacks.insert(target_id, Knockback { ttl: 0.35 });
                    }
                }
            }
        }
    }
}

fn handle_enemy_to_player(em: &mut EntityManager, ps: &mut PhysicsState) {
    let attacking_enemy_ids = em.enemy_get_ids_for_state(SimState::Attacking);
}

// pub fn update(
//     em: &mut EntityManager,
//      dt: f32,
//      ps: &mut PhysicsState,
//  ) {
//      let attacking_state_ids = em.player_get_ids_for_state(PlayerState::Attacking);
//      
//      for player_id in attacking_state_ids {
//          let active_item = em.active_items.get(player_id).unwrap();
//          let rh_id = active_item.right_hand.unwrap();
//  
//          let hb_parent = em.parents.iter().find(|p| p.value().parent_id == rh_id).unwrap();
//          let hb_collider_handle = em.collider_to_entity.iter().find(|c| *c.1 == hb_parent.key()).unwrap().0;
//  
//          let cyl_collider_handle = em.physics_handles.get(player_id).unwrap().collider;
//  
//          let anim = em.animators
//              .get(player_id)
//              .unwrap()
//              .animations
//              .get(&AnimationType::Slash)
//              .unwrap();
//          
//  
//          for (c1, c2, i) in ps.narrow_phase.intersection_pairs_with(*hb_collider_handle) {
//              if i {
//                  if c1 == cyl_collider_handle || c2 == cyl_collider_handle { continue; }
//                  
//                  if let Some(ha) = &anim.hurtbox_activation {
//                      if ha.triggered.get() {
//                          println!("DOING DAMAGE??!!!!!???!!?!!?!!");
//                      } else {
//                          println!("Collided but not activated");
//                      }
//                  }
//              }
//          }
//          
//      }
//  }
