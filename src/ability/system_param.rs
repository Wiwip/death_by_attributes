use crate::ability::{
    Ability, AbilityError, GrantAbilityCommand, GrantedAbilities, TargetData, TryActivateAbility,
};
use crate::actors::Actor;
use crate::assets::AbilityDef;
use bevy::ecs::system::SystemParam;
use bevy::prelude::*;

#[derive(SystemParam)]
pub struct AbilityContext<'w, 's> {
    abilities: Query<'w, 's, &'static Ability>,
    actors: Query<'w, 's, (&'static Actor, &'static GrantedAbilities)>,
    ability_definitions: Res<'w, Assets<AbilityDef>>,
    commands: Commands<'w, 's>,
}

impl<'w, 's> AbilityContext<'w, 's> {
    pub fn grant_ability(
        &mut self,
        ability: &Handle<AbilityDef>,
        grant_ability_target_entity: Entity,
    ) -> Result<Entity, AbilityError> {
        if !self.actors.contains(grant_ability_target_entity) {
            return Err(
                AbilityError::GrantingAbilityToNonActor(grant_ability_target_entity).into(),
            );
        }

        let ability_id = self
            .commands
            .spawn_empty()
            .queue(GrantAbilityCommand {
                parent: grant_ability_target_entity,
                handle: ability.clone(),
            })
            .id();

        Ok(ability_id)
    }

    pub fn try_activate_by_tag<T: Component>(&mut self, entity: Entity) {
        self.commands.trigger(TryActivateAbility::by_tag::<T>(
            entity,
            TargetData::SelfCast,
        ));
    }

    pub fn try_activate_by_def<T: Component>(
        &mut self,
        entity: Entity,
        definition: AssetId<AbilityDef>,
    ) {
        self.commands.trigger(TryActivateAbility::by_def(
            entity,
            definition,
            TargetData::SelfCast,
        ));
    }

    pub fn ability_def(&self, entity: Entity) -> Result<&AbilityDef, AbilityError> {
        let ability = self
            .abilities
            .get(entity)
            .or(Err(AbilityError::AbilityDoesNotExist(entity)))?;
        let definition = self
            .ability_definitions
            .get(&ability.0)
            .ok_or(AbilityError::AbilityDoesNotExist(entity))?;

        Ok(definition)
    }
}
