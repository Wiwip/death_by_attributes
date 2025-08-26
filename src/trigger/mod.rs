mod builder;

use crate::attribute;
use crate::condition::{BoxCondition, ConditionContext};
use crate::context::EffectContext;
use crate::prelude::Attribute;
use crate::{AttributesRef, ReflectAccessAttribute};
use bevy::prelude::*;
use std::marker::PhantomData;

/// Triggers are essentially automated abilities.
/// An ability or effect is automatically applied whenever the conditions of the trigger are met
/// So far my trigger ideas are:
/// - AbilityTrigger
/// - EffectTrigger
/// - TimedTrigger
pub struct TriggerPlugin {}

impl Plugin for TriggerPlugin {
    fn build(&self, app: &mut App) {
        //app.add_systems();
    }
}

#[derive(Event, Debug)]
pub struct GameplayTriggerEvent;

/*#[derive(Component)]
pub struct GameplayCondition {

}*/

/*pub struct StoredGameplayTrigger(pub Box<dyn GameplayTrigger>);

pub trait GameplayTrigger {
    fn evaluate(&self, context: &ConditionContext) -> bool;
    fn trigger(&self);
}*/

attribute!(Health);
attribute!(MaxHealth);

#[derive(Component)]
struct LowHealthTag;

fn global_condition_system<Tag, T1>(
    query: Query<(Entity, &T1), Changed<T1>>,
    mut commands: Commands,
) where
    Tag: Component + Default,
    T1: Attribute,
{
    for (entity, t1) in query.iter() {
        if t1.current_value() < 100 {
            commands.entity(entity).try_insert(Tag::default());
        } else {
            commands.entity(entity).remove::<Tag>();
        }
    }
}

pub type StoredCondition = Box<dyn CustomExecution + Send + Sync>;

pub struct EffectCustomExecution<Input, F> {
    f: F,
    marker: PhantomData<fn() -> Input>,
}

pub trait CustomExecution: Send + Sync {
    fn run(&self, context: &ConditionContext) -> Result<bool, BevyError>;
}

pub trait EffectParam: Send + Sync {
    type Item<'new>;

    fn retrieve<'r>(context: &'r ConditionContext) -> Self::Item<'r>;
}

struct Target<'a, T: 'static> {
    value: &'a T,
}

impl<'res, T: 'static + Component> EffectParam for Target<'res, T> {
    type Item<'new> = Target<'new, T>;

    fn retrieve<'r>(context: &'r ConditionContext) -> Self::Item<'r> {
        Target {
            value: context
                .target_actor
                .get::<T>()
                .expect("Missing target attribute"),
        }
    }
}

struct Source<'a, T: 'static> {
    value: &'a T,
}

impl<'res, T: 'static + Component> EffectParam for Source<'res, T> {
    type Item<'new> = Source<'new, T>;

    fn retrieve<'r>(context: &'r ConditionContext) -> Self::Item<'r> {
        Source {
            value: context
                .source_actor
                .get::<T>()
                .expect("Missing source attribute"),
        }
    }
}


fn x(){
    let a = |health: Target<(Health)>| {

    };
    let b = a.into_condition();

}

fn query(mut query: Query<AttributesRef>) {

    query.transmute_lens::<&Health>();

}
/*impl<F: Fn() -> Result<bool, BevyError>> CustomExecution for EffectCustomExecution<(), F>
where
    F: Send + Sync,
{
    fn eval(&self, _: &ConditionContext) -> Result<bool, BevyError> {
        (self.f)()
    }
}*/

impl<F: Send + Sync, T1: EffectParam> CustomExecution for EffectCustomExecution<(T1,), F>
where
    for<'a, 'b> &'a F: Fn(T1) + Fn(<T1 as EffectParam>::Item<'b>),
{
    fn run(&self, context: &ConditionContext) -> Result<bool, BevyError> {
        fn call_inner<T1>(f: impl Fn(T1), _0: T1) {
            f(_0);
        }

        let _0 = T1::retrieve(context); // attributes.get::<T1>().ok_or("Missing attribute")?;
        call_inner(&self.f, _0);

        Ok(true)
    }
}

