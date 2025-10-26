use crate::condition::GameplayContext;
use bevy::prelude::*;
use serde::Serialize;
use std::marker::PhantomData;

pub trait EffectExecution: Send + Sync {
    fn run(&self, context: &GameplayContext) -> std::result::Result<bool, BevyError>;
}

pub type StoredExecution = Box<dyn EffectExecution>;

/// A condition that wraps a closure or function pointer.
///
/// This allows for creating custom, inline condition logic without needing
/// to define a new struct for every case.
#[derive(Debug, Serialize)]
pub struct FunctionActivation<Input, F> {
    f: F,
    marker: PhantomData<fn() -> Input>,
}

pub trait EffectParam: Send + Sync {
    type Item<'new>;

    fn retrieve<'r>(context: &'r GameplayContext) -> Self::Item<'r>;
}

#[derive(Deref)]
pub struct Dst<'a, T: 'static> {
    value: &'a T,
}

impl<'res, T: 'static + Component> EffectParam for Dst<'res, T> {
    type Item<'new> = Dst<'new, T>;

    fn retrieve<'r>(context: &'r GameplayContext) -> Self::Item<'r> {
        Dst {
            value: context
                .target_actor
                .get::<T>()
                .expect("Missing target attribute"),
        }
    }
}



#[derive(Deref)]
pub struct Src<'a, T: 'static> {
    value: &'a T,
}

impl<'res, T: 'static + Component> EffectParam for Src<'res, T> {
    type Item<'new> = Src<'new, T>;

    fn retrieve<'r>(context: &'r GameplayContext) -> Self::Item<'r> {
        Src {
            value: context
                .source_actor
                .get::<T>()
                .expect("Missing source attribute"),
        }
    }
}

macro_rules! impl_custom_execution {
    ($($params:ident),*) => {
        #[allow(unused_variables)]
        #[allow(non_snake_case)]
        impl<F: Send + Sync, $($params : EffectParam),*> EffectExecution for FunctionActivation<($($params ,)*), F>
            where
                for<'a, 'b> &'a F:
                    Fn($($params),*) -> Result<bool, BevyError> +
                    Fn($(<$params as EffectParam>::Item<'b>),*) -> Result<bool, BevyError>,
        {
            fn run(&self, context: &GameplayContext) -> Result<bool, BevyError> {
                fn call_inner<$($params),*>(
                    f: impl Fn($($params),*) -> Result<bool, BevyError>,
                    $($params: $params),*
                ) -> Result<bool, BevyError> {
                    f($($params),*)
                }

                $(
                    let $params = $params::retrieve(context);
                )*

                call_inner(&self.f, $($params),*)
            }
        }
    };
}

impl_custom_execution!();
impl_custom_execution!(T1);
impl_custom_execution!(T1, T2);
impl_custom_execution!(T1, T2, T3);
impl_custom_execution!(T1, T2, T3, T4);
impl_custom_execution!(T1, T2, T3, T4, T5);
impl_custom_execution!(T1, T2, T3, T4, T5, T6);
impl_custom_execution!(T1, T2, T3, T4, T5, T6, T7);
impl_custom_execution!(T1, T2, T3, T4, T5, T6, T7, T8);

pub trait IntoEffectExecution<'a, Input> {
    type ExecFunction: EffectExecution;

    fn into_condition(self) -> Self::ExecFunction;
}

impl<F: Fn(T1) -> Result<bool, BevyError> + Send + Sync, T1: EffectParam>
    IntoEffectExecution<'_, (T1,)> for F
where
    for<'a, 'b> &'a F: Fn(T1) -> Result<bool, BevyError>
        + Fn(<T1 as EffectParam>::Item<'b>) -> Result<bool, BevyError>,
{
    type ExecFunction = FunctionActivation<(T1,), Self>;

    fn into_condition(self) -> Self::ExecFunction {
        FunctionActivation {
            f: self,
            marker: PhantomData,
        }
    }
}

impl<F: Fn(T1, T2) -> Result<bool, BevyError> + Send + Sync, T1: EffectParam, T2: EffectParam>
    IntoEffectExecution<'_, (T1, T2)> for F
where
    for<'a, 'b> &'a F: Fn(T1, T2) -> Result<bool, BevyError>
        + Fn(<T1 as EffectParam>::Item<'b>, <T2 as EffectParam>::Item<'b>) -> Result<bool, BevyError>,
{
    type ExecFunction = FunctionActivation<(T1, T2), Self>;

    fn into_condition(self) -> Self::ExecFunction {
        FunctionActivation {
            f: self,
            marker: PhantomData,
        }
    }
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
