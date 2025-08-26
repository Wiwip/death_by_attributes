mod abilities;
mod systems;

use crate::AttributesMut;
use crate::ability::systems::{
    activate_ability, reset_ability_cooldown, tick_ability_cooldown, try_activate_ability_observer,
};
use crate::assets::AbilityDef;
use bevy::prelude::*;

use crate::condition::{AbilityCondition, BoxCondition, TagCondition};
use crate::prelude::Who;
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

pub type AbilityActivationFn = Box<dyn Fn(&mut AttributesMut, &mut Commands) + Send + Sync>;

/// The entity that this effect is targeting.
#[derive(Component, Reflect, Debug)]
#[relationship(relationship_target = Abilities)]
pub struct AbilityOf(pub Entity);

/// All effects that are targeting this entity.
#[derive(Component, Reflect, Debug)]
#[relationship_target(relationship = AbilityOf, linked_spawn)]
pub struct Abilities(Vec<Entity>);

#[derive(Component)]
pub struct Ability(pub(crate) Handle<AbilityDef>);

#[derive(Event)]
pub struct TryActivateAbility {
    condition: BoxCondition,
    target_data: TargetData,
}

impl TryActivateAbility {
    pub fn by_tag<T: Component>(target_data: TargetData) -> Self {
        Self {
            condition: BoxCondition::new(TagCondition::<T>::owner()),
            target_data,
        }
    }
    pub fn by_def(handle: AssetId<AbilityDef>, target_data: TargetData) -> Self {
        Self {
            condition: BoxCondition::new(AbilityCondition::new(handle)),
            target_data,
        }
    }
}

#[derive(Component)]
pub struct AbilityCooldown(pub Timer);

pub enum TargetData {
    SelfCast,
    Target(Entity),
}
