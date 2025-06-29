use crate::attributes::AttributeComponent;
use crate::effects::{Effect, EffectDuration, EffectOf, EffectPeriodicTimer, Effects};
use crate::modifiers::{ModAggregator, Modifier};
use crate::{Actor, Dirty, OnAttributeValueChanged, OnModifierApplied};
use bevy::ecs::component::Mutable;
use bevy::ecs::relationship::Relationship;
use bevy::prelude::*;
use ptree::{TreeBuilder, print_tree};
use std::any::type_name;
use std::ops::DerefMut;
use crate::abilities::AbilityCooldown;

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

pub fn tick_ability_cooldown(mut query: Query<&mut AbilityCooldown>, time: Res<Time>) {
    query.par_iter_mut().for_each(|mut cooldown| {
        cooldown.0.tick(time.delta());
    });   
}

pub fn flag_dirty_modifier<T: Component>(
    changed: Query<Entity, Changed<Modifier<T>>>,
    parents: Query<&EffectOf>,
    dirty: Query<&Dirty<T>>,
    mut command: Commands,
) {
    'outer: for changed in changed.iter() {
        command.entity(changed).try_insert(Dirty::<T>::default());
        for entity in parents.iter_ancestors(changed) {
            // Stop marking as the rest of the hierarchy is already dirty
            if dirty.contains(entity) {
                continue 'outer;
            }
            command.entity(entity).try_insert(Dirty::<T>::default());
        }
    }
}

pub fn flag_dirty_attribute<T: Component>(
    changed: Query<Entity, Changed<T>>,
    parents: Query<&EffectOf>,
    dirty: Query<&Dirty<T>>,
    mut command: Commands,
) {
    'outer: for changed in changed.iter() {
        command.entity(changed).try_insert(Dirty::<T>::default());
        for entity in parents.iter_ancestors(changed) {
            // Stop marking as the rest of the hierarchy is already dirty
            if dirty.contains(entity) {
                continue 'outer;
            }
            command.entity(entity).try_insert(Dirty::<T>::default());
        }
    }
}

/// Navigates the tree descendants to update the tree attribute values
/// Effects that have a periodic timer application must be ignored in the current value calculations
pub fn update_effect_tree_system<T: Component<Mutability = Mutable> + AttributeComponent>(
    actors: Query<Entity, (With<Actor>, With<Dirty<T>>)>,
    descendants: Query<&Effects, Without<EffectPeriodicTimer>>,
    modifiers: Query<&Modifier<T>>,
    dirty: Query<&Dirty<T>>,
    periodic_effects: Query<&Effect, With<EffectPeriodicTimer>>,
    mut attributes: Query<&mut T>,
    mut aggregators: Query<&mut ModAggregator<T>>,
    mut commands: Commands,
) {
    debug_once!("Ready: update_effect_tree_system::{}", type_name::<T>());
    for actor_entity in actors.iter() {
        let mut visited_nodes = 0;
        update_effect_tree(
            &mut visited_nodes,
            actor_entity,
            actor_entity,
            descendants,
            modifiers,
            dirty,
            periodic_effects,
            &mut attributes,
            &mut aggregators,
            &mut commands,
        );
    }
}

fn update_effect_tree<T: Component<Mutability = Mutable> + AttributeComponent>(
    visited_nodes: &mut usize,
    actor_entity: Entity,
    current_entity: Entity,
    descendants: Query<&Effects, Without<EffectPeriodicTimer>>,
    modifiers: Query<&Modifier<T>>,
    dirty: Query<&Dirty<T>>,
    periodic_effects: Query<&Effect, With<EffectPeriodicTimer>>,
    mut attributes: &mut Query<&mut T>,
    mut aggregators: &mut Query<&mut ModAggregator<T>>,
    mut commands: &mut Commands,
) -> ModAggregator<T> {
    // If a node doesn't have a modifier, consider it the default one
    let binding = Modifier::<T>::default();
    let modifier = modifiers.get(current_entity).unwrap_or(&binding);
    // Ignore clean branches of the tree
    if !dirty.contains(current_entity) {
        return match aggregators.get(current_entity) {
            Ok(aggregator) => aggregator.clone(),
            Err(_) => ModAggregator::default(),
        };
    };
    // Return value of all the child nodes plus our own
    // Ignore modifier_so_far when they have a periodic timer
    let modifier_so_far = modifier.value.clone()
        + match descendants.get(current_entity) {
            Ok(childrens) => childrens
                .iter()
                .map(|child| {
                    update_effect_tree::<T>(
                        visited_nodes,
                        actor_entity,
                        child,
                        descendants,
                        modifiers,
                        dirty,
                        periodic_effects,
                        &mut attributes,
                        &mut aggregators,
                        &mut commands,
                    )
                })
                .sum(),
            Err(_) => ModAggregator::default(), // No childrens
        };
    // Update the aggregator with the most-recent value
    let Ok(mut aggregator) = aggregators.get_mut(current_entity) else {
        debug!(
            "Could not get an aggregator on {}, this is abnormal.",
            current_entity
        );
        return ModAggregator::default();
    };
    *aggregator = modifier_so_far.clone();
    // Edit the current attribute if the node has one
    if let Ok(mut attribute) = attributes.get_mut(current_entity) {
        let new_val = aggregator.evaluate(attribute.base_value());
        
        if (new_val - &attribute.current_value()).abs() > f64::EPSILON {
            commands.trigger_targets(OnAttributeValueChanged::<T>::default(), actor_entity);
            attribute.set_current_value(new_val);
        }
    };
    // Cleans the node
    commands.entity(current_entity).remove::<Dirty<T>>();
    *visited_nodes += 1;
    // Return the value of the modifier so far so we can update the current values
    modifier_so_far
}

