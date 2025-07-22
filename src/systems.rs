use crate::abilities::AbilityCooldown;
use crate::attributes::Attribute;
use crate::effects::{
    EffectDuration, EffectInactive, EffectPeriodicTimer, EffectSource, EffectTarget,
    EffectTargetedBy,
};
use crate::modifiers::{
    AttributeModifier, ModAggregator, ModTarget, Modifiers, aggregate_entity_modifiers,
};
use crate::stacks::Stacks;
use crate::{Actor, ApplyModifier, Dirty, OnAttributeValueChanged, OnBaseValueChange};
use bevy::ecs::component::{Mutable};
use bevy::prelude::*;
use std::any::type_name;

pub(crate) fn tick_effects_periodic_timer(
    mut query: Query<&mut EffectPeriodicTimer, Without<EffectInactive>>,
    time: Res<Time>,
) {
    query.par_iter_mut().for_each(|mut timer| {
        timer.0.tick(time.delta());
    });
}

/// Updates the duration timers for all active effects.
///
/// The function iterates over all entities that have an `EffectDuration` component,
/// excluding those with an `EffectInactive` component, and progresses their timers.
/// This is done in parallel for performance optimization.
pub(crate) fn tick_effect_duration_timers(
    mut query: Query<&mut EffectDuration, Without<EffectInactive>>,
    time: Res<Time>,
) {
    query.par_iter_mut().for_each(|mut effect_duration| {
        effect_duration.0.tick(time.delta());
    });
}

pub(crate) fn tick_ability_cooldown(mut query: Query<&mut AbilityCooldown>, time: Res<Time>) {
    query.par_iter_mut().for_each(|mut cooldown| {
        cooldown.0.tick(time.delta());
    });
}

