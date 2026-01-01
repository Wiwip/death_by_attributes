use crate::ability::AbilityCooldown;
use crate::assets::AbilityDef;
use crate::attributes::Attribute;
use crate::condition::{AttributeCondition, BoxCondition};
use crate::expression::Expr;
use crate::inspector::pretty_type_name;
use crate::modifier::{AttributeCalculatorCached, ModOp, Modifier, Who};
use crate::mutator::EntityActions;
use crate::prelude::AttributeModifier;
use bevy::ecs::system::IntoObserverSystem;
use bevy::prelude::*;
use num_traits::{AsPrimitive, Num};

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
        let mutator = AttributeModifier::<T>::new(Expr::Lit(cost), ModOp::Sub, Who::Source);
        self.cost_mods.push(Box::new(mutator));

        let condition = AttributeCondition::<T>::source(cost..);
        self.cost_condition.push(BoxCondition::new(condition));
        self
    }

    pub fn with_cooldown(mut self, expr: impl Into<Expr<f64>>) -> Self {
        let val = expr.into();

        self.mutators.push(EntityActions::new(
            move |entity_commands: &mut EntityCommands| {
                entity_commands.try_insert(AbilityCooldown {
                    timer: Timer::from_seconds(0.0, TimerMode::Once),
                    value: val.clone(),
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
