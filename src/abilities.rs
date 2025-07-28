use crate::ActorEntityMut;
use crate::assets::{AbilityDef};
use crate::attributes::Attribute;
use crate::mutator::EntityMutator;
use bevy::ecs::relationship::Relationship;
use bevy::ecs::world::CommandQueue;
use bevy::prelude::*;
use log::debug;
use std::any::type_name;
use std::marker::PhantomData;

pub type AbilityActivationFn = Box<dyn Fn(&mut ActorEntityMut, Commands) + Send + Sync>;

/// The entity that this effect is targeting.
#[derive(Component, Reflect, Debug)]
#[relationship(relationship_target = Abilities)]
pub struct AbilityOf(pub Entity);

/// All effects that are targeting this entity.
#[derive(Component, Reflect, Debug)]
#[relationship_target(relationship = AbilityOf, linked_spawn)]
pub struct Abilities(Vec<Entity>);

#[derive(Component)]
pub struct Ability(Handle<AbilityDef>);

#[derive(Event)]
pub struct TryActivateAbility {
    //pub tag_Requirement: PredicateCondition,
}

impl TryActivateAbility {
    pub fn new() -> TryActivateAbility {
        TryActivateAbility{
            
        }
    }
    pub fn with_tag<T>(mut self) {
        
    }
    pub fn by_def(mut self) {
        
    }
}

struct TagRequirement<T> {
    phantom_data: PhantomData<T>,
}

#[derive(Component)]
pub struct AbilityCooldown(pub Timer);

pub struct GrantAbilityCommand {
    pub handle: Handle<AbilityDef>,
}

impl EntityCommand for GrantAbilityCommand {
    fn apply(self, mut entity: EntityWorldMut) -> () {
        error!("Apply abilities");

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
    cost: Box<dyn Fn(&mut ActorEntityMut, bool) -> bool + Send + Sync>,
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
            cost: Box::new(|_: &mut ActorEntityMut, _: bool| true),
            activation_fn: Box::new(|_, _| {
                warn!("Ability activation not implemented!");
            }),
        }
    }

    pub fn with_cost<C: Attribute>(mut self, cost: f64) -> Self {
        let cost_fn = move |entity_mut: &mut ActorEntityMut, commit: bool| {
            let Some(mut attribute) = entity_mut.get_mut::<C>() else {
                debug!(
                    "Actor [{}] does not have attribute [{}] to apply cost to!",
                    entity_mut.entity(),
                    type_name::<C>()
                );
                return false;
            };
            let cost_applied_value = attribute.current_value() - cost;

            if commit {
                let new_val = attribute.base_value() - cost;
                attribute.set_base_value(new_val);
            }

            cost_applied_value >= 0.0
        };

        self.cost = Box::new(cost_fn);
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
        function: impl Fn(&mut ActorEntityMut, Commands) + Send + Sync + 'static,
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
            cost_fn: self.cost,
            activation_fn: self.activation_fn,
        }
    }
}

pub fn try_activate_ability_observer(
    trigger: Trigger<TryActivateAbility>,
    mut actors: Query<
        ActorEntityMut,
        (
            Without<Ability>,
            Without<AbilityCooldown>,
            Without<AbilityOf>,
        ),
    >,
    mut query: Query<(&Ability, &mut AbilityCooldown, &AbilityOf)>,
    ability_assets: Res<Assets<AbilityDef>>,
    commands: Commands,
) -> Result<(), BevyError> {
    let (ability, mut ability_cooldown, parent) = query.get_mut(trigger.target())?;

    if !ability_cooldown.0.finished() {
        debug!("Ability on cooldown!");
        return Ok(());
    }

    let ability_spec = ability_assets
        .get(&ability.0.clone())
        .ok_or("No ability asset.")?;

    let mut actor_entity_mut = actors.get_mut(parent.get())?;

    let can_activate = (ability_spec.cost_fn)(&mut actor_entity_mut, false);
    if !can_activate {
        debug!("Insufficient resources to activate ability!");
        return Ok(());
    }

    // Commit the cost and reset the cooldown
    (ability_spec.cost_fn)(&mut actor_entity_mut, true);
    ability_cooldown.0.reset();

    // Activate the ability
    (ability_spec.activation_fn)(&mut actor_entity_mut, commands);

    debug!("Ability activated!");
    Ok(())
}
