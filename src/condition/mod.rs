use crate::condition::systems::evaluate_effect_conditions;
use bevy::app::{App, Plugin};
use bevy::prelude::*;
use bevy_inspector_egui::__macro_exports::bevy_reflect::TypeRegistryArc;
use express_it::context::{AttributeKey, EvalContext};
use express_it::expr::ExpressionError;
use std::any::{Any, TypeId};

mod conditions;
mod systems;

use crate::attributes::Attribute;
use crate::modifier::Who;
use crate::schedule::EffectsSet;
use crate::{AttributesMut, AttributesRef};
pub use conditions::{
    AbilityCondition, ChanceCondition, HasComponent, IsAttributeWithinBounds, StackCondition,
};

pub struct ConditionPlugin;

impl Plugin for ConditionPlugin {
    fn build(&self, app: &mut App) {
        // This system is responsible for checking conditions and
        // activating/deactivating their related effects.
        app.add_systems(
            Update,
            evaluate_effect_conditions.in_set(EffectsSet::Prepare),
        );
        //app.add_systems(Update, evaluate_effect_conditions.in_set(EffectsSet::Notify));
    }
}

pub struct GameplayContextMut<'w, 's> {
    pub source_actor: Entity,
    pub target_actor: Entity,
    pub owner: Entity,

    pub actors: Query<'w, 's, AttributesMut<'static, 'static>>,
}

impl GameplayContextMut<'_, '_> {
    pub fn entity(&self, who: Who) -> Entity {
        match who {
            Who::Target => self.target_actor,
            Who::Source => self.source_actor,
            Who::Owner => self.owner,
        }
    }

    pub fn attribute_ref(&self, who: Who) -> AttributesRef<'_> {
        self.actors.get(self.entity(who)).unwrap()
    }

    pub fn attribute_mut(&mut self, who: Who) -> AttributesMut<'_, '_> {
        self.actors.get_mut(self.entity(who)).unwrap()
    }
}

pub struct BevyContext<'w> {
    pub source_actor: &'w AttributesRef<'w>,
    pub target_actor: &'w AttributesRef<'w>,
    pub owner: &'w AttributesRef<'w>,

    pub type_registry: TypeRegistryArc,
}

impl BevyContext<'_> {
    pub fn entity(&self, who: Who) -> Entity {
        match who {
            Who::Target => self.target_actor.id(),
            Who::Source => self.source_actor.id(),
            Who::Owner => self.owner.id(),
        }
    }

    pub fn attribute_ref(&self, who: Who) -> &AttributesRef<'_> {
        match who {
            Who::Target => self.target_actor,
            Who::Source => self.source_actor,
            Who::Owner => self.owner,
        }
    }
}

impl EvalContext for BevyContext<'_> {
    fn get_any(&self, path: &AttributeKey) -> std::result::Result<&dyn Any, ExpressionError> {
        let Some((scope, remainder)) = path.name.split_once('.') else  {
            return Err(ExpressionError::InvalidPath)
        };

        let actor = match scope {
            "src" => self.source_actor,
            "dst" => self.target_actor,
            "parent" => self.owner,
            _ => return Err(ExpressionError::InvalidPath),
        };

        let registry = self.type_registry.read();
        let Some(type_registration) = registry.get(path.type_id) else {
            return Err(ExpressionError::FailedReflect(
                "Failed to get type registration".into(),
            ));
        };
        let Some(reflect_component) = type_registration.data::<ReflectComponent>() else {
            return Err(ExpressionError::FailedReflect(
                "No reflect access attribute found".into(),
            ));
        };
        let Some(dyn_reflect) = reflect_component.reflect(actor) else {
            return Err(ExpressionError::FailedReflect(
                "The entity has no component the requested type.".into(),
            ));
        };

        let dyn_partial_reflect = dyn_reflect.reflect_path(remainder).map_err(|err| {
            ExpressionError::FailedReflect(format!("Invalid reflect path: {err}").into())
        })?;

        let dyn_path_reflect = dyn_partial_reflect.try_as_reflect().ok_or_else(|| {
            ExpressionError::FailedReflect(
                "Reflect value does not support further reflection".into(),
            )
        })?;

        Ok(dyn_path_reflect.as_any())
    }

    fn get_any_component(&self, path: &str, type_id: TypeId) -> std::result::Result<&dyn Any, ExpressionError> {
        let actor = match path {
            "src" => self.source_actor,
            "dst" => self.target_actor,
            "parent" => self.owner,
            _ => return Err(ExpressionError::InvalidPath),
        };

        let registry = self.type_registry.read();
        let Some(type_registration) = registry.get(type_id) else {
            return Err(ExpressionError::FailedReflect(
                "Failed to get type registration".into(),
            ));
        };
        let Some(reflect_component) = type_registration.data::<ReflectComponent>() else {
            return Err(ExpressionError::FailedReflect(
                "No reflect access attribute found".into(),
            ));
        };
        let Some(dyn_reflect) = reflect_component.reflect(actor) else {
            return Err(ExpressionError::FailedReflect(
                "The entity has no component the requested type.".into(),
            ));
        };

        Ok(dyn_reflect.as_any()) // Component (Attribute), not a primitive
    }
}

/*
#[cfg(test)]
mod test {
    use super::*;

    use std::marker::PhantomData;

    attribute!(Test1, f32);
    attribute!(Test2, f32);

    #[test]
    fn test() {
        let mut world = World::new();
        world.spawn((Test1::new(100.0), Test2::new(100.0)));

        let _ = world.run_system_once(|actor: Single<AttributesRef>| {
            let ctx = BevyContext {
                source_actor: &actor,
                target_actor: &actor,
                owner: &actor,

            };
        });
    }
}
*/
