use crate::actors::Actor;
use crate::attributes::{AccessAttribute, Attribute, AttributeQueryData};
use crate::effect::Stacks;
use crate::graph::{NodeType, QueryGraphAdapter};
use crate::inspector::pretty_type_name;
use crate::modifier::Who;
use crate::prelude::{
    AppliedEffects, AttributeCalculator, AttributeModifier, EffectSource, EffectSources,
    EffectStatusParam, EffectTarget, EffectTicker,
};
use crate::{
    ApplyModifier, AttributesMut, AttributesRef, Dirty, OnAttributeValueChanged, OnBaseValueChange,
};
use bevy::ecs::component::Mutable;
use bevy::prelude::*;
use petgraph::visit::{Control, DfsEvent, IntoNeighbors, depth_first_search};
use std::any::type_name;
use std::collections::{HashMap, HashSet};
use std::marker::PhantomData;
/*
#[derive(Event)]
pub struct NotifyDirtyModifier<T: Attribute>(PhantomData<T>);

impl<T: Attribute> Default for NotifyDirtyModifier<T> {
    fn default() -> Self {
        Self(PhantomData)
    }
}

/// If a modifier is dirty, we notify the effect
pub fn flag_dirty_modifier<T: Attribute>(
    changed: Query<Entity, Changed<AttributeModifier<T>>>,
    mut command: Commands,
) {
    for entity in changed.iter() {
        /*println!(
            "{}: AttributeModifier<{}> is dirty.",
            pretty_type_name::<T>(),
            entity
        );*/
        command
            .entity(entity)
            .trigger(NotifyDirtyModifier::<T>::default());
    }
}

pub fn observe_dirty_modifier_notifications<T: Attribute>(
    trigger: Trigger<NotifyDirtyModifier<T>>,
    parent_effects: Query<&ModifierOf>,
    mut commands: Commands,
) {
    //println!("{}: Dirty modifier inserted.", trigger.target());
    commands
        .entity(trigger.target())
        .try_insert(Dirty::<T>::default());

    match parent_effects.get(trigger.target()) {
        Ok(parent) => {
            commands
                .entity(parent.0)
                .trigger(NotifyDirtyEffect::<T>::default());
        }
        Err(err) => {
            error!("{}: Error getting parent effect: {}", trigger.target(), err);
        }
    }
}

pub fn update_changed_attributes<T: Attribute>(
    mut query: Query<AttributeQueryData<T>, Changed<T>>,
    mut commands: Commands,
) {
    for mut attribute in query.iter_mut() {
        let should_notify_update = attribute.update_attribute_from_cache();
        if should_notify_update {
            commands
                .entity(attribute.entity)
                .trigger(OnAttributeValueChanged::<T>::default());
        }
    }
}

#[derive(Event)]
#[event(traversal = &'static EffectTarget, auto_propagate)]
pub struct NotifyDirtyEffect<T: Attribute>(PhantomData<T>);

impl<T: Attribute> Default for NotifyDirtyEffect<T> {
    fn default() -> Self {
        Self(PhantomData)
    }
}

pub fn observe_dirty_effect_notifications<T: Attribute>(
    trigger: Trigger<NotifyDirtyEffect<T>>,
    mut commands: Commands,
) {
    //println!("{}: Dirty modifier inserted on effect.", trigger.target());
    commands
        .entity(trigger.target())
        .try_insert(Dirty::<T>::default());
}
*/

pub fn attribute_changed_system<T: Attribute>(
    query: Query<(&T, &EffectSources), Changed<T>>,
    mut commands: Commands,
) {
    for (attribute, watchers) in query.iter() {
        let notify_targets = watchers.iter().collect::<Vec<Entity>>();
        commands.trigger_targets(
            NotifyAttributeChanged::<T> {
                base_value: attribute.base_value(),
                current_value: attribute.current_value(),
                phantom_data: Default::default(),
            },
            notify_targets,
        );
    }
}

#[derive(Event)]
pub struct NotifyAttributeChanged<T: Attribute> {
    base_value: f64,
    current_value: f64,
    phantom_data: PhantomData<T>,
}

