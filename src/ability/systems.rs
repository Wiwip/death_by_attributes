use crate::ability::{
    Ability, AbilityCooldown, AbilityExecute, AbilityOf, GrantedAbilities, TargetData,
    TryActivateAbility,
};
use crate::assets::AbilityDef;
use crate::condition::{BoxCondition, GameplayContext, GameplayContextMut};
use crate::{AttributesMut, AttributesRef};
use bevy::asset::Assets;
use bevy::prelude::*;
use std::time::Duration;
use crate::expression::Expression;

pub fn tick_ability_cooldown(mut query: Query<&mut AbilityCooldown>, time: Res<Time>) {
    query.par_iter_mut().for_each(|mut cooldown| {
        cooldown.timer.tick(time.delta());
    });
}

/// Tries to activate an ability.
///
/// Base conditions are:
/// - Cooldown
/// - Conditions
/// - Cost
pub fn try_activate_ability_observer(
    trigger: On<TryActivateAbility>,
    actors: Query<(AttributesRef, &GrantedAbilities), Without<AbilityCooldown>>,
    abilities: Query<(AttributesRef, &Ability, Option<&AbilityCooldown>)>,
    ability_assets: Res<Assets<AbilityDef>>,
    mut commands: Commands,
) -> Result<(), BevyError> {
    let Ok((source_entity_ref, actor_abilities)) = actors.get(trigger.ability) else {
        warn!("The Actor({}) has no GrantedAbilities", trigger.ability);
        return Ok(());
    };

    let target_entity_ref = match trigger.target_data {
        TargetData::SelfCast => source_entity_ref,
        TargetData::Target(target) => {
            let Ok((entity, _)) = actors.get(target) else {
                return Ok(());
            };
            entity
        }
    };

    for &ability_entity in actor_abilities.0.iter() {
        let (ability_ref, ability, opt_cooldown) = abilities
            .get(ability_entity)
            .expect("Ability not found in: try_activate_ability_observer.");

        // Handle cooldowns
        let is_finished = match opt_cooldown {
            None => true,
            Some(cd) => cd.timer.is_finished(),
        };
        if !is_finished {
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
            commands.trigger(AbilityCooldownReset {
                target: target_entity_ref.id(),
                source: source_entity_ref.id(),
                ability: ability_entity,
            });
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
    ability_entity_ref: &AttributesRef,
    source_entity_ref: &AttributesRef,
    target_entity_ref: &AttributesRef,
    ability_def: &AbilityDef,
    conditions: &BoxCondition,
) -> Result<bool, BevyError> {
    let context = GameplayContext {
        target_actor: &target_entity_ref,
        source_actor: &source_entity_ref,
        owner: &ability_entity_ref,
    };
    let meet_conditions = conditions.0.eval(&context).unwrap_or(false);
    if !meet_conditions {
        debug!(
            "Ability({}) conditions[{:?}] not met for: {}.",
            ability_entity_ref.id(),
            conditions,
            ability_def.name
        );
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
pub(crate) struct AbilityCooldownReset {
    pub source: Entity,
    pub target: Entity,
    #[event_target]
    pub ability: Entity,
}

pub(crate) fn reset_ability_cooldown(
    trigger: On<AbilityCooldownReset>,
    mut cooldowns: Query<(&AbilityOf, &mut AbilityCooldown)>,
    query: Query<AttributesRef>,
) -> Result<(), BevyError> {
    let Ok((_parent, mut cooldown)) = cooldowns.get_mut(trigger.ability) else {
        // This event does not affect an ability without a cooldown.
        return Ok(());
    };

    let [source, target, owner] = query.get_many([trigger.source, trigger.target, trigger.ability])?;
    let context = GameplayContext {
        target_actor: &source,
        source_actor: &target,
        owner: &owner,
    };

    //let entity_ref = query.get(parent.0)?;
    //let cd_value = cooldown.value.eval(&entity_ref)?;
    let cd_value = cooldown.value.eval(&context)?;

    cooldown
        .timer
        .set_duration(Duration::from_secs_f64(cd_value));
    cooldown.timer.reset();
    Ok(())
}

#[derive(EntityEvent)]
pub struct ActivateAbility {
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
    //let [mut target_actor_mut, mut source_actor_mut, mut ability_actor_mut] =
    //    actors.get_many_mut([trigger.target, trigger.source, trigger.ability])?;
    
    
    /*let mut context = GameplayContextMut {
        target_actor: &mut target_actor_mut,
        source_actor: &mut source_actor_mut,
        owner: &mut ability_actor_mut,
    };*/
    
    debug!("{}: Commit ability cost.", trigger.ability);
    let ability = abilities.get(trigger.ability)?;
    let ability_spec = ability_assets
        .get(&ability.0.clone())
        .ok_or("No ability asset.")?;
    
    for modifiers in &ability_spec.cost_modifiers {
        //modifiers.apply_immediate(&mut context);
        modifiers.apply_delayed(trigger.source, trigger.target, trigger.ability, &mut commands);
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
