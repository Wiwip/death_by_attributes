use crate::actors::Actor;
use crate::assets::EffectDef;
use crate::attributes::{Attribute, AttributeQueryData, AttributeQueryDataReadOnly};
use crate::condition::GameplayContext;
use crate::effect::{
    Effect, EffectSource, EffectStatusParam, EffectTarget, EffectTargeting, EffectTicker, Stacks,
};
use crate::graph::{DependencyGraph, NodeType};
use crate::inspector::pretty_type_name;
use crate::modifier::{ApplyAttributeModifierMessage, AttributeCalculator, OwnedModifiers, Who};
use crate::prelude::*;
use crate::{AttributesRef, CurrentValueChanged, Dirty};
use bevy::prelude::*;
use petgraph::visit::IntoNeighbors;
use std::any::type_name;
use std::marker::PhantomData;

#[derive(EntityEvent)]
#[entity_event(propagate=&'static EffectTarget, auto_propagate)]
pub struct MarkNodeDirty<T: Attribute> {
    pub entity: Entity,
    pub phantom_data: PhantomData<T>,
}

/// Observes [`MarkNodeDirty`] triggers for specific attributes and propagates
/// the event upward through using the [`EffectTarget`] chain.
///
/// Stops when it encounters a dirty node indicating that all later nodes are already dirty.
pub fn mark_node_dirty_observer<T: Attribute>(
    mut trigger: On<MarkNodeDirty<T>>,
    dirty_nodes: Query<&Dirty<T>>,
    mut commands: Commands,
) {
    if dirty_nodes.contains(trigger.entity) {
        trigger.propagate(false);
        return;
    }
    commands
        .entity(trigger.entity)
        .try_insert(Dirty::<T>::default());
}

/// Navigates the tree descendants to update the tree attribute values
/// Effects that have a periodic timer application must be ignored in the current value calculations
pub fn update_effect_system<T: Attribute>(
    graph: DependencyGraph,
    nodes: Query<&NodeType>,
    actors: Query<Entity, With<Actor>>,
    dirty_nodes: Query<&Dirty<T>>,
    statuses: Query<EffectStatusParam>,
    attributes: Query<AttributeQueryDataReadOnly<T>>,
    attribute_refs: Query<AttributesRef>,
    effects: Query<(&OwnedModifiers, &EffectSource, &EffectTarget)>,
    modifiers: Query<&AttributeModifier<T>>,
    mut commands: Commands,
) {
    debug_once!("Ready: update_effect_tree_system::{}", type_name::<T>());
    for actor_entity in actors.iter() {
        // Ignore clean actors
        if !dirty_nodes.contains(actor_entity) {
            continue;
        }

        let calculator = update_effect_tree_attributes::<T>(
            actor_entity,
            &graph,
            &nodes,
            &dirty_nodes,
            &statuses,
            &attributes,
            &attribute_refs,
            &effects,
            &modifiers,
            &mut commands,
        );

        // Signal to update the attribute
        commands.trigger(UpdateAttributeSignal {
            entity: actor_entity,
            calculator,
        });

        // Cleans the node
        commands.entity(actor_entity).try_remove::<Dirty<T>>();
    }
}

fn update_effect_tree_attributes<T: Attribute>(
    current_entity: Entity,
    graph: &DependencyGraph,
    nodes: &Query<&NodeType>,
    dirty_nodes: &Query<&Dirty<T>>,
    statuses: &Query<EffectStatusParam>,
    attributes: &Query<AttributeQueryDataReadOnly<T>>,
    attribute_refs: &Query<AttributesRef>,
    effects: &Query<(&OwnedModifiers, &EffectSource, &EffectTarget)>,
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
        NodeType::Actor => {
            // Traverse children
            let calculator = graph
                .neighbors(current_entity)
                .map(|entity| {
                    update_effect_tree_attributes::<T>(
                        entity,
                        graph,
                        nodes,
                        dirty_nodes,
                        statuses,
                        attributes,
                        attribute_refs,
                        effects,
                        modifiers,
                        commands,
                    )
                })
                .fold(AttributeCalculator::default(), |acc, child| {
                    acc.combine(child)
                });

            calculator
        }
        NodeType::Effect => {
            let Ok((modifier_entities, source, target)) = effects.get(current_entity) else {
                error!("{}: Error getting effect type.", current_entity);
                return AttributeCalculator::default();
            };

            let calculator = modifier_entities
                .iter()
                .filter_map(|modifier_entity| {
                    // Get the modifier
                    let Ok(modifier) = modifiers.get(modifier_entity) else {
                        return None;
                    };
                    // Get the effect to check if we should apply it
                    let Ok(status) = statuses.get(current_entity) else {
                        return None;
                    };
                    if status.is_periodic() || status.is_inactive() {
                        return None;
                    }

                    let [source, target, effect] = attribute_refs
                        .get_many([source.0, target.0, current_entity])
                        .unwrap();
                    let context = GameplayContext {
                        source_actor: &source,
                        target_actor: &target,
                        owner: &effect,
                    };

                    let calc = AttributeCalculator::convert(modifier, &context).unwrap_or_default();

                    Some(calc)
                })
                .fold(AttributeCalculator::default(), |acc, child| {
                    acc.combine(child)
                });

            calculator
        }
    };

    // Signal to update the attribute
    commands.trigger(UpdateAttributeSignal {
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
    if let Ok(mut attribute) = attributes.get_mut(trigger.event_target()) {
        attribute.calculator_cache.calculator = trigger.event().calculator;

        let old_value = attribute.attribute.current_value();

        let should_notify_observers = attribute.update_attribute(&trigger.event().calculator);
        if should_notify_observers {
            commands.trigger(CurrentValueChanged::<T> {
                entity: trigger.event_target(),
                phantom_data: Default::default(),
                old: old_value,
                new: attribute.attribute.current_value(),
            });
        }
    };
}

pub fn apply_periodic_effect<T: Attribute>(
    actors: Query<AttributesRef>,
    effects: Query<(
        AttributesRef,
        &Effect,
        &EffectTicker,
        &OwnedModifiers,
        &EffectTarget,
        &EffectSource,
    )>,
    modifiers: Query<&AttributeModifier<T>>,
    mut event_writer: MessageWriter<ApplyAttributeModifierMessage<T>>,
    effect_assets: Res<Assets<EffectDef>>,
) {
    for (effect_ref, effect, timer, owned_modifiers, target, source) in effects.iter() {
        if !timer.just_finished() {
            continue;
        }

        let effect_def = effect_assets
            .get(&effect.0)
            .ok_or("No effect asset.")
            .unwrap();

        let source_actor_ref = actors.get(source.0).unwrap();
        let target_actor_ref = actors.get(target.0).unwrap();

        let context = GameplayContext {
            target_actor: &target_actor_ref,
            source_actor: &source_actor_ref,
            owner: &effect_ref,
        };

        // Determines whether the effect should activate
        let should_be_activated = effect_def
            .activate_conditions
            .iter()
            .all(|condition| condition.0.eval(&context).unwrap_or(false));

        if !should_be_activated {
            continue;
        }

        // Timer has triggered. Grab modifiers and apply them.
        for children in owned_modifiers.iter() {
            let Ok(attribute_modifier) = modifiers.get(children) else {
                continue;
            };

            // Clone the modifier so we can apply the stack count to it.
            let applied_modifier = attribute_modifier.clone();

            event_writer.write(ApplyAttributeModifierMessage {
                source_entity: source.0,
                target_entity: target.0,
                effect_entity: effect_ref.id(),
                modifier: applied_modifier,
            });
        }
    }
}
