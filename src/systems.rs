use crate::abilities::GameAbilityContainer;
use crate::effects::{Effect, EffectDuration, EffectPeriodicTimer, EffectTarget};
use crate::mutator::{EffectMutators, ModAggregator, Mutator};
use crate::{
    ActorEntityMut, CachedMutations, OnAttributeMutationChanged, OnBaseValueChanged,
    OnCurrentValueChanged,
};
use bevy::ecs::relationship::Relationship;
use bevy::prelude::*;
use std::ops::DerefMut;

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

pub fn on_instant_effect_applied(
    trigger: Trigger<OnAdd, Effect>,
    effects: Query<(&EffectTarget, &EffectMutators, &Effect), Without<EffectDuration>>,
    mutators: Query<&Mutator>,
    mut entities: Query<ActorEntityMut>,
    mut cache: ResMut<CachedMutations>,
    mut commands: Commands,
) {
    let effect_entity = trigger.target();

    let Ok((actor_entity, mutator_entities, _)) = effects.get(effect_entity) else {
        warn_once!("instant_effect_applied failed for {}", effect_entity);
        return;
    };

    let type_map = cache.evaluators.entry(actor_entity.get()).or_default();

    for mutator_entity in mutator_entities.iter() {
        let Ok(mutator) = mutators.get(mutator_entity) else {
            continue;
        };

        // Initialise the cache so current values are properly updated
        let _ = type_map
            .entry(mutator.0.target())
            .or_insert((mutator.clone(), ModAggregator::default()));

        // Once recovered, apply the aggregator
        let aggregator = mutator.to_aggregator();
        let entity_mut = entities.get_mut(actor_entity.get()).unwrap();
        let _ = mutator.apply_aggregator(entity_mut, aggregator);
    }

    // Notify actor entity that a base value has changed
    commands.trigger_targets(OnBaseValueChanged, actor_entity.get());

    // Despawn the effect since it is instant
    commands.entity(effect_entity).despawn();
}

pub fn on_duration_effect_applied(
    trigger: Trigger<OnAdd, Effect>,
    effects: Query<
        (&EffectTarget, &EffectMutators, &Effect),
        (With<EffectDuration>, Without<EffectPeriodicTimer>),
    >,
    mutators: Query<&Mutator>,
    mut cache: ResMut<CachedMutations>,
    mut commands: Commands,
) {
    let mut updated = false;
    let effect_entity = trigger.target();
    let Ok((actor_entity, mutator_entities, _)) = effects.get(effect_entity) else {
        warn_once!("on_duration_effect_added failed for {}", effect_entity);
        return;
    };

    let type_map = cache.evaluators.entry(actor_entity.get()).or_default();

    for mutator_entity in mutator_entities.iter() {
        let Ok(mutator) = mutators.get(mutator_entity) else {
            warn_once!("failed to retrieve a mutator for {}", mutator_entity);
            continue;
        };

        // Query and update the cached aggregators
        let (_, stored_aggregator) = type_map
            .entry(mutator.0.target())
            .or_insert((mutator.clone(), ModAggregator::default()));
        let aggregator = mutator.0.to_aggregator();
        *stored_aggregator += aggregator;

        updated = true;
    }

    // Notify updated entities that their base values has changed.
    if updated {
        commands.trigger_targets(OnAttributeMutationChanged, actor_entity.get());
    }
}

pub fn trigger_periodic_effects(
    mut entities: Query<ActorEntityMut, (Without<Effect>, Without<Mutator>)>,
    effects: Query<
        (
            &EffectTarget,
            &EffectMutators,
            &Effect,
            &EffectPeriodicTimer,
        ),
        With<EffectDuration>,
    >,
    mutators: Query<&Mutator>,
    mut cache: ResMut<CachedMutations>,
    mut commands: Commands,
) {
    let mut updated_entities: Vec<Entity> = Vec::new();

    for (actor_entity, mutator_entities, _, periodic_timer) in effects.iter() {
        if !periodic_timer.just_finished() {
            continue;
        }

        let type_map = cache.evaluators.entry(actor_entity.get()).or_default();
        updated_entities.push(actor_entity.get());

        for mutator_entity in mutator_entities.iter() {
            let Ok(mutator) = mutators.get(mutator_entity) else {
                continue;
            };

            // Necessary for the cache to be initialized. Kind of a hack...
            let _ = type_map
                .entry(mutator.0.target())
                .or_insert((mutator.clone(), ModAggregator::default()));

            // Once recovered, apply the aggregator
            let aggregator = mutator.to_aggregator();
            if let Ok(entity_mut) = entities.get_mut(actor_entity.get()) {
                let _ = mutator.apply_aggregator(entity_mut, aggregator);
            }
        }
    }

    // Notify updated entities that their base values has changed.
    if !updated_entities.is_empty() {
        commands.trigger_targets(OnBaseValueChanged, updated_entities);
    }
}

