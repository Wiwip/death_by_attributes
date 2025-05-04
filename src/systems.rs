use crate::abilities::GameAbilityContainer;
use crate::effects::{Effect, EffectDuration, EffectPeriodicTimer, MutationAggregatorCache};
use crate::mutator::{ModAggregator, StoredMutator};
use crate::{AttributeEntityMut, BaseValueChanged, CurrentValueChanged};
use bevy::platform::collections::HashMap;
use bevy::prelude::*;
use std::any::TypeId;
use std::ops::{Deref, DerefMut};

pub fn tick_effects_periodic_timer(mut query: Query<&mut EffectPeriodicTimer>, time: Res<Time>) {
    query.par_iter_mut().for_each(|mut timer| {
        timer.0.tick(time.delta());
    });
}

pub fn tick_effects_duration_timer(mut query: Query<&mut EffectDuration>, time: Res<Time>) {
    query.par_iter_mut().for_each(|mut duration| {
        match duration.deref_mut() {
            EffectDuration::Permanent => { /* Mo timer to tick */ }
            EffectDuration::Duration(timer) => {
                timer.tick(time.delta());
            }
        };
    });
}

pub fn update_base_values(
    effects: Query<(&Effect, &EffectPeriodicTimer)>,
    mut entities: Query<(Entity, &Children, AttributeEntityMut), Without<Effect>>,
    mut mutation_cache: ResMut<MutationAggregatorCache>,
    mut commands: Commands,
) {
    let mut updated_entities = Vec::new();

    for (applied_entity, applied_effects, mut entity_mut) in entities.iter_mut() {
        let mut modifiers: HashMap<TypeId, (StoredMutator, ModAggregator)> = Default::default();

        for effect_entity in applied_effects.iter() {
            let Ok((effect, periodic_timer)) = effects.get(effect_entity) else {
                continue;
            };

            if !periodic_timer.just_finished() {
                continue;
            }

            updated_entities.push(applied_entity);

            for mutator in &effect.modifiers {
                let (_, aggregator) = modifiers
                    .entry(mutator.0.target())
                    .or_insert_with(|| (mutator.clone(), ModAggregator::default()));

                if let Ok(value) = &mutator.0.to_aggregator() {
                    // Update the aggregator to be applied
                    aggregator.additive = aggregator.additive + value.additive;
                    aggregator.multi = aggregator.multi + value.multi;

                    // Round-about way to set the dirty_bool to true
                    let type_map = mutation_cache.evaluators.entry(applied_entity).or_default();
                    let (_, _, _, current_value_dirty) = type_map
                        .entry(mutator.0.target())
                        .or_insert((mutator.clone(), ModAggregator::default(), false, false));
                    *current_value_dirty = true;
                }
            }
        }

        for (_, (mutator, aggregator)) in modifiers {
            mutator
                .0
                .apply_aggregator(entity_mut.reborrow(), aggregator);
        }
    }

    // Notify updated entities that their base values has changed.
    if !updated_entities.is_empty() {
        commands.trigger_targets(CurrentValueChanged, updated_entities);
    }
}

pub fn update_current_values(
    mut entities: Query<(Entity, AttributeEntityMut)>,
    mut evaluation_state: ResMut<MutationAggregatorCache>,
    mut commands: Commands,
) {
    let mut updated_entities = Vec::new();

    for (entity, mut entity_mut) in entities.iter_mut() {
        let Some(type_map) = evaluation_state.evaluators.get_mut(&entity) else {
            continue;
        };

        for (stored_mutator, stored_aggregator, _, current_value_dirty) in type_map.values_mut() {
            if *current_value_dirty {
                stored_mutator.update_current_value(entity_mut.reborrow(), *stored_aggregator);

                updated_entities.push(entity);
            }

            *current_value_dirty = false;
        }
    }

    // Notify updated entities that their current values has changed.
    if !updated_entities.is_empty() {
        commands.trigger_targets(BaseValueChanged, updated_entities);
    }
}

pub fn tick_ability_cooldowns(mut query: Query<&mut GameAbilityContainer>, time: Res<Time>) {
    for mut abilities in &mut query {
        for (_, ability) in abilities.get_abilities_mut().iter_mut() {
            ability.cooldown.tick(time.delta());
        }
    }
}

pub fn on_instant_effect_added(
    trigger: Trigger<OnAdd, Effect>,
    query: Query<(&ChildOf, &Effect), Without<EffectDuration>>,
    mut entities: Query<AttributeEntityMut>,
    mut commands: Commands,
) {
    let effect_entity = trigger.target();
    let Ok((effect_target, effect)) = query.get(effect_entity) else {
        return;
    };

    let modifiers = &effect.modifiers;
    for modifier in modifiers.iter() {
        let entity_mut = entities.get_mut(effect_target.0).unwrap();
        if let Ok(aggregator) = modifier.0.to_aggregator() {
            let _ = modifier.0.apply_aggregator(entity_mut, aggregator);
        }
    }

    // Notify entity that a value has changed
    commands.trigger_targets(BaseValueChanged, effect_target.0);
    commands.trigger_targets(CurrentValueChanged, effect_target.0);

    // Despawn the effect since it is instant
    commands.entity(effect_entity).despawn();
}

pub fn on_effect_removed(
    trigger: Trigger<OnRemove, Effect>,
    query: Query<(&ChildOf, &Effect), With<EffectDuration>>,
    mut mutation_cache: ResMut<MutationAggregatorCache>,
    mut commands: Commands,
) {
    let effect_entity = trigger.target();
    let Ok((target_entity, effect)) = query.get(effect_entity) else {
        return;
    };

    let type_map = mutation_cache
        .evaluators
        .entry(target_entity.0)
        .or_default();

    for modifier in effect.modifiers.iter() {
        let (_, stored_aggregator, _, current_value_dirty) = type_map
            .entry(modifier.0.target())
            .or_insert((modifier.clone(), ModAggregator::default(), false, false));

        if let Ok(aggregator) = modifier.0.to_aggregator() {
            *stored_aggregator -= aggregator;
            *current_value_dirty = true;
        } else {
            *current_value_dirty = false;
        }
    }

    // Notify entity that a value has changed
    commands.trigger_targets(BaseValueChanged, target_entity.0);
    commands.trigger_targets(CurrentValueChanged, target_entity.0);
}

/*pub fn on_duration_effect_added(
    trigger: Trigger<OnAdd, Effect>,
    query: Query<(&ChildOf, &Effect), With<EffectDuration>>,
    mut mutation_cache: ResMut<MutationAggregatorCache>,
) {
    let effect_entity = trigger.target();
    let Ok((effect_target, effect)) = query.get(effect_entity) else {
        return;
    };

    let type_map = mutation_cache
        .evaluators
        .entry(effect_target.0)
        .or_default();

    println!("on_duration_effect_added");
    println!("trigger.target: {:?}", effect_entity);
    println!("target_entity: {:?}", effect_entity);

    for mutator in effect.modifiers.iter() {
        let (_, stored_aggregator, _, current_value_dirty) = type_map
            .entry(mutator.0.target())
            .or_insert((mutator.clone(), ModAggregator::default(), false, false));

        /*if let Ok(aggregator) = mutator.0.to_aggregator() {
            *stored_aggregator += aggregator;
            *current_value_dirty = true;
        } else {
            *current_value_dirty = false;
        }*/
    }
}*/