pub fn attribute_changed_observer<S: Attribute, T: Attribute>(
    trigger: Trigger<NotifyAttributeChanged<S>>,
    mut attribute_modifiers_query: Query<&mut AttributeModifier<T>>,
) {
    /*println!(
        "Target: {} | Observer: {} | Type: {}",
        trigger.target(),
        trigger.observer(),
        type_name::<T>()
    );*/
    let mut modifier = attribute_modifiers_query
        .get_mut(trigger.observer())
        .unwrap();
    let mod_value = modifier.modifier.value_mut();
    *mod_value = trigger.current_value;
}

/// Navigates the tree descendants to update the tree attribute values
/// Effects that have a periodic timer application must be ignored in the current value calculations
pub fn update_effect_system<T: Attribute>(
    graph: QueryGraphAdapter,
    actors: Query<Entity, With<Actor>>,
    nodes: Query<&NodeType>,
    mut attributes: Query<AttributeQueryData<T>>,
    attribute_modifiers_query: Query<&AttributeModifier<T>>,
    mut commands: Commands,
) {
    debug_once!("Ready: update_effect_tree_system::{}", type_name::<T>());
    for actor_entity in actors.iter() {
        traverse_effect_tree::<T>(
            &graph,
            &nodes,
            actor_entity,
            1.0,
            &mut attributes,
            &attribute_modifiers_query,
            &mut commands,
        );
    }
}

fn traverse_effect_tree<T: Attribute>(
    graph: &QueryGraphAdapter,
    nodes: &Query<&NodeType>,
    current_entity: Entity,
    effect_intensity: f64,
    attributes: &mut Query<AttributeQueryData<T>>,
    attribute_modifiers_query: &Query<&AttributeModifier<T>>,
    commands: &mut Commands,
) -> AttributeCalculator {
    let Ok(node_type) = nodes.get(current_entity) else {
        error!("{}: Error getting node type.", current_entity);
        return AttributeCalculator::default();
    };

    let node_calculator = match node_type {
        NodeType::Actor => {
            // Traverse children with the new intensity
            let calculator = graph
                .neighbors(current_entity)
                .map(|entity| {
                    traverse_effect_tree::<T>(
                        graph,
                        nodes,
                        entity,
                        effect_intensity,
                        attributes,
                        attribute_modifiers_query,
                        commands,
                    )
                })
                .fold(AttributeCalculator::default(), |acc, child| {
                    acc.combine(child)
                });
            calculator
        }
        NodeType::Effect => {
            // Traverse children with the new intensity
            let calculator = graph
                .neighbors(current_entity)
                .map(|entity| {
                    traverse_effect_tree::<T>(
                        graph,
                        nodes,
                        entity,
                        effect_intensity,
                        attributes,
                        attribute_modifiers_query,
                        commands,
                    )
                })
                .fold(AttributeCalculator::default(), |acc, child| {
                    acc.combine(child)
                });
            calculator
        }
        NodeType::Modifier => {
            if let Ok(modifier) = attribute_modifiers_query.get(current_entity) {
                AttributeCalculator::from(modifier.modifier)
            } else {
                AttributeCalculator::default()
            }
        }
    };
    
    if let Ok(mut attribute) = attributes.get_mut(current_entity) {
        attribute.calculator_cache.calculator = node_calculator;
        let should_notify_observers = attribute.update_attribute(&node_calculator);
        if should_notify_observers {
            commands.trigger_targets(OnAttributeValueChanged::<T>::default(), current_entity);
        }
    };

    // Cleans the node
    commands.entity(current_entity).try_remove::<Dirty<T>>();

    node_calculator
}

