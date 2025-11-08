use crate::{entity_manager::EntityManager, enums_types::EntityType, physics::PhysicsState, some_data::{GROUP_PLAYER, GROUP_TERRAIN}};
use rapier3d::prelude::*;

pub fn grounding_solver(em: &mut EntityManager, ps: &PhysicsState) {
    let ids = vec![
        em.get_ids_for_type(EntityType::TrashGuy),
        em.get_ids_for_type(EntityType::MooseMan)
    ]
    .concat();

    for id in ids.iter() {
        let ph = em.physics_handles.get(*id).unwrap();
        let ch = ph.collider;
        let rb_handle = ph.rigid_body;

        let trans = em.transforms.get(*id).unwrap();
        let colliders = &ps.collider_set;
        let bodies = &ps.rigid_body_set;
        let query = ps.query_pipeline.as_ref().unwrap();
        //let r = collider.radius;
        let gs = em.grounded_states.get_mut(*id).unwrap();
        let position = trans.position;

        let ray = Ray::new(point![position.x, position.y + 0.02, position.z], vector![0.0, -1.0, 0.0]);

        let filter = QueryFilter::default()
            .groups(InteractionGroups::new(GROUP_PLAYER.into(), GROUP_TERRAIN.into()))
            .exclude_rigid_body(rb_handle)
            .exclude_sensors()
            .into();

        let dist = match gs.is_grounded {
            true => gs.ray_length_grounded,
            false => gs.ray_length_airborn
        };

        let prev = gs.is_grounded;
        let result = query.cast_ray(bodies, colliders, &ray, dist, true, filter);

        gs.is_grounded = result.is_some();
        gs.just_landed = !prev && gs.is_grounded;
        gs.just_left = prev && !gs.is_grounded;
    }
}