/*impl<F, T1, T2> CustomExecution for EffectCustomExecution<(T1, T2), F>
where
    F: Fn(&T1, &T2) -> bool + Send + Sync,
    T1: EffectParam,
    T2: EffectParam,
{
    fn eval(&self, context: &ConditionContext) -> Result<bool, BevyError> {
        let _0 = T1::retrieve(context); // attributes.get::<T1>().ok_or("Missing attribute")?;
        let _1 = T2::retrieve(context); //attributes.get::<T2>().ok_or("Missing attribute")?;

        Ok((self.f)(_0, _1))
    }
}*/

pub trait IntoGameplayCondition<Input> {
    type Execution: CustomExecution;

    fn into_condition(self) -> Self::Execution;
}

/*impl<F: Fn() -> Result<bool, BevyError>> IntoGameplayCondition<()> for F
where
    F: Send + Sync,
{
    type Execution = EffectCustomExecution<(), Self>;

    fn into_condition(self) -> Self::Execution {
        EffectCustomExecution {
            f: self,
            marker: PhantomData,
        }
    }
}*/

impl<F: Fn(T1) + Send + Sync, T1: EffectParam> IntoGameplayCondition<(T1,)> for F
where
    for<'a, 'b> &'a F: Fn(T1) + Fn(<T1 as EffectParam>::Item<'b>),
{
    type Execution = EffectCustomExecution<(T1,), Self>;

    fn into_condition(self) -> Self::Execution {
        EffectCustomExecution {
            f: self,
            marker: PhantomData,
        }
    }
}
/*
impl<F: Fn(&T1, &T2) -> bool, T1, T2> IntoGameplayCondition<(T1, T2)> for F
where
    T1: Component + 'static,
    T2: Component + 'static,
    F: Send + Sync,
{
    type Execution = EffectCustomExecution<(T1, T2), Self>;

    fn into_condition(self) -> Self::Execution {
        EffectCustomExecution {
            f: self,
            marker: PhantomData,
        }
    }
}
*/
#[cfg(test)]
mod tests {
    use super::*;
    use crate::actors::ActorBuilder;
    use crate::assets::{ActorDef, EffectDef};
    use crate::attribute;
    use crate::condition::AttributeCondition;
    use crate::context::EffectContext;
    use crate::prelude::*;
    use crate::{AttributesPlugin, ReflectAccessAttribute, init_attribute};
    use bevy::prelude::*;

    #[test]
    fn conditions_test() {
        let mut app = App::new();

        app.add_plugins(MinimalPlugins)
            .init_schedule(Update)
            .add_plugins(AssetPlugin::default())
            .add_plugins(AttributesPlugin)
            .add_plugins((init_attribute::<Health>, init_attribute::<MaxHealth>));

        app.add_systems(
            Startup,
            |mut ctx: EffectContext, mut actors: ResMut<Assets<ActorDef>>| {
                let actor = actors.add(
                    ActorBuilder::new()
                        .with::<Health>(10)
                        .with::<MaxHealth>(100)
                        .build(),
                );
                let effect = Effect::permanent()
                    .modify::<MaxHealth>(Mod::add(15), Who::Source)
                    .build();
                let actor_id = ctx.spawn_actor(&actor).id();

                ctx.apply_dynamic_effect_to_self(actor_id, effect);
            },
        );

        app.add_systems(PostUpdate, |query: Query<&Health>| {
            let health = query.single().unwrap();
            println!("Health: {}", health.current_value());
        });

        //app.add_systems(PostUpdate, global_low_health_system);

        app.add_systems(
            Update,
            |query: Query<(Entity, Has<LowHealthTag>), With<Health>>| {
                for (entity, tag) in query.iter() {
                    if tag {
                        println!("Low health tag found");
                    } else {
                        println!("NOT Low health");
                    }
                }
            },
        );

        app.update();
        app.update();
    }
}
