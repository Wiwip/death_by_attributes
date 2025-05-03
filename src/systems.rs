#![allow(warnings)]

use crate::abilities::GameAbilityContainer;
use crate::effects::{Effect, EffectDuration, EffectPeriodicApplication, EffectTarget, EvalStruct};
use crate::AttributeEntityMut;
use bevy::prelude::*;
use bevy::utils::TypeIdMap;
use std::time::Instant;

pub fn tick_effects_periodic_timer(mut query: Query<&mut EffectPeriodicApplication>, time: Res<Time>) {
    query.par_iter_mut().for_each(|mut timer| {
        timer.0.tick(time.elapsed());
    });
}

pub fn tick_effects_duration(mut query: Query<&mut EffectDuration>, time: Res<Time>) {
    let start = Instant::now();

    query.par_iter_mut().for_each(|mut timer| {
        timer.0.tick(time.elapsed());
    });

    let elapsed = start.elapsed();
    println!("tick_active_effects: {:?}", elapsed)
}


pub fn update_base_values(
    query: Query<(&Effect, &EffectTarget, &EffectPeriodicApplication)>,
    mut entities: Query<AttributeEntityMut>,
) {
    let start = Instant::now();

    for (effect, target, timer) in query.iter() {
        if timer.0.just_finished() {
            let Ok(mut entity_mut) = entities.get_mut(target.0) else {
                continue;
            };

            for modifier in &effect.modifiers {
                let Ok(aggregator) = modifier.0.get_aggregator(&mut entity_mut) else {
                    continue;
                };

                let _ = modifier.0.apply_from_aggregator(&mut entity_mut, aggregator);
            }
        }
    }

    let elapsed = start.elapsed();
    println!("update_attributes: {:?}", elapsed)
}

pub fn update_current_values(
    mut entities: Query<(Entity, AttributeEntityMut)>,
    evaluation_state: Res<EvalStruct>,
) {
    let start = Instant::now();

    for (entity, mut entity_mut) in entities.iter_mut() {
        match evaluation_state.evaluators.get(&entity) {
            None => {}
            Some(map) => {
                for (_, (mutator, aggregator)) in map {
                    let _ = mutator.0.apply_from_aggregator(&mut entity_mut, *aggregator);
                }
            }
        }
    }

    let elapsed = start.elapsed();
    println!("update_attributes: {:?}", elapsed)
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
    query: Query<&Effect, Without<EffectDuration>>,
    mut entities: Query<AttributeEntityMut>,
) {
    let entity = trigger.target();
    let Ok(effect) = query.get(entity) else {
        return;
    };

    let modifiers = &effect.modifiers;
    for modifier in modifiers.iter() {
        let mut entity_mut = entities.get_mut(entity).unwrap();
        if let Ok(aggregator) = modifier.0.get_aggregator(&mut entity_mut) {
            let _ = modifier
                .0
                .apply_from_aggregator(&mut entity_mut, aggregator);
        }
    }
}

pub fn on_duration_effect_added(
    trigger: Trigger<OnAdd, Effect>,
    query: Query<&Effect, With<EffectDuration>>,
    mut entities: Query<AttributeEntityMut>,
    mut evaluation_state: ResMut<EvalStruct>,
) {
    let entity = trigger.target();
    let Ok(effect) = query.get(entity) else {
        return;
    };

    let modifiers = &effect.modifiers;
    for modifier in modifiers.iter() {
        let mut entity_mut = entities.get_mut(entity).unwrap();

        let entity_effects = evaluation_state
            .evaluators
            .entry(entity)
            .or_insert_with(|| TypeIdMap::with_hasher(Default::default()));

        if let Ok(aggregator) = modifier.0.get_aggregator(&mut entity_mut) {
            let (_, current_aggregator) = entity_effects
                .entry(modifier.0.attribute_id())
                .or_insert_with(|| (modifier.clone(), aggregator));

            *current_aggregator += aggregator;
        }
    }
}

pub fn on_effect_removed(
    trigger: Trigger<OnRemove, Effect>,
    query: Query<&Effect, With<EffectDuration>>,
    mut entities: Query<AttributeEntityMut>,
    mut evaluation_state: ResMut<EvalStruct>,
) {
    let entity = trigger.target();
    let effect = query.get(entity).unwrap();

    let modifiers = &effect.modifiers;
    for modifier in modifiers.iter() {
        let mut entity_mut = entities.get_mut(entity).unwrap();

        let entity_effects = evaluation_state
            .evaluators
            .entry(entity)
            .or_insert_with(|| TypeIdMap::with_hasher(Default::default()));

        if let Ok(aggregator) = modifier.0.get_aggregator(&mut entity_mut) {
            let (_, current_aggregator) = entity_effects
                .entry(modifier.0.attribute_id())
                .or_insert_with(|| (modifier.clone(), aggregator));

            *current_aggregator -= aggregator;
        }
    }
}
