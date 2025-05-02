use crate::abilities::GameAbilityContainer;
use crate::effects::{GameEffectContainer, GameEffectDuration, GameEffectEvent, GameEffectPeriod};
use crate::mutator::{MutatorWrapper, ModAggregator};
use crate::{AttributeEntityMut, CurrentValueUpdateTrigger};
use bevy::prelude::Vec;
use bevy::prelude::*;
use bevy::utils::{PreHashMap, PreHashMapExt};
use std::any::TypeId;
use std::cell::RefCell;
use std::time::Instant;
use thread_local::ThreadLocal;

pub fn handle_apply_effect_events(
    mut query: Query<(AttributeEntityMut, &mut GameEffectContainer)>,
    mut event_reader: EventReader<GameEffectEvent>,
) {
    let start = Instant::now();

    for ev in event_reader.read() {
        if let Ok((mut entity_mut, mut container)) = query.get_mut(ev.entity) {
            match &ev.effect.duration {
                GameEffectDuration::Instant => {
                    for modifier in &ev.effect.modifiers {
                        modifier.0.apply(&mut entity_mut).unwrap()
                    }
                }
                GameEffectDuration::Duration(_) => {
                    container.add_effect(&ev.effect);
                }
                GameEffectDuration::Permanent => {
                    container.add_effect(&ev.effect);
                }
            }
        } else {
            warn!(
                "Attempted to apply an effect to an invalid entity [{:?}] or it doesn't have a GameEffectContainer component.",
                ev.entity
            )
        }
    }

    let elapsed = start.elapsed();
    //println!("handle_apply_effect_events: {:?}", elapsed)
}

pub fn tick_active_effects(mut query: Query<&mut GameEffectContainer>, time: Res<Time>) {
    let start = Instant::now();

    query.par_iter_mut().for_each(|mut container| {
        for effect in &mut container.effects.iter_mut() {
            effect.tick_effect(time.delta());
        }
        container.remove_expired_effects();
    });

    let elapsed = start.elapsed();
    //println!("tick_active_effects: {:?}", elapsed)
}

pub fn update_attribute_base_value(mut query: Query<(AttributeEntityMut, &GameEffectContainer)>) {
    let start = Instant::now();

    query
        .par_iter_mut()
        .for_each(|(mut entity_mut, container)| {
            for effect in &container.effects {
                if let Some(period) = &effect.periodic_application {
                    match period {
                        GameEffectPeriod::Periodic(timer) => {
                            if timer.just_finished() {
                                for modifier in &effect.modifiers {
                                    modifier.0.apply(&mut entity_mut);
                                }
                            }
                        }
                        _ => {}
                    }
                }
            }
        });

    let elapsed = start.elapsed();
    //println!("update_attribute_base_value: {:?}", elapsed)
}

#[derive(Default)]
pub struct EvalStruct {
    evaluators: PreHashMap<(TypeId, usize), ModAggregator>,
    modifiers: PreHashMap<(TypeId, usize), MutatorWrapper>,
}

#[derive(Default)]
pub(crate) struct UpdateTracker {
    entities: Vec<Entity>,
}

pub fn update_attribute_current_value(
    mut commands: Commands,
    mut query: Query<(Entity, AttributeEntityMut, &GameEffectContainer)>,
    entities: Local<ThreadLocal<RefCell<UpdateTracker>>>,
) {
    let start = Instant::now();

    let a = query
        .par_iter_mut()
        .for_each(|(entity, mut entity_mut, container)| {
            let mut evaluation_state = EvalStruct::default();

            for effect in container.effects.iter() {
                if effect.periodic_application.is_none() {
                    for modifier in &effect.modifiers {
                        evaluation_state
                            .modifiers
                            .get_or_insert_with(&modifier.0.evaluator_id(), || modifier.clone());

                        let mut value = evaluation_state
                            .evaluators
                            .get_or_insert_with(&modifier.0.evaluator_id(), || {
                                ModAggregator::default()
                            });

                        value += modifier.0.get_aggregator(&mut entity_mut);
                    }
                }
            }

            for (type_id, &aggregator) in evaluation_state.evaluators.iter() {
                let modifier_ref = evaluation_state.modifiers.get(type_id).unwrap();
                let _ = modifier_ref.0.apply_from_aggregator(&mut entity_mut, aggregator);
            }

            if !evaluation_state.evaluators.is_empty() {
                let mut updated_entities = entities.get_or_default().borrow_mut();
                let updated_entities = &mut *updated_entities;
                updated_entities.entities.push(entity);
            }
        });

    // Sends a trigger to the entities whose current value on any attributes was updated.
    let mut updated_entities = entities.get_or_default().borrow_mut();
    let updated_entities = &mut *updated_entities;

    // There's nothing to trigger if noi entities were updated
    if !updated_entities.entities.is_empty() {
        commands.trigger_targets(CurrentValueUpdateTrigger, updated_entities.entities.clone());
        updated_entities.entities.clear();
    }

    let elapsed = start.elapsed();
    //println!("update_attribute_current_value: {:?}", elapsed)
}

pub fn tick_ability_cooldowns(mut query: Query<&mut GameAbilityContainer>, time: Res<Time>) {
    for mut abilities in &mut query {
        for (_, ability) in abilities.get_abilities_mut().iter_mut() {
            ability.cooldown.tick(time.delta());
        }
    }
}
