use crate::attributes::AttributeComponent;
use crate::effects::{Effect, EffectDuration, EffectPeriodicTimer};
use crate::modifiers::{EffectOf, Effects, ModAggregator, Modifier, ModifierOf};
use crate::{Actor, Dirty, OnValueChanged};
use bevy::ecs::component::Mutable;
use bevy::ecs::relationship::Relationship;
use bevy::prelude::*;
use ptree::{TreeBuilder, print_tree};
use std::any::type_name;
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

pub fn flag_dirty_modifier_nodes<T: Component>(
    changed: Query<Entity, Changed<Modifier<T>>>,
    parents: Query<&EffectOf>,
    dirty: Query<&Dirty<T>>,
    mut command: Commands,
) {
    'outer: for changed in changed.iter() {
        command.entity(changed).insert(Dirty::<T>::default());
        for entity in parents.iter_ancestors(changed) {
            // Stop marking as the rest of the hierarchy is already dirty
            if dirty.contains(entity) {
                continue 'outer;
            }

            command.entity(entity).insert(Dirty::<T>::default());
        }
    }
}

/// Navigates the tree descendants to update the tree attribute values
/// Effects that have a periodic timer application must be ignored in the current value calculations
pub fn update_attribute_tree_system<T: Component<Mutability = Mutable> + AttributeComponent>(
    actors: Query<Entity, (With<Actor>, With<Dirty<T>>)>,
    descendants: Query<&Effects, Without<EffectPeriodicTimer>>,
    modifiers: Query<&Modifier<T>>,
    dirty: Query<&Dirty<T>>,
    periodic_effects: Query<&Effect, With<EffectPeriodicTimer>>,
    mut attributes: Query<&mut T>,
    mut aggregator: Query<&mut ModAggregator<T>>,
    mut commands: Commands,
) {
    debug_once!(
        "Installed: update_attribute_tree_system::{}",
        type_name::<T>()
    );
    for actor_entity in actors.iter() {
        update_attribute_tree(
            actor_entity,
            descendants,
            modifiers,
            dirty,
            periodic_effects,
            &mut attributes,
            &mut aggregator,
            &mut commands,
        );
    }
}

fn update_attribute_tree<T: Component<Mutability = Mutable> + AttributeComponent>(
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
                    update_attribute_tree::<T>(
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
            Err(_) => ModAggregator::default(),
        };
    // Update the aggregator with the most-recent value
    let Ok(mut aggregator) = aggregators.get_mut(current_entity) else {
        return ModAggregator::default();
    };
    *aggregator = modifier_so_far.clone();
    // Edit the current attribute if the node has one
    if let Ok(mut attribute) = attributes.get_mut(current_entity) {
        let new_val = aggregator.evaluate(attribute.base_value());
        attribute.set_current_value(new_val)
    };
    // Cleans the node
    commands.entity(current_entity).remove::<Dirty<T>>();
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
    let name = entities.get(current_entity).unwrap();
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
    parents: Query<&ModifierOf>,
    mut attributes: Query<&mut T>,
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
    apply_modifiers_to_ancestors(
        modifier_entity,
        modifier,
        parents,
        &mut attributes,
        &mut commands,
    );
}

/// Despawn instant effects right after they were added as we cannot remove them in the triggers directly
pub fn despawn_instant_effect(
    effects: Query<Entity, (Added<Effect>, Without<EffectDuration>)>,
    mut commands: Commands,
) {
    for effect in effects.iter() {
        commands.entity(effect).despawn();
    }
}

pub fn trigger_periodic_effect<T: Component<Mutability = Mutable> + AttributeComponent>(
    effects: Query<(Entity, &Effect, &EffectPeriodicTimer, &Effects), With<EffectDuration>>,
    modifiers: Query<&Modifier<T>>,
    parents: Query<&ModifierOf>,
    mut attributes: Query<&mut T>,
    mut commands: Commands,
) {
    for (effect_entity, _, timer, effect_childrens) in effects.iter() {
        if !timer.0.just_finished() {
            continue;
        }
        // Timer has triggered. Grab modifiers and apply them.
        for children in effect_childrens.iter() {
            let Ok(modifier) = modifiers.get(children) else {
                continue;
            };
            apply_modifiers_to_ancestors(
                effect_entity,
                modifier,
                parents,
                &mut attributes,
                &mut commands,
            );
        }
    }
}

/// Traverses the tree ancestors till it reaches the root actor
/// Applies the given modifier to all attribute T on the way up
/// Marks nodes as dirty
fn apply_modifiers_to_ancestors<T: Component<Mutability = Mutable> + AttributeComponent>(
    origin_entity: Entity,
    modifier: &Modifier<T>,
    parents: Query<&ModifierOf>,
    attributes: &mut Query<&mut T>,
    commands: &mut Commands,
) {
    // We apply the modification to the ancestors till we reach the actor root
    for entity in parents.iter_ancestors(origin_entity) {
        // Assumes that the entire branch is dirty if a modifier is applied.
        // Consider replacing with triggers given we're mostly interested in updating the current value.
        commands.entity(entity).insert(Dirty::<T>::default());

        let Ok(mut attribute) = attributes.get_mut(entity) else {
            continue;
        };
        let old_value = attribute.base_value();
        attribute.set_base_value(modifier.value.evaluate(old_value));

        // Notify whenever we mutate an attribute
        commands.trigger_targets(OnValueChanged, entity);
    }
}

/*

pub fn trigger_periodic_effects(
    mut entities: Query<ActorEntityMut, (Without<Effect>, Without<Mutator>)>,
    effects: Query<
        (
            &Modifies,
            &ModifiedBy,
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

        debug!("Effect expired {:?}", entity);
        commands.entity(entity).despawn();
    }
}

pub fn on_duration_effect_removed(
    trigger: Trigger<OnRemove, Effect>,
    effects: Query<
        (&Modifies, &ModifiedBy, &Effect),
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
*/
