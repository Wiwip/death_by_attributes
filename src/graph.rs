use crate::prelude::*;
use bevy::ecs::system::SystemParam;
use bevy::prelude::*;
use petgraph::visit::Visitable;
use petgraph::visit::{GraphBase, IntoNeighbors};
use std::collections::HashSet;

/// Attributes are Components and inserted on Entities.
/// - Derived attributes could be used
/// Effects are spawned as child of entities
/// - Effects can modify the modifiers till now, such as an intensity metric or stacks
/// Modifiers are added to Effects as Vec<Mod>
/// - Modifiers must apply to an attribute

#[derive(Component, Reflect, Debug)]
pub enum NodeType {
    Actor,
    Effect,
    Modifier,
}

// Lightweight wrapper that implements petgraph traits
#[derive(SystemParam)]
pub struct QueryGraphAdapter<'w, 's> {
    dependencies: Query<'w, 's, (Entity, &'static AppliedEffects)>,
}

impl GraphBase for QueryGraphAdapter<'_, '_> {
    type NodeId = Entity;
    type EdgeId = (Entity, Entity);
}

impl IntoNeighbors for &QueryGraphAdapter<'_, '_> {
    type Neighbors = vec::IntoIter<Entity>;

    fn neighbors(self, node: Self::NodeId) -> Self::Neighbors {
        match self.dependencies.get(node) {
            Ok((_, effects)) => effects.iter().collect::<Vec<Entity>>().into_iter(),
            Err(_) => vec![].into_iter(), // No child entities
        }
    }
}

impl Visitable for QueryGraphAdapter<'_, '_> {
    type Map = HashSet<Entity>;

    fn visit_map(&self) -> Self::Map {
        HashSet::new()
    }

    fn reset_map(&self, map: &mut Self::Map) {
        map.clear();
    }
}

/*
#[cfg(test)]
mod tests {
    use super::*;
    use crate::actors::ActorBuilder;
    use crate::assets::ActorDef;
    use crate::context::EffectContext;
    use crate::{AttributesPlugin, ReflectAccessAttribute};
    use crate::{attribute, init_attribute};

    attribute!(Health, f64);
    attribute!(Mana, f64);
    attribute!(Strength, f64);

    #[derive(Resource)]
    struct Definitions {
        actor_def: Handle<ActorDef>,
    }

    #[test]
    fn test_simple_modifier() {
        let mut app = App::new();
        init_app(&mut app);

        app.add_systems(
            Startup,
            (
                create_test_actor,
                |mut ctx: EffectContext, def: Res<Definitions>| {
                    let test_actor = ctx.spawn_actor(&def.actor_def).id();

                    let effect = EffectBuilder::permanent()
                        .modify::<Health>(Mod::Add(10.0), Who::Source)
                        .modify::<Mana>(Mod::More(1.0), Who::Source)
                        .build();

                    ctx.apply_dynamic_effect_to_self(test_actor, effect);
                },
            )
                .chain(),
        );

        app.add_systems(Update, |actor: Query<(&Health, &Mana)>| {
            let (health, mana) = actor.single().unwrap();
            assert!((health.current_value - 110.0).abs() < f64::EPSILON);
            assert!((mana.current_value - 2000.0).abs() < f64::EPSILON);
        });

        app.update();
    }

    fn init_app(app: &mut App) {
        app.add_plugins(MinimalPlugins)
            .add_plugins(AssetPlugin::default())
            .add_plugins(AttributesPlugin)
            .add_plugins((
                init_attribute::<Health>,
                init_attribute::<Mana>,
                init_attribute::<Strength>,
            ));
    }

    fn create_test_actor(mut actor_assets: ResMut<Assets<ActorDef>>, mut commands: Commands) {
        let actor_template = actor_assets.add(
            ActorBuilder::new()
                .with::<Strength>(12.0)
                .with::<Health>(100.0)
                .with::<Mana>(1000.0)
                .build(),
        );
        commands.insert_resource(Definitions {
            actor_def: actor_template,
        });
    }
}
*/
