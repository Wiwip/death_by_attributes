mod abilities;
mod systems;

use crate::ability::systems::{
    activate_ability, reset_ability_cooldown, tick_ability_cooldown, try_activate_ability_observer,
};
use crate::assets::AbilityDef;
use bevy::prelude::*;

use crate::condition::{AbilityCondition, BoxCondition, TagCondition};
use crate::prelude::Value;
pub use abilities::{AbilityBuilder, GrantAbilityCommand};

pub struct AbilityPlugin;

impl Plugin for AbilityPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(PreUpdate, tick_ability_cooldown)
            .add_observer(try_activate_ability_observer)
            .add_observer(reset_ability_cooldown)
            .add_observer(activate_ability)
            .register_type::<AbilityOf>()
            .register_type::<Abilities>();
    }
}

/// The entity that this effect is targeting.
#[derive(Component, Reflect, Debug)]
#[relationship(relationship_target = Abilities)]
pub struct AbilityOf(pub Entity);

/// All effects that are targeting this entity.
#[derive(Component, Reflect, Debug, Default)]
#[relationship_target(relationship = AbilityOf, linked_spawn)]
pub struct Abilities(Vec<Entity>);

#[derive(Component)]
pub struct Ability(pub(crate) Handle<AbilityDef>);

#[derive(EntityEvent)]
pub struct TryActivateAbility {
    #[event_target]
    ability: Entity,
    condition: BoxCondition,
    target_data: TargetData,
}

impl TryActivateAbility {
    pub fn by_tag<T: Component>(target: Entity, target_data: TargetData) -> Self {
        Self {
            ability: target,
            condition: BoxCondition::new(TagCondition::<T>::owner()),
            target_data,
        }
    }
    pub fn by_def(target: Entity, handle: AssetId<AbilityDef>, target_data: TargetData) -> Self {
        Self {
            ability: target,
            condition: BoxCondition::new(AbilityCondition::new(handle)),
            target_data,
        }
    }
}

#[derive(Component, Reflect)]
pub struct AbilityCooldown {
    timer: Timer,
    #[reflect(ignore)]
    value: Value<f64>,
}

pub enum TargetData {
    SelfCast,
    Target(Entity),
}

#[derive(EntityEvent)]
pub struct AbilityBegin {
    pub source: Entity,
    #[event_target]
    pub ability: Entity,
}

#[derive(EntityEvent)]
pub struct AbilityExecute {
    pub target: Entity,
    pub source: Entity,
    #[event_target]
    pub ability: Entity,
}

#[derive(EntityEvent)]
pub struct AbilityEnd {
    pub source: Entity,
    #[event_target]
    pub ability: Entity,
}

#[derive(EntityEvent)]
pub struct AbilityCancel {
    pub source: Entity,
    #[event_target]
    pub ability: Entity,
}
