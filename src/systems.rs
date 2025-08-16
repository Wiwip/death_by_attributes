use crate::actors::Actor;
use crate::attributes::{Attribute, AttributeQueryData};
use crate::effect::Stacks;
use crate::graph::{NodeType, QueryGraphAdapter};
use crate::inspector::pretty_type_name;
use crate::modifier::Who;
use crate::prelude::*;
use crate::{Dirty, OnAttributeValueChanged};
use bevy::prelude::*;
use petgraph::visit::IntoNeighbors;
use std::any::type_name;
use std::marker::PhantomData;

#[derive(Event)]
#[event(traversal = &'static EffectTarget, auto_propagate)]
pub struct NotifyDirtyNode<T: Attribute>(PhantomData<T>);

impl<T: Attribute> Default for NotifyDirtyNode<T> {
    fn default() -> Self {
        Self(PhantomData)
    }
}

pub fn observe_dirty_effect_notifications<T: Attribute>(
    mut trigger: Trigger<NotifyDirtyNode<T>>,
    dirty_nodes: Query<&Dirty<T>>,
    mut commands: Commands,
    time: Res<Time>,
) {
    if dirty_nodes.contains(trigger.target()) {
        trigger.propagate(false);
        return;
    }

    println!(
        "{:?}:{}: Dirty<{}>",
        time.elapsed(),
        trigger.target(),
        pretty_type_name::<T>()
    );
    commands
        .entity(trigger.target())
        .try_insert(Dirty::<T>::default());
}

#[derive(Event)]
pub struct NotifyAttributeChanged<T: Attribute> {
    pub base_value: f64,
    pub current_value: f64,
    pub phantom_data: PhantomData<T>,
}

pub fn on_change_attribute_observer<S: Attribute, T: Attribute>(
    trigger: Trigger<NotifyAttributeChanged<S>>,
    mut attribute_modifiers_query: Query<(Entity, &mut AttributeModifier<T>)>,
    mut commands: Commands,
) {
    let (mod_entity, mut modifier) = attribute_modifiers_query
        .get_mut(trigger.observer())
        .unwrap();
    let mod_value = modifier.modifier.value_mut();
    *mod_value = trigger.current_value;

    commands
        .entity(mod_entity)
        .trigger(NotifyDirtyNode::<T>::default());

    debug!(
        "{} <{},{}>: Attribute changed. New value: {} ",
        trigger.observer(),
        pretty_type_name::<S>(),
        pretty_type_name::<T>(),
        trigger.current_value,
    );
}

/// Navigates the tree descendants to update the tree attribute values
/// Effects that have a periodic timer application must be ignored in the current value calculations
pub fn update_effect_system<T: Attribute>(
    graph: QueryGraphAdapter,
    actors: Query<Entity, With<Actor>>,
    nodes: Query<&NodeType>,
    dirty_nodes: Query<&Dirty<T>>,
    statuses: Query<EffectStatusParam>,
    mut attributes: Query<AttributeQueryData<T>>,
    modifiers: Query<&AttributeModifier<T>>,
    mut commands: Commands,
) {
    debug_once!("Ready: update_effect_tree_system::{}", type_name::<T>());
    for actor_entity in actors.iter() {
        // Ignore clean actors
        if !dirty_nodes.contains(actor_entity) {
            continue;
        }

        update_effect_tree_attributes::<T>(
            &graph,
            &nodes,
            actor_entity,
            &dirty_nodes,
            &statuses,
            &mut attributes,
            &modifiers,
            &mut commands,
        );
    }
}

fn update_effect_tree_attributes<T: Attribute>(
    graph: &QueryGraphAdapter,
    nodes: &Query<&NodeType>,
    current_entity: Entity,
    dirty_nodes: &Query<&Dirty<T>>,
    statuses: &Query<EffectStatusParam>,
    attributes: &mut Query<AttributeQueryData<T>>,
    modifiers: &Query<&AttributeModifier<T>>,
    commands: &mut Commands,
) -> AttributeCalculator {
    let Ok(node_type) = nodes.get(current_entity) else {
        error!("{}: Error getting node type.", current_entity);
        return AttributeCalculator::default();
    };

    let Ok(status) = statuses.get(current_entity) else {
        return AttributeCalculator::default();
    };
    if status.is_periodic() || status.is_inactive() {
        return AttributeCalculator::default();
    }
    if !dirty_nodes.contains(current_entity) {
        println!(
            "{}: Early return on: {}",
            pretty_type_name::<T>(),
            current_entity
        );
        match attributes.get(current_entity) {
            Ok(attribute) => {
                return attribute.calculator_cache.calculator;
            }
            _ => {}
        }
    }

    let node_calculator = match node_type {
        NodeType::Actor | NodeType::Effect => {
            // Traverse children with the new intensity
            let calculator = graph
                .neighbors(current_entity)
                .map(|entity| {
                    update_effect_tree_attributes::<T>(
                        graph,
                        nodes,
                        entity,
                        dirty_nodes,
                        statuses,
                        attributes,
                        modifiers,
                        commands,
                    )
                })
                .fold(AttributeCalculator::default(), |acc, child| {
                    acc.combine(child)
                });
            calculator
        }
        NodeType::Modifier => {
            if let Ok(modifier) = modifiers.get(current_entity) {
                AttributeCalculator::from(modifier.modifier)
            } else {
                // This happens when we are looking for component A, but the modifier applies to component B
                AttributeCalculator::default()
            }
        }
    };

    // Update the attribute
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

pub fn apply_periodic_effect<T: Attribute>(
    effects: Query<(
        &EffectTicker,
        &AppliedEffects,
        &Stacks,
        &EffectTarget,
        &EffectSource,
    )>,
    modifiers: Query<&AttributeModifier<T>>,
    mut event_writer: EventWriter<ApplyAttributeModifierEvent>,
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
            let scaled_modifier = attribute_modifier.modifier.clone() * stacks.current_value();

            match attribute_modifier.who {
                Who::Target => {
                    event_writer.write(ApplyAttributeModifierEvent {
                        target: target.0,
                        modifier: scaled_modifier,
                        attribute: attribute_modifier.as_accessor(),
                    });
                }
                Who::Source => {
                    event_writer.write(ApplyAttributeModifierEvent {
                        target: source.0,
                        modifier: scaled_modifier,
                        attribute: attribute_modifier.as_accessor(),
                    });
                }
                Who::Effect => {
                    todo!()
                }
            }
        }
    }
}
