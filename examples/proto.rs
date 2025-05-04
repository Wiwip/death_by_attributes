use attributes_macro::Attribute;
use bevy::MinimalPlugins;
use bevy::app::App;
use bevy::ecs::query::QueryEntityError;
use bevy::prelude::Component;
use bevy::prelude::Deref;
use bevy::prelude::Reflect;
use bevy::prelude::*;
use death_by_attributes::abilities::GameAbilityContainer;
use death_by_attributes::attributes::AttributeComponent;
use death_by_attributes::attributes::AttributeDef;
use death_by_attributes::effects::{Effect, EffectDuration, EffectPeriodicTimer};
use death_by_attributes::evaluators::FixedEvaluator;
use death_by_attributes::mutator::ModType::Additive;
use death_by_attributes::mutator::{Mutator, StoredMutator};
use death_by_attributes::{AttributeEntityMut, DeathByAttributesPlugin, attribute};

attribute!(Health);

fn main() {
    App::new()
        .add_plugins(MinimalPlugins)
        //.add_plugins(DeathByAttributesPlugin)
        .add_systems(Startup, setup)
        .add_systems(Update, update)
        .run();
}

fn setup(mut command: Commands) {
    //let attribute = AttributeMut::<Health>::default();
    let evaluator = FixedEvaluator::new(12.0, Additive);
    let mutator = Mutator::new::<Health>(evaluator);

    let effect = Effect {
        modifiers: vec![StoredMutator::new(mutator)],
    };

    let id = command.spawn((Health::new(100.0),)).id();

    command.spawn((ChildOf(id), effect));
}

fn update(
    mut query: Query<(AttributeEntityMut, &Children), Without<Effect>>,
    effects: Query<&Effect>,
) {
    for (mut entity, children) in query.iter_mut() {
        ///let attribute = AttributeMut::<Health>::default();
        /*let health = attribute
        .get_mut(&mut entity)
        .expect("Component does not exist");*/
        let eff: Vec<&Effect> = children
            .iter()
            .filter_map(|e| match effects.get(e) {
                Ok(result) => Some(result),
                Err(_) => None,
            })
            .collect();

        println!("Effect: {}", eff.iter().count());
        for effect in eff {
            println!("Modifiers: {:?}", effect.modifiers.iter().count());
            for modifier in effect.modifiers.iter() {
                //let aggregator = modifier.0.get_aggregator(&mut entity).unwrap();
                //let _ = modifier.0.apply_from_aggregator(&mut entity, aggregator);
            }
        }

        /*let health = attribute
            .get_mut(&mut entity)
            .expect("Component does not exist");
        println!("Health: {}", health.base_value);*/
    }
}
