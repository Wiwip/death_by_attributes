use crate::ability::{Ability, AbilityCooldown};
use crate::assets::AbilityDef;
use crate::attributes::{Attribute, IntoValue, Lit, Value};
use crate::condition::{AttributeCondition, BoxCondition};
use crate::inspector::pretty_type_name;
use crate::modifier::{Modifier, Who};
use crate::mutator::EntityActions;
use crate::prelude::{AttributeCalculatorCached, AttributeModifier, ModOp};
use bevy::asset::{Assets, Handle};
use bevy::ecs::system::IntoObserverSystem;
use bevy::ecs::world::CommandQueue;
use bevy::prelude::*;
use num_traits::{AsPrimitive, Num};
use std::sync::Arc;

pub struct GrantAbilityCommand {
    pub parent: Entity,
    pub handle: Handle<AbilityDef>,
}

impl EntityCommand for GrantAbilityCommand {
    fn apply(self, mut entity: EntityWorldMut) -> () {
        let ability_def = {
            // Create a temporary scope to borrow the world
            let world = entity.world();
            let actor_assets = world.resource::<Assets<AbilityDef>>();
            actor_assets.get(&self.handle).unwrap()
        }; // World borrow ends here

        let mut queue = {
            let mut queue = CommandQueue::default();
            let mut commands = Commands::new(&mut queue, entity.world());

            // Apply mutators
            for mutator in &ability_def.mutators {
                let mut entity_commands = commands.entity(entity.id());
                mutator.apply(&mut entity_commands);
            }

            for observer in &ability_def.observers {
                let mut entity_commands = commands.entity(self.parent);
                observer.apply(&mut entity_commands);
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
    mutators: Vec<EntityActions>,
    triggers: Vec<EntityActions>,
    cost_condition: Vec<BoxCondition>,
    cost_mods: Vec<Box<dyn Modifier>>,
}

impl AbilityBuilder {
    pub fn new() -> AbilityBuilder {
        Self {
            name: "Ability".to_string(),
            mutators: Default::default(),
            triggers: vec![],
            cost_condition: vec![],
            cost_mods: vec![],
        }
    }

    pub fn with<T: Attribute>(
        mut self,
        value: impl Num + AsPrimitive<T::Property> + Copy + Send + Sync + 'static,
    ) -> AbilityBuilder {
        self.mutators.push(EntityActions::new(
            move |entity_commands: &mut EntityCommands| {
                entity_commands.insert((T::new(value), AttributeCalculatorCached::<T>::default()));
            },
        ));
        self
    }

    pub fn with_cost<T: Attribute>(mut self, cost: T::Property) -> Self {
        let mutator =
            AttributeModifier::<T>::new(Value(Arc::new(Lit(cost))), ModOp::Sub, Who::Source);
        self.cost_mods.push(Box::new(mutator));

        let condition = AttributeCondition::<T>::source(cost..);
        self.cost_condition.push(BoxCondition::new(condition));
        self
    }

    pub fn with_cooldown(
        mut self,
        value: impl IntoValue<Out = f64> + Send + Sync + Clone + 'static,
    ) -> Self {
        self.mutators.push(EntityActions::new(
            move |entity_commands: &mut EntityCommands| {
                entity_commands.try_insert(AbilityCooldown {
                    timer: Timer::from_seconds(0.0, TimerMode::Once),
                    value: value.clone().into_value(),
                });
            },
        ));
        self
    }

    pub fn add_execution<E: EntityEvent, B: Bundle, M>(
        mut self,
        observer: impl IntoObserverSystem<E, B, M> + Clone + Send + Sync + 'static,
    ) -> Self {
        self.mutators.push(EntityActions::new(
            move |entity_commands: &mut EntityCommands| {
                entity_commands.observe(observer.clone());
            },
        ));
        self
    }

    pub fn add_trigger<E: EntityEvent, B: Bundle, M>(
        mut self,
        observer: impl IntoObserverSystem<E, B, M> + Clone + Send + Sync + 'static,
    ) -> Self {
        self.triggers.push(EntityActions::new(
            move |actor_commands: &mut EntityCommands| {
                let mut observer = Observer::new(observer.clone());
                observer.watch_entity(actor_commands.id());

                actor_commands.commands().spawn((
                    observer,
                    Name::new(format!("On<{}>", pretty_type_name::<E>())),
                ));
            },
        ));
        self
    }

    pub fn with_tag<T: Component + Default>(mut self) -> Self {
        self.mutators.push(EntityActions::new(
            move |entity_commands: &mut EntityCommands| {
                entity_commands.try_insert(T::default());
            },
        ));
        self
    }

    pub fn with_name(mut self, name: String) -> Self {
        self.name = name;
        self
    }

    pub fn build(self) -> AbilityDef {
        AbilityDef {
            name: self.name,
            description: "".to_string(),
            mutators: self.mutators,
            observers: self.triggers,
            cost: self.cost_condition,
            execution_conditions: vec![],
            cost_modifiers: self.cost_mods,
        }
    }
}
