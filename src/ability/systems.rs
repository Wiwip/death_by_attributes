use crate::ability::{Abilities, Ability, AbilityCooldown, TargetData, TryActivateAbility};
use crate::assets::AbilityDef;
use crate::condition::{BoxCondition, ConditionContext};
use crate::trigger::{StoredCondition};
use crate::{AttributesMut, AttributesRef};
use bevy::asset::Assets;
use bevy::prelude::*;

pub fn tick_ability_cooldown(mut query: Query<&mut AbilityCooldown>, time: Res<Time>) {
    query.par_iter_mut().for_each(|mut cooldown| {
        cooldown.0.tick(time.delta());
    });
}

pub fn try_activate_ability_observer(
    trigger: Trigger<TryActivateAbility>,
    actors: Query<(AttributesRef, &Abilities), Without<AbilityCooldown>>,
    abilities: Query<(AttributesRef, &Ability, &AbilityCooldown)>,
    ability_assets: Res<Assets<AbilityDef>>,
    mut commands: Commands,
) -> Result<(), BevyError> {
    let (source_actor_ref, actor_abilities) = actors.get(trigger.target())?;

    for &ability_entity in actor_abilities.0.iter() {
        let (ability_ref, ability, cooldown) = abilities.get(ability_entity)?;
        let target_entity_ref = match trigger.target_data {
            TargetData::SelfCast => source_actor_ref,
            TargetData::Target(target) => actors.get(target)?.0,
        };

        if !cooldown.0.finished() {
            debug!("Ability on cooldown!");
            continue;
        }

        let ability_spec = ability_assets
            .get(&ability.0.clone())
            .ok_or("No ability asset.")?;

        let can_activate = can_activate_ability(
            &ability_ref,
            &source_actor_ref,
            &target_entity_ref,
            &ability_spec,
            &trigger.condition,
        )
        .ok()
        .unwrap_or(false);

        if can_activate {
            commands.entity(ability_entity).trigger(ResetCooldown);
            commands.entity(ability_entity).trigger(ActivateAbility {
                caller: source_actor_ref.id(),
            });
        }
    }

    Ok(())
}

fn can_activate_ability(
    ability_entity: &AttributesRef,
    source_entity_ref: &AttributesRef,
    target_entity_ref: &AttributesRef,
    ability_def: &AbilityDef,
    conditions: &BoxCondition,
) -> Result<bool, BevyError> {
    let context = ConditionContext {
        target_actor: &target_entity_ref,
        source_actor: &source_entity_ref,
        owner: &ability_entity,
    };
    let meet_conditions = conditions.0.eval(&context);
    if !meet_conditions {
        debug!("Ability conditions not met!");
        return Ok(false);
    }

    let can_activate = ability_def
        .cost
        .iter()
        .all(|condition| condition.0.eval(&context));

    if !can_activate {
        debug!("Insufficient resources to activate ability!");
        return Ok(false);
    }
    Ok(true)
}

#[derive(Event)]
pub(crate) struct ResetCooldown;

pub(crate) fn reset_ability_cooldown(
    trigger: Trigger<ActivateAbility>,
    mut cooldowns: Query<&mut AbilityCooldown>,
) -> Result<(), BevyError> {
    let mut cooldown = cooldowns.get_mut(trigger.target())?;
    cooldown.0.reset();
    Ok(())
}

#[derive(Event)]
pub(crate) struct ActivateAbility {
    caller: Entity,
}

pub(crate) fn activate_ability(
    trigger: Trigger<ActivateAbility>,
    mut actors: Query<AttributesMut>,
    abilities: Query<&Ability>,
    ability_assets: Res<Assets<AbilityDef>>,
    mut commands: Commands,
) -> Result<(), BevyError> {
    let mut actor_mut = actors.get_mut(trigger.caller)?;
    let ability = abilities.get(trigger.target())?;

    let ability_spec = ability_assets
        .get(&ability.0.clone())
        .ok_or("No ability asset.")?;

    debug!("Commit ability cost!");
    for effect in &ability_spec.cost_effects {
        effect.apply(&mut actor_mut);
    }

    // Activate the ability
    (ability_spec.activation_fn)(&mut actor_mut, &mut commands);
    Ok(())
}
