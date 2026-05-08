use crate::{
    command_buffer::CommandBuffer, entity_manager::EntityManager, enums_types::StatusEffectBehavior,
};

pub fn update(em: &mut EntityManager, dt: f32, _cmds: &mut CommandBuffer) {
    let ids = em
        .status_effects
        .iter()
        .map(|entry| entry.key())
        .collect::<Vec<_>>();

    for id in ids {
        let Some(effects) = em.status_effects.get_mut(id) else {
            continue;
        };

        for effect in effects.iter_mut() {
            effect.remaining -= dt;

            for behavior in &mut effect.behaviors {
                match behavior {
                    StatusEffectBehavior::PeriodicDamage {
                        damage_per_tick,
                        tick_interval,
                    } => {
                        effect.tick_accumulator += dt;

                        if effect.tick_accumulator >= *tick_interval {
                            effect.tick_accumulator -= *tick_interval;

                            let health = em.healths.get_mut(id).unwrap();

                            *health -= *damage_per_tick;
                            println!("STATUS EFFECTTTTTT {}", health);
                        }
                    }
                    StatusEffectBehavior::StatModifier { stat, op, amount } => (),
                }
            }
        }
    }
}
