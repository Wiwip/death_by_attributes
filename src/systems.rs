use crate::actors::Actor;
use crate::attributes::{Attribute, AttributeQueryData, AttributeQueryDataReadOnly};
use crate::effect::Stacks;
use crate::graph::{NodeType, QueryGraphAdapter};
use crate::modifier::Who;
use crate::prelude::*;
use crate::{AttributesRef, Dirty, OnAttributeValueChanged};
use bevy::prelude::*;
use num_traits::AsPrimitive;
use petgraph::visit::IntoNeighbors;
use std::any::type_name;
use std::marker::PhantomData;

#[derive(EntityEvent)]
#[entity_event(propagate=&'static EffectTarget)]
pub struct MarkNodeDirty<T: Attribute> {
    pub entity: Entity,
    pub phantom_data: PhantomData<T>,
}

/// Observes [`MarkNodeDirty`] triggers for specific attributes and propagates
/// the event upward through using the [`EffectTarget`] chain.
///
/// Stops when it encounters a dirty node indicating that all later nodes are already dirty.
pub fn observe_dirty_node_notifications<T: Attribute>(
    mut trigger: On<MarkNodeDirty<T>>,
    dirty_nodes: Query<&Dirty<T>>,
    mut commands: Commands,
) {
    if dirty_nodes.contains(trigger.target()) {
        trigger.propagate(false);
        return;
    }
    commands
        .entity(trigger.target())
        .try_insert(Dirty::<T>::default());
}

#[derive(EntityEvent)]
pub struct NotifyAttributeDependencyChanged<T: Attribute> {
    pub entity: Entity,
    pub base_value: T::Property,
    pub current_value: T::Property,
    pub phantom_data: PhantomData<T>,
}

pub fn on_change_attribute_observer<S: Attribute, T: Attribute>(
    trigger: On<NotifyAttributeDependencyChanged<S>>,
    mut attribute_modifiers_query: Query<(Entity, &mut AttributeModifier<T>)>,
    mut commands: Commands,
) where
    S::Property: AsPrimitive<T::Property>,
{
    let (mod_entity, mut modifier) = attribute_modifiers_query
        .get_mut(trigger.observer())
        .unwrap();

    let scaling = modifier.scaling;

    let converted_source_attribute = trigger.current_value.as_();
    let scaled_converted_source_attribute = converted_source_attribute.as_() * scaling;

    modifier.scaling = scaled_converted_source_attribute;

    commands.trigger(MarkNodeDirty::<T> {
        entity: mod_entity,
        phantom_data: Default::default(),
    });

    /*debug!(
        "{} <{},{}>: Attribute changed. New value: {} ",
        trigger.observer(),
        pretty_type_name::<S>(),
        pretty_type_name::<T>(),
        trigger.current_value,
    );*/
}

/// Navigates the tree descendants to update the tree attribute values
/// Effects that have a periodic timer application must be ignored in the current value calculations
pub fn update_effect_system<T: Attribute>(
    graph: QueryGraphAdapter,
    actors: Query<Entity, With<Actor>>,
    nodes: Query<&NodeType>,
    dirty_nodes: Query<&Dirty<T>>,
    statuses: Query<EffectStatusParam>,
    attributes: Query<AttributeQueryDataReadOnly<T>>,
    attribute_refs: Query<AttributesRef>,
    modifiers: Query<&AttributeModifier<T>>,
    mut commands: Commands,
) {
    debug_once!("Ready: update_effect_tree_system::{}", type_name::<T>());
    for actor_entity in actors.iter() {
        // Ignore clean actors
        if !dirty_nodes.contains(actor_entity) {
            continue;
        }

        let Ok(attribute_ref) = attribute_refs.get(actor_entity) else {
            error!("{}: Error getting attribute ref.", actor_entity);
            continue;
        };

        update_effect_tree_attributes::<T>(
            &graph,
            &nodes,
            actor_entity,
            &dirty_nodes,
            &statuses,
            &attributes,
            &attribute_ref,
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
    attributes: &Query<AttributeQueryDataReadOnly<T>>,
    actor_ref: &AttributesRef,
    modifiers: &Query<&AttributeModifier<T>>,
    commands: &mut Commands,
) -> AttributeCalculator<T> {
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
        match attributes.get(current_entity) {
            Ok(attribute) => {
                return attribute.calculator_cache.calculator;
            }
            _ => {} // Continue traversing the tree.
        }
    }

    let node_calculator = match node_type {
        NodeType::Actor | NodeType::Effect => {
            // Traverse children
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
                        actor_ref,
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
                AttributeCalculator::convert(modifier, &actor_ref)
            } else {
                // This happens when we are looking for component A, but the modifier applies to component B
                AttributeCalculator::default()
            }
        }
    };

    // Signal to update the attribute
    commands
        .trigger(UpdateAttributeSignal {
            entity: current_entity,
            calculator: node_calculator,
        });

    // Cleans the node
    commands.entity(current_entity).try_remove::<Dirty<T>>();

    node_calculator
}

#[derive(EntityEvent)]
pub struct UpdateAttributeSignal<T: Attribute> {
    entity: Entity,
    calculator: AttributeCalculator<T>,
}

pub fn update_attribute<T: Attribute>(
    trigger: On<UpdateAttributeSignal<T>>,
    mut attributes: Query<AttributeQueryData<T>>,
    mut commands: Commands,
) {
    if let Ok(mut attribute) = attributes.get_mut(trigger.target()) {
        attribute.calculator_cache.calculator = trigger.event().calculator;
        let should_notify_observers = attribute.update_attribute(&trigger.event().calculator);
        if should_notify_observers {
            commands.trigger(OnAttributeValueChanged::<T> {
                entity: trigger.target(),
                _marker: Default::default(),
            });
        }
    };
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
    mut event_writer: MessageWriter<ApplyAttributeModifierEvent<T>>,
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
            let stack_count = stacks.current_value();
            let scaled_modifier = attribute_modifier.clone(); // TODO * stack_count;

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
