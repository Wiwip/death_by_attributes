use crate::abilities::GameAbilityComponent;
use crate::context::GameAttributeContextMut;
use crate::effect::{
    apply_instant_effect, apply_realtime_effect, GameEffectContainer, GameEffectDuration,
    GameEffectEvent, GameEffectPeriod,
};
use crate::events::CurrentValueUpdateTrigger;
use crate::modifiers::{Modifier, ModifierAggregator};
use bevy::prelude::*;
use bevy::utils::HashSet;

pub fn handle_apply_effect_events(
    mut query: Query<EntityMut>,
    mut event_reader: EventReader<GameEffectEvent>,
    mut context: GameAttributeContextMut,
) {
    for ev in event_reader.read() {
        if let Ok(mut entity_mut) = query.get_mut(ev.entity) {
            // The context having an entity_ref blocks us from having a mutable ref to the game effect container
            // thus the addition to the vector list

            match &ev.effect.duration {
                GameEffectDuration::Instant => {
                    apply_instant_effect(&mut context, &mut entity_mut, &ev.effect);
                }
                GameEffectDuration::Duration(_) => {
                    if let Some(gec) = context.get_effect_container(&entity_mut) {
                        gec.add_effect(&ev.effect);
                    };
                }
                GameEffectDuration::Permanent => {
                    if let Some(gec) = context.get_effect_container(&entity_mut) {
                        gec.add_effect(&ev.effect);
                    };
                }
            }
        } else {
            warn!(
                "Attempted to apply an effect to an invalid entity [{:?}] or it doesn't have a GameEffectContainer component.",
                ev.entity
            )
        }
    }
}

pub fn tick_active_effects(mut query: Query<&mut GameEffectContainer>, time: Res<Time>) {
    for mut gec in &mut query {
        for effect in &mut gec.effects.try_lock().unwrap().iter_mut() {
            effect.tick_effect(time.delta());
        }
        gec.remove_expired_effects();
    }
}

pub fn update_attribute_base_value(
    mut query: Query<EntityMut, With<GameEffectContainer>>,
    time: Res<Time>,
    context: GameAttributeContextMut,
) {
    for entity_mut in query.iter_mut() {
        if let Some(gec) = context.get_effect_container(&entity_mut) {
            let effect_lock = gec.effects.try_lock().unwrap();
            for effect in effect_lock.iter() {
                if let Some(period) = &effect.periodic_application {
                    match period {
                        GameEffectPeriod::Periodic(timer) => {
                            if timer.just_finished() {
                                apply_instant_effect(&context, &entity_mut, effect);
                            }
                        }
                        GameEffectPeriod::Realtime => {
                            apply_realtime_effect(
                                &context,
                                &entity_mut,
                                effect,
                                time.delta_seconds(),
                            );
                        }
                    }
                }
            }
        };
    }
}

pub fn update_attribute_current_value(
    mut commands: Commands,
    mut query: Query<EntityMut>,
    context: GameAttributeContextMut,
) {
    for entity_mut in query.iter_mut() {
        let mut modifier_list = Vec::new();
        let mut attribute_list = HashSet::new();

        let Some(gec) = context.get_effect_container(&entity_mut) else {
            continue;
        };

        let effect_lock = gec.effects.try_lock().unwrap();
        for effect in effect_lock.iter() {
            if effect.periodic_application.is_none() {
                modifier_list.extend(&effect.modifiers);

                for modifier in &effect.modifiers {
                    attribute_list.insert(modifier.get_attribute_id());
                }
            }
        }

        for attribute_id in attribute_list {
            let aggregate: ModifierAggregator = modifier_list
                .iter()
                .filter(|&item| attribute_id == item.get_attribute_id())
                .map(|&item| match item {
                    Modifier::Scalar(scalar_mod) => ModifierAggregator::from(scalar_mod),
                    Modifier::Meta(meta_mod) => {
                        let scalar_mod_option = context.convert_modifier(&entity_mut, meta_mod);
                        if let Some(scalar_mod) = scalar_mod_option {
                            ModifierAggregator::from(&scalar_mod)
                        } else {
                            ModifierAggregator::default()
                        }
                    }
                })
                .sum();

            if let Some(attribute) = context.get_mut_by_id(&entity_mut, attribute_id) {
                attribute.current_value = aggregate.get_current_value(attribute.base_value);
                commands.trigger_targets(CurrentValueUpdateTrigger, entity_mut.id());
            }
        }
    }
}

pub fn tick_ability_cooldowns(mut query: Query<&mut GameAbilityComponent>, time: Res<Time>) {
    for mut gac in &mut query {
        for (_, ability) in &mut gac.abilities {
            ability.cooldown.write().unwrap().tick(time.delta());
        }
    }
}
