use crate::attributes::{GameAttributeContextMut};
use bevy::ecs::component::Components;
use bevy::prelude::*;
use bevy::utils::HashSet;

use crate::effect::{
    apply_instant_effect, apply_realtime_effect, GameEffectContainer, GameEffectDuration,
    GameEffectEvent, GameEffectPeriod,
};
use crate::events::CurrentValueUpdateTrigger;
use crate::modifiers::{Modifier, ModifierAggregator};

pub fn handle_apply_effect_events(
    mut query: Query<(EntityMut)>,
    mut event_reader: EventReader<GameEffectEvent>,
    type_registry: Res<AppTypeRegistry>,
    components: &Components,
) {
    for ev in event_reader.read() {
        if let Ok(entity_mut) = query.get_mut(ev.entity) {
            // The context having an entity_ref blocks us from having a mutable ref to the game effect container
            // thus the addition to the vector list
            let context = GameAttributeContextMut {
                entity_mut,
                components,
                type_registry: type_registry.0.clone(),
            };

            match &ev.effect.duration {
                GameEffectDuration::Instant => {
                    apply_instant_effect(&context, &ev.effect);
                }
                GameEffectDuration::Duration(_) => {
                    if let Some(gec) = context.get_game_effect_container() {
                        gec.add_effect(&ev.effect);
                    };
                }
                GameEffectDuration::Permanent => {
                    if let Some(gec) = context.get_game_effect_container() {
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
    mut query: Query<(EntityMut)>,
    time: Res<Time>,
    type_registry: Res<AppTypeRegistry>,
    components: &Components,
) {
    for entity_mut in query.iter_mut() {
        let context = GameAttributeContextMut {
            entity_mut,
            components,
            type_registry: type_registry.0.clone(),
        };

        if let Some(gec) = context.get_game_effect_container() {
            let effect_lock = gec.effects.try_lock().unwrap();
            for effect in effect_lock.iter() {
                if let Some(period) = &effect.periodic_application {
                    match period {
                        GameEffectPeriod::Periodic(timer) => {
                            if timer.just_finished() {
                                apply_instant_effect(&context, effect);
                            }
                        }
                        GameEffectPeriod::Realtime => {
                            apply_realtime_effect(&context, effect, time.delta_seconds());
                        }
                    }
                }
            }
        };
    }
}

pub fn update_attribute_current_value(
    mut commands: Commands,
    mut query: Query<(Entity, EntityMut)>,
    type_registry: Res<AppTypeRegistry>,
    components: &Components,
) {
    for (entity, entity_mut) in query.iter_mut() {
        let mut modifier_list = Vec::new();
        let mut attribute_list = HashSet::new();

        let context = GameAttributeContextMut {
            entity_mut,
            components,
            type_registry: type_registry.0.clone(),
        };

        let Some(gec) = context.get_game_effect_container() else {
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
                        let scalar_mod_option = context.convert_modifier(meta_mod);
                        if let Some(scalar_mod) = scalar_mod_option {
                            ModifierAggregator::from(&scalar_mod)
                        } else {
                            ModifierAggregator::default()
                        }
                    }
                })
                .sum();

            if let Some(attribute) = context.get_attribute_mut(attribute_id) {
                attribute.current_value = aggregate.get_current_value(attribute.base_value);
                commands.trigger_targets(CurrentValueUpdateTrigger, entity);
            }
        }
    }
}

