use crate::ReflectAccessAttribute;
use crate::condition::systems::evaluate_effect_conditions;
use bevy::app::{App, Plugin};
use bevy::ecs::system::RunSystemOnce;
use bevy::prelude::*;
use bevy_inspector_egui::__macro_exports::bevy_reflect::TypeRegistryArc;
use express_it::context::{EvalContext, Path, RetrieveAttribute};
use express_it::expr::ExpressionError;
use express_it::float::FloatBinaryOp;
use num_traits::{AsPrimitive, Float, Num};
use std::any::{Any, TypeId};
use std::fmt::Debug;
use bevy::reflect::ReflectFromPtr;
use bevy_inspector_egui::restricted_world_view::Error;

mod conditions;
mod systems;

use crate::{AttributesMut, AttributesRef, attribute};

use crate::attributes::Attribute;
use crate::modifier::Who;
use crate::schedule::EffectsSet;
pub use conditions::{
    AbilityCondition, And, AttributeCondition, ChanceCondition, ConditionExt, Not, Or,
    StackCondition, TagCondition,
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

pub trait Condition: Debug + Send + Sync {
    fn eval(&self, context: &BevyContext) -> Result<bool, BevyError>;
}

#[derive(Debug)]
pub struct BoxCondition(pub Box<dyn Condition>);

impl BoxCondition {
    pub fn new<C: Condition + 'static>(condition: C) -> Self {
        Self(Box::new(condition))
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
    fn get_any(
        &self,
        path: &Path,
        type_id: TypeId,
    ) -> std::result::Result<&dyn Any, ExpressionError> {

        let registry = self.type_registry.read();
        println!("Registrations: {}", registry.iter().count());
        let reflect_attribute = registry.get_type_data::<ReflectAccessAttribute>(type_id);
        let Some(reflect_access_attribute) = reflect_attribute else {
            return Err(ExpressionError::FailedReflect("Failed to get type data.".into()));
        };

        let Some(type_registration) = registry.get(type_id) else {
            error!("Failed to get type registration for entity {:?}", type_id);
            return Err(ExpressionError::FailedReflect("Failed to get type registration".into()));
        };
        let Some(reflect_component) = type_registration.data::<ReflectComponent>() else {
            error!("No reflect access attribute found");
            return Err(ExpressionError::FailedReflect("No reflect access attribute found".into()));
        };
        let Some(dyn_reflect) = reflect_component.reflect(self.source_actor) else {
            error!("Failed to reflect entity");
            return Err(ExpressionError::FailedReflect("Failed to reflect entity".into()));
        };

        let Some(attribute) = reflect_access_attribute.get(dyn_reflect) else {
            return Err(ExpressionError::FailedReflect("reflect_access_attribute.get(dyn_reflect)".into()));
        };

        Ok(attribute.any_current_value())
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