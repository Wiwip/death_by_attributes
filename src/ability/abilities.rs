use crate::ability::{Ability, AbilityCooldown};
use crate::assets::AbilityDef;
use crate::attributes::{Attribute, Value};
use crate::condition::{AttributeCondition, BoxCondition};
use crate::modifier::{Modifier, Who};
use crate::mutator::EntityActions;
use crate::prelude::{AttributeModifier, ModOp};
use bevy::asset::{Assets, Handle};
use bevy::ecs::system::IntoObserverSystem;
use bevy::ecs::world::CommandQueue;
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
            actor_assets.get(&self.handle).unwrap()
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
    mutators: Vec<EntityActions>,
    cost_condition: Vec<BoxCondition>,
    cost_mods: Vec<Box<dyn Modifier>>,
}

impl AbilityBuilder {
    pub fn new() -> AbilityBuilder {
        Self {
            name: "Ability".to_string(),
            mutators: Default::default(),
            cost_condition: vec![],
            cost_mods: vec![],
        }
    }

    pub fn with_cost<C: Attribute>(mut self, cost: C::Property) -> Self {
        let mutator = AttributeModifier::<C>::new(Value::lit(cost), ModOp::Sub, Who::Source, 1.0);
        self.cost_mods.push(Box::new(mutator));

        let condition = AttributeCondition::<C>::source(cost..);
        self.cost_condition.push(BoxCondition::new(condition));
        self
    }

    pub fn with_cooldown(mut self, seconds: f32) -> Self {
        self.mutators.push(EntityActions::new(
            move |entity_commands: &mut EntityCommands| {
                entity_commands.insert(AbilityCooldown(Timer::from_seconds(
                    seconds,
                    TimerMode::Once,
                )));
            },
        ));
        self
    }

    pub fn add_observer<E: EntityEvent, B: Bundle, M>(mut self, observer: impl IntoObserverSystem<E, B, M> + Clone + Send + Sync + 'static) -> Self {
        self.mutators.push(EntityActions::new(
            move |entity_commands: &mut EntityCommands| {
                println!("Adding observer: {:?}", entity_commands.id());
                entity_commands.observe(observer.clone());
            },
        ));
        self
    }

    pub fn with_tag<T: Component + Default>(mut self) -> Self {
        self.mutators.push(EntityActions::new(
            move |entity_commands: &mut EntityCommands| {
                entity_commands.insert(T::default());
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
            cost: self.cost_condition,
            condition: vec![],
            cost_effects: self.cost_mods,
        }
    }
}
