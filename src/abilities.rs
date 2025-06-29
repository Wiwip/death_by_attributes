use crate::ActorEntityMut;
use crate::attributes::AttributeComponent;
use crate::effects::Effect;
use bevy::ecs::component::Mutable;
use bevy::ecs::world::CommandQueue;
use bevy::prelude::*;
use log::debug;
use std::any::type_name;
use bevy::ecs::relationship::Relationship;

pub type AbilityActivationFn = fn(ActorEntityMut, Commands);
pub type AbilityCostFn = fn(&ActorEntityMut) -> bool;

/// The entity that this effect is targeting.
#[derive(Component, Reflect, Debug)]
#[relationship(relationship_target = Abilities)]
pub struct AbilityOf(pub Entity);

/// All effects that are targeting this entity.
#[derive(Component, Reflect, Debug)]
#[relationship_target(relationship = AbilityOf, linked_spawn)]
pub struct Abilities(Vec<Entity>);

#[derive(Component)]
pub struct GameAbility;

#[derive(Event)]
pub struct TryActivateAbility;

#[derive(Component)]
pub struct AbilityActivation {
    activation_fn: Box<dyn Fn(ActorEntityMut, Commands) + Send + Sync>,
}

#[derive(Component)]
pub struct AbilityCost {
    cost_fn: Box<dyn Fn(&mut ActorEntityMut, bool) -> bool + Send + Sync>,
}

#[derive(Component)]
pub struct AbilityCooldown(pub Timer);

pub enum GameEffectTarget {
    Source,
    Target,
}

#[derive(Default)]
pub struct GameAbilitySpec {
    pub applied_effects: Vec<(GameEffectTarget, Effect)>,
    //pub cost: Option<AbilityCostFn>,
    pub cooldown: Timer,
    pub ability_activation: Option<AbilityActivationFn>,
}

pub struct AbilityBuilder {
    ability_entity: Entity,
    queue: CommandQueue,
}

impl AbilityBuilder {
    pub fn new(ability: Entity, actor: Entity) -> AbilityBuilder {
        let mut queue = CommandQueue::default();
        queue.push(move |world: &mut World| {
            world.entity_mut(actor).add_related::<AbilityOf>(&[ability]);
        });
        Self {
            ability_entity: ability,
            queue,
        }
    }

    pub fn with_cost<C: Component<Mutability = Mutable> + AttributeComponent>(
        mut self,
        cost: f64,
    ) -> Self {
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
        self.queue.push(move |world: &mut World| {
            world.entity_mut(self.ability_entity).insert(AbilityCost {
                cost_fn: Box::new(cost_fn),
            });
        });
        self
    }

    pub fn with_cooldown(mut self, seconds: f32) -> Self {
        self.queue.push(move |world: &mut World| {
            world
                .entity_mut(self.ability_entity)
                .insert(AbilityCooldown(Timer::from_seconds(
                    seconds,
                    TimerMode::Once,
                )));
        });
        self
    }

    pub fn with_activation(mut self, function: AbilityActivationFn) -> Self {
        self.queue.push(move |world: &mut World| {
            world
                .entity_mut(self.ability_entity)
                .insert(AbilityActivation {
                    activation_fn: Box::new(function),
                });
        });
        self
    }

    pub fn build(mut self, commands: &mut Commands) {
        self.queue.push(move |world: &mut World| {
            world
                .entity_mut(self.ability_entity)
                .observe(try_activate_ability_observer);
        });

        commands.append(&mut self.queue);
    }
}

fn try_activate_ability_observer(
    trigger: Trigger<TryActivateAbility>,
    mut actors: Query<ActorEntityMut>,
    mut query: Query<(
        &AbilityActivation,
        &AbilityCost,
        &mut AbilityCooldown,
        &AbilityOf,
    )>,
    commands: Commands,
) {
    let Ok((ability_activation, ability_cost, mut ability_cooldown, parent)) =
        query.get_mut(trigger.target())
    else {
        debug!("Ability triggered on non-ability entity!");
        return;
    };

    if !ability_cooldown.0.finished() {
        debug!("Ability on cooldown!");
        return;
    }

    let Ok(mut actor_entity_mut) = actors.get_mut(parent.get()) else {
        debug!("Ability triggered on non-actor entity!");
        return;
    };

    let can_activate = (ability_cost.cost_fn)(&mut actor_entity_mut, false);
    if !can_activate {
        debug!("Insufficient resources to activate ability!");
        return;
    }

    // Commit the cost and reset the cooldown
    (ability_cost.cost_fn)(&mut actor_entity_mut, true);
    ability_cooldown.0.reset();

    // Activate the ability
    (ability_activation.activation_fn)(actor_entity_mut, commands);

    debug!("Ability activated!");
}
