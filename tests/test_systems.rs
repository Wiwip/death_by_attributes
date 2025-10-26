use bevy::ecs::system::RunSystemOnce;
use bevy::prelude::*;
use root_attribute::attributes::Value;
use root_attribute::context::EffectContext;
use root_attribute::init_attribute;
use root_attribute::prelude::*;

attribute!(TestA);

/*
#[test]
fn test_update_base_values() {
    let mut app = App::new();
    app.add_systems(Update, trigger_periodic_effects);
    app.add_observer(on_duration_effect_applied);
    app.add_observer(on_base_value_changed);
    app.insert_resource(CachedMutations::default());

    let player = app.world_mut().spawn(TestA::new(0.0)).id();
    let effect = app.world_mut().spawn_empty().id();

    EffectBuilder::new(player, effect)
        .with_permanent_duration()
        .with_periodic_application(1.0)
        .mutate_by_scalar::<TestA>(42.0, Additive)
        .apply(&mut app.world_mut().commands());

    let value = app.world().get::<TestA>(player).unwrap();
    assert_eq!(0.0, value.base_value);

    app.world_mut().flush();

    let mut timer = app
        .world_mut()
        .get_mut::<EffectPeriodicTimer>(effect)
        .unwrap();
    timer.0.tick(Duration::from_secs(10));

    let value = app.world().get::<TestA>(player).unwrap();
    assert_eq!(0.0, value.base_value);

    app.update();

    let value = app.world().get::<TestA>(player).unwrap();
    assert_eq!(42.0, value.base_value);

    app.update();

    let value = app.world().get::<TestA>(player).unwrap();
    assert_eq!(84.0, value.base_value);
}
*/

#[test]
fn test_update_current_values() {
    let mut app = App::new();
    app.add_plugins((MinimalPlugins, AssetPlugin::default(), AttributesPlugin));
    app.add_plugins(init_attribute::<TestA>);

    let entity = app.world_mut().spawn(TestA::new(0.0)).id();

    // Create a dynamic effect and apply it directly to the entity
    let _ = app
        .world_mut()
        .run_system_once(move |mut ctx: EffectContext| {
            let effect_def = EffectBuilder::new(EffectApplicationPolicy::Permanent)
                .modify::<TestA>(Value::lit(42.0), ModOp::Add, Who::Target, 1.0)
                .build();

            ctx.apply_dynamic_effect_to_self(entity, effect_def);
            println!("yay!");
        });

    app.update();
    app.update();

    let value = app.world().get::<TestA>(entity).unwrap();
    println!("{:?}", value);
    assert_eq!(0.0, value.base_value);
    assert_eq!(42.0, value.current_value);

    // Despawn the effect and confirm the attribute values
    //app.world_mut().despawn(effect);

    app.update();

    let value = app.world().get::<TestA>(entity).unwrap();
    assert_eq!(0.0, value.base_value);
    assert_eq!(0.0, value.current_value);
}
