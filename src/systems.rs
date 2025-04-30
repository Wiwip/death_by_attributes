use crate::AttributeEntityMut;
use crate::effects::{GameEffectContainer, GameEffectDuration, GameEffectEvent, GameEffectPeriod};
use crate::modifiers::ModAggregator;
use bevy::platform::collections::{HashMap, HashSet, hash_map};
use bevy::platform::hash::Hashed;
use bevy::prelude::*;
use bevy::utils::PreHashMap;
use std::any::{Any, TypeId};
use std::borrow::Borrow;
use std::collections::hash_map::Entry;
use std::time::Instant;

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
                        modifier.0.apply_base(&mut entity_mut).unwrap()
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
    println!("handle_apply_effect_events: {:?}", elapsed)
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
    println!("tick_active_effects: {:?}", elapsed)
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
                                    modifier.0.apply_base(&mut entity_mut).unwrap()
                                }
                            }
                        }
                        _ => {}
                    }
                }
            }
        });

    let elapsed = start.elapsed();
    println!("update_attribute_base_value: {:?}", elapsed)
}

pub fn update_attribute_current_value(
    mut query: Query<(AttributeEntityMut, &GameEffectContainer)>,
) {
    let start = Instant::now();
    for (mut entity_mut, container) in query.iter_mut() {
        let mut modifier_list = Vec::new();
        let mut attribute_list = PreHashMap::default();

        for effect in container.effects.iter() {
            if effect.periodic_application.is_none() {
                modifier_list.extend(&effect.modifiers);

                for modifier in &effect.modifiers {
                    attribute_list.insert(modifier.0.evaluator_id(), modifier.clone());
                }
            }
        }

        for (type_id, eval) in attribute_list {
            let aggregator: ModAggregator = modifier_list
                .iter()
                .filter(|&item| type_id == item.0.evaluator_id())
                .map(|&item| item.0.aggregator())
                .sum();

            let _ = eval.0.commit(&mut entity_mut, aggregator);
            //println!("{:?}", aggregator);

            // commands.trigger_targets(CurrentValueUpdateTrigger, entity_mut.id());
        }
    }
    let elapsed = start.elapsed();
    println!("update_attribute_current_value: {:?}", elapsed)
}

pub fn tick_ability_cooldowns(mut query: Query<&mut Transform>, time: Res<Time>) {
    for mut gac in &mut query {
        /*for (_, ability) in gac.get_abilities_mut().iter_mut() {
            ability.cooldown.write().unwrap().tick(time.delta());
        }*/
    }
}