/*
fn update_effect_tree<T: Attribute>(
    graph: &QueryGraphAdapter,
    current_entity: Entity,
    child_effects: Query<&AppliedEffects>,
    statuses: Query<EffectStatusParam>,
    dirty_attribute: Query<&Dirty<T>>,
    attribute_modifiers_query: &Query<&AttributeModifier<T>>,
    attributes: &mut Query<AttributeQueryData<T>>,
    commands: &mut Commands,
) -> AttributeCalculator {
    // Ignore clean branches of the tree that have the updated cached value of the calculator
    if attributes.contains(current_entity) && !dirty_attribute.contains(current_entity) {
        return match attributes.get(current_entity) {
            Ok(attribute) => attribute.calculator_cache.calculator,
            Err(e) => {
                println!("{}: Error getting attribute: {}", current_entity, e);
                AttributeCalculator::default()
            }
        };
    };

    // Inactive effects or those with periodic application are ignored from the persistent calculations
    if let Ok(effect_status) = statuses.get(current_entity) {
        if effect_status.is_inactive() || effect_status.is_periodic() {
            return AttributeCalculator::default();
        }
    };

    // Accumulates the value of the modifiers on this node from all the attached modifiers
    let this_node_modifiers =
        collect_entity_modifiers(current_entity, &modifiers_query, &attribute_modifiers_query);
    let this_node_calculator = AttributeCalculator::from(&this_node_modifiers.collect::<Vec<_>>());

    let child_calculator = child_effects
        .get(current_entity)
        .map(|effects| {
            effects
                .iter()
                .fold(AttributeCalculator::default(), |acc, child| {
                    let child_calc = update_effect_tree::<T>(
                        child,
                        child_effects,
                        statuses,
                        dirty_attribute,
                        modifiers_query,
                        attribute_modifiers_query,
                        attributes,
                        commands,
                    );
                    acc.combine(child_calc)
                })
        })
        .unwrap_or_default();

    let combined_calculator = this_node_calculator.combine(child_calculator);
    //println!("{}: Combined: {:?}", current_entity, combined_calculator);

    // Update the attribute if it exists
    if let Ok(mut attribute) = attributes.get_mut(current_entity) {
        attribute.calculator_cache.calculator = combined_calculator.clone();
        let should_notify_observers = attribute.update_attribute(&combined_calculator);
        if should_notify_observers {
            commands.trigger_targets(OnAttributeValueChanged::<T>::default(), current_entity);
        }
    };

    // Cleans the node
    commands.entity(current_entity).try_remove::<Dirty<T>>();
    //println!("{}: Node visited and cleaned.", current_entity);

    // Return the value of the modifier so far so we can update the current values
    combined_calculator
}
*/
pub fn apply_periodic_effect<T: Component<Mutability = Mutable> + Attribute>(
    effects: Query<(
        &EffectTicker,
        // Add node types
        &AppliedEffects,
        &Stacks,
        &EffectTarget,
        &EffectSource,
    )>,
    modifiers: Query<&AttributeModifier<T>>,
    mut commands: Commands,
) {
    for (timer, effect_modifiers, stacks, target, source) in effects.iter() {
        if !timer.just_finished() {
            continue;
        }

        // Timer has triggered. Grab modifiers and apply them.
        for children in effect_modifiers.iter() {
            let Ok(attribute_modifier) = modifiers.get(children) else {
                continue;
            };

            // Apply the stack count to the modifier
            let scaled_modifier = attribute_modifier.modifier.clone() * stacks.0 as f64;

            match attribute_modifier.who {
                Who::Target => {
                    commands.trigger_targets(
                        ApplyModifier::<T> {
                            phantom_data: Default::default(),
                            modifier: scaled_modifier,
                        },
                        target.0,
                    );
                }
                Who::Source => {
                    commands.trigger_targets(
                        ApplyModifier::<T> {
                            phantom_data: Default::default(),
                            modifier: scaled_modifier,
                        },
                        source.0,
                    );
                }
                Who::Owner => {
                    todo!()
                }
            }
        }
    }
}

pub fn apply_modifier_on_trigger<T: Component<Mutability = Mutable> + Attribute>(
    trigger: Trigger<ApplyModifier<T>>,
    mut query: Query<&mut T>,
    mut commands: Commands,
) {
    let Ok(mut attribute) = query.get_mut(trigger.target()) else {
        return;
    };
    let old_value = attribute.base_value();
    let calculator = AttributeCalculator::from(trigger.modifier);
    let new_val = calculator.eval(attribute.base_value());

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