pub fn on_base_value_changed(
    trigger: Trigger<OnBaseValueChanged>,
    mut entities: Query<ActorEntityMut, (Without<Effect>, Without<Mutator>)>,
    cache: Res<CachedMutations>,
    mut commands: Commands,
) {
    let updated_entity = trigger.target();

    let Ok(mut entity_mut) = entities.get_mut(updated_entity) else {
        println!("update_current_values failed for {}", updated_entity);
        return;
    };
    let Some(type_map) = cache.evaluators.get(&updated_entity) else {
        println!(
            "cache.evaluators.get(&updated_entity) {:?}\n{:#?}",
            updated_entity, cache.evaluators
        );
        return;
    };

    let mut updated_value = false;

    for (stored_mutator, stored_aggregator) in type_map.values() {
        let result = stored_mutator.update_current_value(entity_mut.reborrow(), *stored_aggregator);
        if updated_value == false && result == true {
            updated_value = true;
        }
    }

    // Notify entity of the changed attributes
    if updated_value {
        commands.trigger_targets(OnCurrentValueChanged, updated_entity);
    }
}

pub fn on_attribute_mutation_changed(
    trigger: Trigger<OnAttributeMutationChanged>,
    mut entities: Query<ActorEntityMut, (Without<Effect>, Without<Mutator>)>,
    cache: Res<CachedMutations>,
    mut commands: Commands,
) {
    let updated_entity = trigger.target();

    let Ok(mut entity_mut) = entities.get_mut(updated_entity) else {
        println!("update_current_values failed for {}", updated_entity);
        return;
    };
    let Some(type_map) = cache.evaluators.get(&updated_entity) else {
        println!(
            "cache.evaluators.get(&updated_entity) {:?}\n{:#?}",
            updated_entity, cache.evaluators
        );
        return;
    };

    let mut updated_value = false;

    for (stored_mutator, stored_aggregator) in type_map.values() {
        let result = stored_mutator.update_current_value(entity_mut.reborrow(), *stored_aggregator);
        if updated_value == false && result == true {
            updated_value = true;
        }
    }

    // Notify entity of the changed attributes
    if updated_value {
        commands.trigger_targets(OnCurrentValueChanged, updated_entity);
    }
}

pub(crate) fn tick_ability_cooldowns(mut query: Query<&mut GameAbilityContainer>, time: Res<Time>) {
    for mut abilities in &mut query {
        for (_, ability) in abilities.get_abilities_mut().iter_mut() {
            ability.cooldown.tick(time.delta());
        }
    }
}

pub fn check_duration_effect_expiry(
    query: Query<(Entity, &EffectDuration)>,
    mut commands: Commands,
) {
    for (entity, duration) in query.iter() {
        let EffectDuration::Duration(duration) = duration else {
            continue;
        };

        if !duration.finished() {
            continue;
        }

        info!("Effect expired {:?}", entity);
        commands.entity(entity).despawn();
    }
}

pub fn on_duration_effect_removed(
    trigger: Trigger<OnRemove, Effect>,
    effects: Query<
        (&EffectTarget, &EffectMutators, &Effect),
        (With<EffectDuration>, Without<EffectPeriodicTimer>),
    >,
    mutators: Query<&Mutator>,
    mut cache: ResMut<CachedMutations>,
    mut commands: Commands,
) {
    let mut updated = false;
    let effect_entity = trigger.target();
    let Ok((actor_entity, mutator_entities, _)) = effects.get(effect_entity) else {
        warn_once!("on_effect_removed failed for {}", effect_entity);
        return;
    };

    let type_map = cache.evaluators.entry(actor_entity.get()).or_default();

    for mutator_entity in mutator_entities.iter() {
        let Ok(mutator) = mutators.get(mutator_entity) else {
            warn_once!("failed to retrieve a mutator for {}", mutator_entity);
            continue;
        };

        // Query and update the cached aggregators
        let (_, stored_aggregator) = type_map
            .entry(mutator.0.target())
            .or_insert((mutator.clone(), ModAggregator::default()));
        let aggregator = mutator.0.to_aggregator();
        *stored_aggregator -= aggregator;

        updated = true;
    }

    // Notify updated entities that their base values has changed.
    if updated {
        commands.trigger_targets(OnBaseValueChanged, actor_entity.get());
    }
}
