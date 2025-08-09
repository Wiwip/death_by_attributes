use crate::attributes::Attribute;
use crate::modifier::Mutator;
use crate::{AttributesMut, AttributesRef};
use bevy::prelude::*;
use std::any::TypeId;
use std::collections::HashMap;
use crate::effect::EffectTargeting;
use crate::prelude::{Effect, Effects};

#[derive(Component)]
pub struct EffectCalculationContext<'a> {
    source_map: HashMap<TypeId, f64>,
    target_map: HashMap<TypeId, f64>,
    source_actor: AttributesRef<'a>,
    target_actor: AttributesRef<'a>,
    pub modifiers: Vec<Box<dyn Mutator>>,
}

impl<'a> EffectCalculationContext<'a> {
    pub fn new(capture_context: EffectCaptureContext<'a>) -> Self {
        Self {
            source_map: capture_context.source_map,
            target_map: capture_context.target_map,
            source_actor: capture_context.source_actor,
            target_actor: capture_context.target_actor,
            modifiers: vec![],
        }
    }

    pub fn capture_source<T: Attribute>(
        &mut self,
        entity_ref: &EntityRef,
    ) -> Result<(), BevyError> {
        let value = entity_ref.get::<T>().ok_or("Could not get attribute")?;
        self.source_map
            .insert(TypeId::of::<T>(), value.current_value());
        Ok(())
    }

    pub fn capture_target<T: Attribute>(
        &mut self,
        entity_ref: &EntityRef,
    ) -> Result<(), BevyError> {
        let value = entity_ref.get::<T>().ok_or("Could not get attribute")?;
        self.target_map
            .insert(TypeId::of::<T>(), value.current_value());
        Ok(())
    }

    pub fn get_source<T: Attribute>(&self) -> Option<&f64> {
        self.source_map.get(&TypeId::of::<T>())
    }

    pub fn get_target<T: Attribute>(&self) -> Option<&f64> {
        self.target_map.get(&TypeId::of::<T>())
    }

    pub fn to_modifiers(self) -> Vec<Box<dyn Mutator>> {
        self.modifiers
    }
}

pub struct EffectCaptureContext<'a> {
    pub(crate) target_map: HashMap<TypeId, f64>,
    pub(crate) source_map: HashMap<TypeId, f64>,
    pub(crate) source_actor: AttributesRef<'a>,
    pub(crate) target_actor: AttributesRef<'a>,
}

impl<'a> EffectCaptureContext<'a> {
    pub fn capture_source<T: Attribute>(&mut self) -> Result<(), BevyError> {
        let value = self
            .source_actor
            .get::<T>()
            .ok_or("Could not get attribute")?;
        self.source_map
            .insert(TypeId::of::<T>(), value.current_value());
        Ok(())
    }

    pub fn capture_target<T: Attribute>(&mut self) -> Result<(), BevyError> {
        let value = self
            .target_actor
            .get::<T>()
            .ok_or("Could not get attribute")?;
        self.target_map
            .insert(TypeId::of::<T>(), value.current_value());
        Ok(())
    }

    pub fn from(
        targeting: &EffectTargeting,
        actors: &'a mut Query<(Option<&Effects>, AttributesMut), Without<Effect>>,
    ) -> Self {
        let (source_actor, target_actor) = match targeting {
            EffectTargeting::SelfCast(entity) => {
                let (_, actor) = actors.get(*entity).unwrap();
                (actor, actor)
            }
            EffectTargeting::Targeted { source, target } => {
                let (_, source_actor_ref) = actors.get(*target).unwrap();
                let (_, target_actor_ref) = actors.get(*source).unwrap();
                (source_actor_ref, target_actor_ref)
            }
        };

        Self {
            target_map: Default::default(),
            source_map: Default::default(),
            source_actor,
            target_actor,
        }
    }
}

pub trait EffectExecution: Send + Sync {
    fn capture_attributes(
        &self,
        context: &mut EffectCaptureContext,
    ) -> Result<(), BevyError>;
    fn execute_effect(
        &self,
        context: &mut EffectCalculationContext,
    ) -> Result<(), BevyError>;
}

#[cfg(test)]
mod tests {
    /*use super::*;
    use crate::ReflectAccessAttribute;
    use crate::attributes::Attribute;
    use crate::context::EffectContext;
    use crate::modifiers::{AttributeModifier, ModTarget, ModType, Mutator};
    use crate::{AttributesRef, attribute};
    use bevy::prelude::*;
    use std::any::TypeId;

    attribute!(Health);
    attribute!(Damage);

    struct TestCalculation;

    impl EffectExecution for TestCalculation {
        fn capture_attributes(
            &self,
            context: &mut EffectCaptureContext,
        ) -> Result<(), BevyError> {
            context.capture_source::<Damage>()?;
            Ok(())
        }

        fn execute_effect(
            &self,
            context: &mut EffectCalculationContext,
        ) -> Result<(), BevyError> {
            let damage = context
                .get_source::<Damage>()
                .ok_or("No damage attribute captured.")?;

            let damage_mod =
                AttributeModifier::<Health>::new(*damage, ModType::Additive, ModTarget::Target);
            context.modifiers.push(Box::new(damage_mod));

            Ok(())
        }
    }*/

    #[test]
    fn test_execute_effect() {
        /*let mut app = App::new();
        let world = app.world_mut();

        let effect = EffectBuilder::new()
            .with_instant_application()
            .with_custom_execution(TestCalculation)
            .build();

        world.spawn((Health::new(100.0), Damage::new(10.0)));

        app.add_systems(
            Update,
            |query: Query<Entity, With<Health>>, context: EffectContext| {
                let query = query.single().unwrap();

                //context.apply_effect_to_self()
            },
        );*/
    }
}
