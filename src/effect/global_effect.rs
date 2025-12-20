use std::any::type_name;
use crate::actors::{Actor, ActorBuilder};
use crate::assets::EffectDef;
use crate::context::EffectContext;
use bevy::prelude::*;

pub struct GlobalEffectPlugin;

impl Plugin for GlobalEffectPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(PreStartup, spawn_global_actor);
        app.insert_resource(GlobalEffects(vec![]));

        app.add_observer(observe_spawned_actor);
    }
}

#[derive(Resource, Deref, DerefMut)]
pub struct GlobalEffects(Vec<Handle<EffectDef>>);

#[derive(Component, Clone, Copy)]
pub struct GlobalActor;

#[derive(Component, Clone, Copy)]
pub struct GlobalEffect;

pub fn spawn_global_actor(mut commands: Commands, mut ctx: EffectContext) {
    let global_actor = commands.spawn(GlobalActor).id();

    let actor = ActorBuilder::new()
        .name("Global Effect Actor")
        .build();

    let actor_handle = ctx.add_actor(actor);
    ctx.insert_actor(global_actor, &actor_handle);
}

/// When an actor is spawned, ensures that the global effects apply to the actor.
fn observe_spawned_actor(trigger: On<Add, Actor>, mut ctx: EffectContext) {
    ctx.spawn_global_effects(trigger.entity);
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::ability::AbilityBuilder;
    use crate::actors::{Actor, ActorBuilder};
    use crate::assets::{AbilityDef, ActorDef};
    use crate::condition::AttributeCondition;
    use crate::context::EffectContext;
    use crate::init_attribute;
    use crate::prelude::*;
    use bevy::ecs::system::RunSystemOnce;
    use bevy::prelude::*;

    attribute!(TestAttribute, f64);

    #[derive(Component, Copy, Clone, Debug, PartialEq)]
    struct ConditionTag;

    fn prepare_actor(
        mut actor_assets: ResMut<Assets<ActorDef>>,
        mut ctx: EffectContext,
        registry: Registry,
    ) {
        let actor_template = actor_assets.add(
            ActorBuilder::new()
                .name("TestActor".into())
                .with::<TestAttribute>(0.0)
                .grant_ability(&registry.ability(TEST_ABILITY_TOKEN))
                .with_effect(&registry.effect(CONDITION_EFFECT))
                .build(),
        );
        ctx.spawn_actor(actor_template);
    }

    fn prepare_effects(mut registry: RegistryMut) {
        registry.add_effect(
            TEST_EFFECT,
            Effect::permanent()
                .name("Increase Effect".into())
                .modify::<TestAttribute>(200.0, ModOp::Add, Who::Target)
                .build(),
        );

        registry.add_effect(
            CONDITION_EFFECT,
            Effect::permanent()
                .name("Condition Effect".into())
                .activate_while(AttributeCondition::<TestAttribute>::target(150.0..))
                .insert(ConditionTag)
                .build(),
        );
    }

    pub const TEST_EFFECT: EffectToken = EffectToken::new_static("test.test");
    pub const CONDITION_EFFECT: EffectToken = EffectToken::new_static("test.condition");

    fn prepare_abilities(mut abilities: RegistryMut) {
        abilities.add_ability(TEST_ABILITY_TOKEN, fireball_ability());
    }

    pub const TEST_ABILITY_TOKEN: AbilityToken = AbilityToken::new_static("test.test");

    pub fn fireball_ability() -> AbilityDef {
        AbilityBuilder::new()
            .with_name("Test Ability".into())
            .build()
    }

    #[test]
    fn test_attribute_condition() {
        let mut app = App::new();
        app.add_plugins((MinimalPlugins, AssetPlugin::default(), AttributesPlugin));
        app.add_plugins(init_attribute::<TestAttribute>);

        app.add_systems(
            Startup,
            (prepare_effects, prepare_abilities, prepare_actor).chain(),
        );

        app.update();

        let mut query = app.world_mut().query::<(Entity, &Actor, &TestAttribute)>();
        let actor_id = query.single(app.world()).unwrap().0;

        // Check that the effect is inactive
        let mut query = app
            .world_mut()
            .query::<(Entity, &Effect, &ConditionTag, Option<&EffectInactive>)>();
        let opt_inactive = query.single(app.world()).unwrap().3;
        assert!(opt_inactive.is_some());

        app.world_mut()
            .run_system_once(move |mut ctx: EffectContext, registry: Registry| {
                ctx.apply_effect_to_self(actor_id, &registry.effect(TEST_EFFECT));
            })
            .unwrap();

        app.update();
        app.update();

        // Check that the effect is active
        let mut query = app
            .world_mut()
            .query::<(Entity, &Effect, &ConditionTag, Option<&EffectInactive>)>();
        let opt_inactive = query.single(app.world()).unwrap().3;
        assert!(opt_inactive.is_none());
    }
}
