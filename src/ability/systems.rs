use crate::ability::{
    Abilities, Ability, AbilityCooldown, AbilityExecute, TargetData, TryActivateAbility,
};
use crate::assets::AbilityDef;
use crate::condition::{BoxCondition, GameplayContext};
use crate::{AttributesMut, AttributesRef};
use bevy::asset::Assets;
use bevy::prelude::*;
use bevy_egui::egui::debug_text::print;

pub fn tick_ability_cooldown(mut query: Query<&mut AbilityCooldown>, time: Res<Time>) {
    query.par_iter_mut().for_each(|mut cooldown| {
        cooldown.0.tick(time.delta());
    });
}

/// Tries to activate an ability.
///
/// Base conditions are:
/// - Cooldown
/// - Cost
pub fn try_activate_ability_observer(
    trigger: On<TryActivateAbility>,
    actors: Query<(AttributesRef, &Abilities), Without<AbilityCooldown>>,
    abilities: Query<(AttributesRef, &Ability, &AbilityCooldown)>,
    ability_assets: Res<Assets<AbilityDef>>,
    mut commands: Commands,
) -> Result<(), BevyError> {
    let (source_entity_ref, actor_abilities) = actors.get(trigger.ability)?;

    for &ability_entity in actor_abilities.0.iter() {
        let (ability_ref, ability, cooldown) = abilities
            .get(ability_entity)
            .expect("Ability not found in: try_activate_ability_observer.");
        let target_entity_ref = match trigger.target_data {
            TargetData::SelfCast => source_entity_ref,
            TargetData::Target(target) => actors.get(target)?.0,
        };

        if !cooldown.0.is_finished() {
            debug!("Ability on cooldown!");
            continue;
        }

        let ability_spec = ability_assets
            .get(&ability.0.clone())
            .ok_or("No ability asset.")?;

        let can_activate = can_activate_ability(
            &ability_ref,
            &source_entity_ref,
            &target_entity_ref,
            &ability_spec,
            &trigger.condition,
        )
        .ok()
        .unwrap_or(false);

        if can_activate {
            commands.trigger(AbilityCooldownReset(ability_entity));
            commands.trigger(ActivateAbility {
                target: target_entity_ref.id(),
                source: source_entity_ref.id(),
                ability: ability_entity,
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
    let context = GameplayContext {
        target_actor: &target_entity_ref,
        source_actor: &source_entity_ref,
        owner: &ability_entity,
    };
    let meet_conditions = conditions.0.eval(&context).unwrap_or(false);
    if !meet_conditions {
        debug!("Ability conditions not met!");
        return Ok(false);
    }

    let can_activate = ability_def
        .cost
        .iter()
        .all(|condition| condition.0.eval(&context).unwrap_or(false));

    if !can_activate {
        debug!("Insufficient resources to activate ability!");
        return Ok(false);
    }
    Ok(true)
}

#[derive(EntityEvent)]
pub(crate) struct AbilityCooldownReset(pub Entity);

pub(crate) fn reset_ability_cooldown(
    trigger: On<AbilityCooldownReset>,
    mut cooldowns: Query<&mut AbilityCooldown>,
) -> Result<(), BevyError> {
    let mut cooldown = cooldowns.get_mut(trigger.0)?;
    cooldown.0.reset();
    Ok(())
}

#[derive(EntityEvent)]
pub(crate) struct ActivateAbility {
    #[event_target]
    pub target: Entity,
    pub source: Entity,
    pub ability: Entity,
}

/// Bypass [TryActivateAbility]'s checks. Usually triggered after a successful [TryActivateAbility].
pub(crate) fn activate_ability(
    trigger: On<ActivateAbility>,
    mut actors: Query<AttributesMut>,
    abilities: Query<&Ability>,
    ability_assets: Res<Assets<AbilityDef>>,
    mut commands: Commands,
) -> Result<(), BevyError> {
    let mut actor_mut = actors.get_mut(trigger.target)?;
    let ability = abilities.get(trigger.ability)?;

    let ability_spec = ability_assets
        .get(&ability.0.clone())
        .ok_or("No ability asset.")?;

    debug!("Commit ability cost");
    for effect in &ability_spec.cost_effects {
        effect.apply(&mut actor_mut);
    }

    // Activate the ability
    debug!("{}: Execute ability", trigger.ability);
    commands.trigger(AbilityExecute {
        source: trigger.source,
        target: trigger.target,
        ability: trigger.ability,
    });
    Ok(())
}