pub fn flag_dirty_modifier<T: Attribute>(
    changed: Query<Entity, Changed<AttributeModifier<T>>>,
    parents: Query<&EffectTarget>,
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

pub fn flag_dirty_attribute<T: Attribute>(
    changed: Query<Entity, Changed<T>>,
    parents: Query<&EffectTarget>,
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
pub fn update_effect_tree_system<T: Component<Mutability = Mutable> + Attribute>(
    actors: Query<Entity, With<Actor>>,
    descendants: Query<&EffectTargetedBy>,
    mut is_periodic: Query<&EffectPeriodicTimer>,
    mut is_inactive: Query<&EffectInactive>,
    dirty: Query<&Dirty<T>>,
    mut modifiers_query: Query<&Modifiers>,
    mut attributes: Query<&mut T>,
    mut modifiers: Query<&AttributeModifier<T>>,
    mut aggregators: Query<&mut ModAggregator<T>>,
    mut commands: Commands,
) {
    debug_once!("Ready: update_effect_tree_system::{}", type_name::<T>());
    for actor_entity in actors.iter() {
        let mut visited_nodes = 0;
        update_effect_tree(
            &mut visited_nodes,
            actor_entity,
            descendants,
            &mut is_periodic,
            &mut is_inactive,
            dirty,
            &mut modifiers_query,
            &mut attributes,
            &mut modifiers,
            &mut aggregators,
            &mut commands,
        );
    }
}

fn update_effect_tree<T: Component<Mutability = Mutable> + Attribute>(
    visited_nodes: &mut usize,
    current_entity: Entity,
    descendants: Query<&EffectTargetedBy>,
    mut is_periodic: &mut Query<&EffectPeriodicTimer>,
    mut is_inactive: &mut Query<&EffectInactive>,
    dirty: Query<&Dirty<T>>,
    mut modifiers_query: &mut Query<&Modifiers>,
    mut attribute_query: &mut Query<&mut T>,
    mut attribute_modifier_query: &mut Query<&AttributeModifier<T>>,
    mut aggregator_query: &mut Query<&mut ModAggregator<T>>,
    mut commands: &mut Commands,
) -> ModAggregator<T> {
    // Ignore clean branches of the tree
    /* if !dirty.contains(current_entity) {
        return match aggregators.get(current_entity) {
            Ok(aggregator) => aggregator.clone(),
            Err(_) => ModAggregator::default(),
        };
    };*/

    if is_inactive.contains(current_entity) | is_periodic.contains(current_entity) {
        return ModAggregator::<T>::default();
    };

    // Accumulates the value of the modifiers on this node from all the attached modifiers
    let this_node_mod_aggregator =
        aggregate_entity_modifiers(current_entity, modifiers_query, attribute_modifier_query);

    let modifier_so_far = this_node_mod_aggregator
        + match descendants.get(current_entity) {
            Ok(child_effects) => child_effects
                .iter()
                .map(|child| {
                    update_effect_tree::<T>(
                        visited_nodes,
                        child,
                        descendants,
                        &mut is_periodic,
                        &mut is_inactive,
                        dirty,
                        &mut modifiers_query,
                        &mut attribute_query,
                        &mut attribute_modifier_query,
                        &mut aggregator_query,
                        &mut commands,
                    )
                })
                .sum(),
            Err(_) => ModAggregator::default(), // No childrens
        };
    // Update the aggregator with the most-recent value
    let Ok(mut aggregator) = aggregator_query.get_mut(current_entity) else {
        return ModAggregator::default();
    };

    // We save the aggregator's updated value
    *aggregator = modifier_so_far.clone();

    // Edit the current attribute if the node has one
    if let Ok(mut attribute) = attribute_query.get_mut(current_entity) {
        let new_val = aggregator.evaluate(attribute.base_value());

        if (new_val - &attribute.current_value()).abs() > f64::EPSILON {
            commands.trigger_targets(OnAttributeValueChanged::<T>::default(), current_entity);
            attribute.set_current_value(new_val);
        }
    };
    // Cleans the node
    commands.entity(current_entity).remove::<Dirty<T>>();
    *visited_nodes += 1;
    // Return the value of the modifier so far so we can update the current values
    modifier_so_far
}

/*
pub fn pretty_print_tree_system(
    components: &Components,
    actors: Query<Entity, With<Actor>>,
    descendants: Query<&EffectTargetedBy>,
    modifiers: Query<&Modifiers>,
    entity_refs: Query<ModifiersQuery>,
    type_registry: Res<AppTypeRegistry>,
    stacks: Query<&Stacks>,
    entities: Query<&Name>,
) {
    let mut builder = TreeBuilder::new("Actor-Attribute Tree".into());

    for actor in actors.iter() {
        recursive_pretty_print(
            components,
            actor,
            &mut builder,
            descendants,
            modifiers,
            entity_refs,
            &type_registry,
            stacks,
            entities,
        );
    }
    let tree = builder.build();
    let _ = print_tree(&tree);
}

pub fn recursive_pretty_print(
    components: &Components,
    current_entity: Entity,
    builder: &mut TreeBuilder,
    descendants: Query<&EffectTargetedBy>,
    modifiers: Query<&Modifiers>,
    entity_refs: Query<ModifiersQuery>,
    type_registry: &Res<AppTypeRegistry>,
    stacks: Query<&Stacks>,
    entities: Query<&Name>,
) {
    let Ok(name) = entities.get(current_entity) else {
        return;
    };
    let binding = Stacks::default();
    let stack = stacks.get(current_entity).unwrap_or(&binding);
    let tree_item = format!("[{current_entity}] {name} [{}]", stack.0);

    // Iterate over owned modifiers
    for &modifiers in modifiers.get(current_entity).iter() {
        for modifier in modifiers.iter() {
            let Ok(modifier_ref) = entity_refs.get(modifier) else {
                continue;
            };
            modifier_ref.print_modifiers(components, &type_registry)
        }
    }

    // Iterate recursively on all the sub effects
    if let Ok(childrens) = descendants.get(current_entity) {
        builder.begin_child(tree_item);
        for child in childrens.iter() {
            recursive_pretty_print(
                components,
                child,
                builder,
                descendants,
                modifiers,
                entity_refs,
                type_registry,
                stacks,
                entities,
            );
        }
        builder.end_child();
    } else {
        builder.add_empty_child(tree_item);
    }
}
*/

pub fn apply_periodic_effect<T: Component<Mutability = Mutable> + Attribute>(
    effects: Query<(
        &EffectPeriodicTimer,
        &Modifiers,
        &Stacks,
        &EffectTarget,
        &EffectSource,
    )>,
    modifiers: Query<&AttributeModifier<T>>,
    mut commands: Commands,
) {
    for (timer, effect_modifiers, stacks, target, source) in effects.iter() {
        if !timer.0.just_finished() {
            continue;
        }

        // Timer has triggered. Grab modifiers and apply them.
        for children in effect_modifiers.iter() {
            let Ok(modifier) = modifiers.get(children) else {
                continue;
            };

            // Apply the stack count to the modifier
            let mutator = modifier.aggregator.clone() * stacks.0 as f64;

            match modifier.application {
                ModTarget::Target => {
                    commands.trigger_targets(
                        ApplyModifier::<T> {
                            phantom_data: Default::default(),
                            value: mutator,
                        },
                        target.0,
                    );
                }
                ModTarget::Source => {
                    commands.trigger_targets(
                        ApplyModifier::<T> {
                            phantom_data: Default::default(),
                            value: mutator,
                        },
                        source.0,
                    );
                }
            }
        }
    }
}

pub(crate) fn apply_modifier_on_trigger<T: Component<Mutability = Mutable> + Attribute>(
    trigger: Trigger<ApplyModifier<T>>,
    mut query: Query<&mut T>,
    mut commands: Commands,
) {
    let Ok(mut attribute) = query.get_mut(trigger.target()) else {
        return;
    };
    let old_value = attribute.base_value();
    let new_val = trigger.value.evaluate(attribute.base_value());

    if (old_value - new_val).abs() > f64::EPSILON {
        commands.trigger_targets(
            OnBaseValueChange::<T> {
                phantom_data: Default::default(),
                old: old_value,
                new: new_val,
                entity: trigger.target(),
            },
            trigger.target(),
        );
        attribute.set_base_value(new_val);
    }
}
