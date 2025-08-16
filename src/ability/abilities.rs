use crate::AttributesMut;
use crate::ability::{Ability, AbilityActivationFn, AbilityCooldown};
use crate::assets::AbilityDef;
use crate::attributes::Attribute;
use crate::condition::{AttributeCondition, BoxCondition};
use crate::modifier::{Mutator, Who};
use crate::mutator::EntityMutator;
use crate::prelude::{AttributeModifier, Mod};
use bevy::asset::{Assets, Handle};
use bevy::ecs::world::CommandQueue;
use bevy::log::warn;
use bevy::prelude::*;

pub struct GrantAbilityCommand {
    pub handle: Handle<AbilityDef>,
}

impl EntityCommand for GrantAbilityCommand {
    fn apply(self, mut entity: EntityWorldMut) -> () {
        let ability_def = {
            // Create a temporary scope to borrow the world
            let world = entity.world();
            let actor_assets = world.resource::<Assets<AbilityDef>>();
            actor_assets.get(&self.handle).unwrap() //.clone() // Clone if needed
        }; // World borrow ends here

        let mut queue = {
            let mut queue = CommandQueue::default();
            let mut commands = Commands::new(&mut queue, entity.world());

            // Apply mutators
            for mutator in &ability_def.mutators {
                let mut entity_commands = commands.entity(entity.id());
                (mutator.func)(&mut entity_commands);
            }

            queue
        };

        entity.insert((Ability(self.handle), Name::new(ability_def.name.clone())));

        // Apply the commands
        entity.world_scope(|world| {
            world.commands().append(&mut queue);
            world.flush();
        });
    }
}

pub struct AbilityBuilder {
    name: String,
    mutators: Vec<EntityMutator>,
    cost_condition: Vec<BoxCondition>,
    cost_mods: Vec<Box<dyn Mutator>>,
    activation_fn: AbilityActivationFn,
}

impl AbilityBuilder {
    pub fn with_name(mut self, name: String) -> Self {
        self.name = name;
        self
    }
}

impl AbilityBuilder {
    pub fn new() -> AbilityBuilder {
        Self {
            name: "Ability".to_string(),
            mutators: Default::default(),
            cost_condition: vec![],
            cost_mods: vec![],
            activation_fn: Box::new(|_, _| {
                warn!("Ability activation not implemented!");
            }),
        }
    }

    pub fn with_cost<C: Attribute>(mut self, cost: f64) -> Self {
        let mutator = AttributeModifier::<C>::new(Mod::Add(-cost), Who::Source, 1.0);
        self.cost_mods.push(Box::new(mutator));

        let condition = AttributeCondition::source::<C>(cost..);
        self.cost_condition.push(BoxCondition::new(condition));
        self
    }

    pub fn with_cooldown(mut self, seconds: f32) -> Self {
        self.mutators.push(EntityMutator::new(
            move |entity_commands: &mut EntityCommands| {
                entity_commands.insert(AbilityCooldown(Timer::from_seconds(
                    seconds,
                    TimerMode::Once,
                )));
            },
        ));
        self
    }

    pub fn with_activation(
        mut self,
        function: impl Fn(&mut AttributesMut, &mut Commands) + Send + Sync + 'static,
    ) -> Self {
        self.activation_fn = Box::new(function);
        self
    }

    pub fn with_tag<T: Component + Default>(mut self) -> Self {
        self.mutators.push(EntityMutator::new(
            move |entity_commands: &mut EntityCommands| {
                entity_commands.insert(T::default());
            },
        ));
        self
    }

    pub fn build(self) -> AbilityDef {
        AbilityDef {
            name: self.name,
            description: "".to_string(),
            mutators: self.mutators,
            cost: self.cost_condition,
            cost_effects: self.cost_mods,
            activation_fn: self.activation_fn,
        }
    }
}
