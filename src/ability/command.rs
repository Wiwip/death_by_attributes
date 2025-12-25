use crate::ability::{Ability, AbilityOf};
use crate::assets::AbilityDef;
use bevy::asset::{Assets, Handle};
use bevy::ecs::world::CommandQueue;
use bevy::prelude::*;

pub struct GrantAbilityCommand {
    pub parent: Entity,
    pub handle: Handle<AbilityDef>,
}

impl EntityCommand for GrantAbilityCommand {
    fn apply(self, mut entity: EntityWorldMut) -> () {
        let id = entity.id();
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

        entity.insert((
            Ability(self.handle),
            AbilityOf(id),
            Name::new(ability_def.name.clone()),
        ));

        // Apply the commands
        entity.world_scope(|world| {
            world.commands().append(&mut queue);
            world.flush();
        });
    }
}