pub fn pretty_print_tree_system(
    actors: Query<Entity, With<Actor>>,
    descendants: Query<&Effects>,
    entities: Query<&Name>,
) {
    let mut builder = TreeBuilder::new("Actor-Attribute Tree".into());
    for actor in actors.iter() {
        recursive_pretty_print(actor, &mut builder, descendants, entities);
    }
    let tree = builder.build();
    let _ = print_tree(&tree);
}

pub fn recursive_pretty_print(
    current_entity: Entity,
    builder: &mut TreeBuilder,
    descendants: Query<&Effects>,
    entities: Query<&Name>,
) {
    let Ok(name) = entities.get(current_entity) else {
        return;
    };
    let tree_item = format!("{name}[{}] ", current_entity);
    // Iterate recursively on all the childrens
    if let Ok(childrens) = descendants.get(current_entity) {
        builder.begin_child(tree_item);
        for child in childrens.iter() {
            recursive_pretty_print(child, builder, descendants, entities);
        }
        builder.end_child();
    } else {
        builder.add_empty_child(tree_item);
    }
}

/// Confirms that an effect is instant when triggered.
/// Navigates the tree ancestors applying the modifier to any Attribute<T> it encounters.
/// Marks the branch as dirty.
/// Consider a 'do not propagate' flag
pub fn trigger_instant_effect_applied<T: Component<Mutability = Mutable> + AttributeComponent>(
    trigger: Trigger<OnAdd, Modifier<T>>,
    effects: Query<(Entity, &Effect), Without<EffectDuration>>,
    modifiers: Query<&Modifier<T>>,
    parents: Query<&EffectOf>,
    mut commands: Commands,
) {
    let modifier_entity = trigger.target();
    let Ok(parent) = parents.get(modifier_entity) else {
        return;
    };
    // If we can't get the effect, it has a duration. Therefore, we do not want to apply it.
    if !effects.contains(parent.get()) {
        return;
    };
    let Ok(modifier) = modifiers.get(modifier_entity) else {
        return;
    };
    commands.trigger_targets(
        OnModifierApplied::<T> {
            phantom_data: Default::default(),
            value: modifier.value.clone(),
        },
        trigger.target(),
    );
    commands.trigger_targets(OnAttributeValueChanged::<T>::default(), trigger.target());
}

/// Despawn instant effects right after they were added as we cannot remove them in the triggers directly
pub fn despawn_instant_effect(
    effects: Query<Entity, (Added<Effect>, Without<EffectDuration>)>,
    mut commands: Commands,
) {
    for effect in effects.iter() {
        commands.entity(effect).try_despawn();
    }
}

pub fn trigger_periodic_effect<T: Component<Mutability = Mutable> + AttributeComponent>(
    effects: Query<(&EffectPeriodicTimer, &Effects), With<EffectDuration>>,
    modifiers: Query<&Modifier<T>>,
    mut commands: Commands,
) {
    for (timer, effect_childrens) in effects.iter() {
        if !timer.0.just_finished() {
            continue;
        }
        // Timer has triggered. Grab modifiers and apply them.
        for children in effect_childrens.iter() {
            let Ok(modifier) = modifiers.get(children) else {
                continue;
            };
            commands.trigger_targets(
                OnModifierApplied::<T> {
                    phantom_data: Default::default(),
                    value: modifier.value.clone(),
                },
                children,
            );
            commands.trigger_targets(OnAttributeValueChanged::<T>::default(), children);
        }
    }
}
