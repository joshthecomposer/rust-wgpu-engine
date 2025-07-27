use glam::{vec2, Vec2};

use crate::{entity_manager::EntityManager, enums_types::Faction};

pub fn update(em: &mut EntityManager) {
    handle_entity_collisions(em);
}

fn handle_entity_collisions(em: &mut EntityManager) {
    let mut resolutions: Vec<(usize, Vec2)> = vec![];

    for c1 in em.cylinders.iter() {
        for c2 in em.cylinders.iter() {
            if c1.key() >= c2.key() {
                continue;
            }

            if let (Some(p1), Some(p2)) = (em.parents.get(c1.key()), em.parents.get(c2.key())) {
                if let (Some(f1), Some(f2)) = (em.factions.get(p1.parent_id), em.factions.get(p2.parent_id)) {
                    if *f1 == Faction::Static || *f2 == Faction::Static {
                        continue;
                    }

                    if *f1 == Faction::Item || *f2 == Faction::Item {
                        continue;
                    }
                }
            }

            let id1 = c1.key();
            let id2 = c2.key();

            let t1 = em.transforms.get(id1);
            let t2 = em.transforms.get(id2);
            if t1.is_none() || t2.is_none() {
                continue;
            }

            let t1 = t1.unwrap();
            let t2 = t2.unwrap();

            let cyl1 = c1.value();
            let cyl2 = c2.value();

            // Horizontal overlap (XZ-plane)
            let delta = vec2(t1.position.x - t2.position.x, t1.position.z - t2.position.z);
            let dist_sq = delta.length_squared();
            let radius_sum = cyl1.r + cyl2.r;

            let overlap_horizontal = dist_sq < (radius_sum * radius_sum);

            // Vertical overlap (Y-axis)
            let y1_min = t1.position.y;
            let y1_max = y1_min + cyl1.h;
            let y2_min = t2.position.y;
            let y2_max = y2_min + cyl2.h;
            let overlap_vertical = y1_min < y2_max && y1_max > y2_min;

            if overlap_horizontal && overlap_vertical {
                let dist = dist_sq.sqrt();
                let mtv = if dist != 0.0 {
                    let penetration = radius_sum - dist;
                    let direction = delta / dist;
                    direction * penetration
                } else {
                    vec2(1.0, 1.0).normalize() * 0.01
                };

                resolutions.push((id1, mtv * 0.5));
                resolutions.push((id2, -mtv * 0.5));
            }
        }
    }

    for (child_id, offset) in resolutions {
        let parent_id = em.parents.get(child_id).unwrap().parent_id;
        if let Some(t) = em.transforms.get_mut(parent_id) {
            t.position.x += offset.x;
            t.position.z += offset.y;
        }
    }
}
