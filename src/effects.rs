use crate::ActorEntityMut;
use crate::assets::GameEffect;
use crate::attributes::Attribute;
use crate::conditions::{AttributeCondition, ErasedCondition};
use crate::modifiers::{
    AttributeModifier, ModAggregator, ModTarget, ModType, ModifierFn, ModifierOf, ModifierRef,
    Mutator,
};
use crate::stacks::Stacks;
use crate::stacks::{EffectStackingPolicy, apply_stacking_policy};
use bevy::ecs::component::Mutable;
use bevy::ecs::system::IntoObserverSystem;
use bevy::prelude::Name;
use bevy::prelude::TimerMode::Repeating;
use bevy::prelude::*;
use std::fmt::{Debug, Formatter};
use std::ops::Range;
use std::time::Duration;

pub enum EffectStatus {
    Active,
    Inactive,
}

#[derive(Clone, Copy, Debug, Reflect)]
pub enum Target {
    SelfEntity,
    TargetEntity,
}

#[derive(Event)]
pub struct OnEffectStatusChangeEvent(pub EffectStatus);

#[derive(Component, Debug, Default)]
#[component(storage = "SparseSet")]
pub struct EffectInactive;

#[derive(Component, Debug, Default, Deref)]
#[require(Stacks)]
pub struct Effect(pub Handle<GameEffect>);

/// Who created this effect?
#[derive(Component, Reflect, Debug)]
#[relationship(relationship_target = EffectSources)]
pub struct EffectSource(pub Entity);

/// All effects originating from this entity
#[derive(Component, Reflect, Debug)]
#[relationship_target(relationship = EffectSource, linked_spawn)]
pub struct EffectSources(Vec<Entity>);

/// All effects targeting this entity
#[derive(Component, Reflect, Debug)]
#[relationship(relationship_target = EffectTargetedBy)]
pub struct EffectTarget(pub Entity);

/// All effects that are targeting this entity.
#[derive(Component, Reflect, Debug)]
#[relationship_target(relationship = EffectTarget, linked_spawn)]
pub struct EffectTargetedBy(Vec<Entity>);

#[derive(Component, Reflect, Deref, DerefMut, Clone)]
pub struct EffectPeriodicTimer(pub Timer);

impl EffectPeriodicTimer {
    pub fn new(seconds: f32) -> Self {
        Self(Timer::from_seconds(seconds, Repeating))
    }
}

/// Represents the duration policy of an effect in a system.
///
/// This enum is used to define how long an effect stays active.
/// It provides three variants to specify the effect's duration:
///
/// - `Instant`: The effect takes place immediately and does not persist.
/// - `Permanent`: The effect is applied indefinitely without expiration.
/// - `Temporary(Duration)`: The effect is active for a specified period of time, defined by a `Duration`.
pub enum EffectDurationPolicy {
    Instant,
    Permanent,
    Temporary(Duration),
}

#[derive(Component, Clone)]
pub struct EffectDuration(pub Timer);

impl Debug for EffectDuration {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}", self.0)
    }
}

pub struct EffectBuilder {
    effect_entity_commands: Vec<Box<ModifierFn>>,
    effects: Vec<Box<dyn Mutator>>,
    modifiers: Vec<Box<dyn Mutator>>,
    duration: EffectDurationPolicy,
    period: Option<EffectPeriodicTimer>,
    conditions: Vec<ErasedCondition>,
    stacking_policy: EffectStackingPolicy,
}

impl EffectBuilder {
    pub fn new() -> GameEffectDurationBuilder {
        GameEffectDurationBuilder {
            effect_builder: EffectBuilder {
                effect_entity_commands: vec![],
                effects: vec![],
                modifiers: vec![],
                duration: EffectDurationPolicy::Instant,
                period: None,
                conditions: vec![],
                stacking_policy: EffectStackingPolicy::None,
            },
        }
    }

    pub fn modify_by_scalar<T: Attribute + Component<Mutability=Mutable>>(
        mut self,
        magnitude: f64,
        mod_type: ModType,
        mod_target: ModTarget,
    ) -> Self {
        self.effect_entity_commands.push(Box::new(
            move |entity_mut: &mut EntityCommands, _: Entity| {
                entity_mut.insert(ModAggregator::<T>::default());
            },
        ));

        self.modifiers.push(Box::new(AttributeModifier::<T>::new(
            magnitude, mod_type, mod_target,
        )));
        self
    }

    /// Spawns an observer watching the actor's attributes on the modifier entity.
    /// When OnValueChanged is triggered, it takes the current value of the attribute,
    /// it applies the scaling factor and updates the modifier's value to the new value.  
    pub fn modify_by_ref<T, S>(
        mut self,
        scaling_factor: f64,
        mod_type: ModType,
        mod_target: ModTarget,
    ) -> Self
    where
        T: Attribute + Component<Mutability=Mutable>,
        S: Attribute + Component<Mutability=Mutable>,
    {
        self.effect_entity_commands.push(Box::new(
            move |effect_entity: &mut EntityCommands, _: Entity| {
                effect_entity.insert(ModAggregator::<T>::default());
            },
        ));

        self.modifiers.push(Box::new(ModifierRef::<T, S>::new(
            scaling_factor,
            mod_type,
            mod_target,
        )));
        self
    }

    pub fn with_trigger<E: Event, B: Bundle, M>(
        mut self,
        _observer: impl IntoObserverSystem<E, B, M>,
    ) -> Self {
        self.effect_entity_commands.push(Box::new(
            move |_effect_entity: &mut EntityCommands, _: Entity| {
                //effect_entity.insert(Condition::<T>::default());
            },
        ));
        self
    }

    pub fn with_condition<T: Attribute + Component<Mutability=Mutable>>(
        mut self,
        condition_check: Range<f64>,
    ) -> Self {
        let condition = AttributeCondition::<T, Range<f64>>::new(condition_check);
        self.conditions.push(ErasedCondition::new(condition));
        self
    }

    /*pub fn with_tag_requirement<T: AttributeComponent + Component<Mutability = Mutable>>(
        mut self,
        condition_check: fn(f64) -> bool,
    ) -> Self {
        self.effects.push(Box::new(Condition::<T> {
            _target: Default::default(),
            condition_fn: condition_check,
        }));
        self
    }*/

    /*pub fn with_condition_complex<T: AttributeComponent + Component<Mutability = Mutable>>(
        mut self,
        condition_check: fn(f64) -> bool,
    ) -> Self {


        self.effects.push(Box::new(Condition::<T> {
            _target: Default::default(),
            condition_fn: condition_check,
        }));
        self
    }*/

    pub fn with_name(mut self, name: String) -> Self {
        self.effect_entity_commands.push(Box::new(
            move |effect_entity: &mut EntityCommands, _: Entity| {
                effect_entity.insert(Name::new(name.clone()));
            },
        ));
        self
    }

    pub fn with_bundle(mut self, bundle: impl Bundle + Copy) -> Self {
        self.effect_entity_commands.push(Box::new(
            move |effect_entity: &mut EntityCommands, _: Entity| {
                effect_entity.insert(bundle);
            },
        ));
        self
    }

    pub fn with_stacking_policy(mut self, policy: EffectStackingPolicy) -> Self {
        self.stacking_policy = policy;
        self
    }

    pub fn build(self) -> GameEffect {
        GameEffect {
            effect_fn: self.effect_entity_commands,
            effect_modifiers: self.effects,
            modifiers: self.modifiers,
            duration: self.duration,
            period: self.period,
            conditions: self.conditions,
            stacking_policy: self.stacking_policy,
        }
    }
}

/// A builder structure for constructing a `GameEffectPeriod`.
///
/// This struct is used in conjunction with the `EffectBuilder` to provide
/// an interface for configuring and creating game effect periods.
pub struct GameEffectPeriodBuilder {
    effect_builder: EffectBuilder,
}

impl GameEffectPeriodBuilder {
    /// Configures the `EffectBuilder` to apply its effect periodically over the specified time interval.
    ///
    /// # Parameters
    /// - `seconds`: The duration in seconds for the periodic application of the effect. This determines
    ///   how often the effect will be applied repeatedly.
    ///
    /// # Returns
    /// A modified `EffectBuilder` instance with the periodic application behaviour configured.
    /// # Panics
    /// This function does not explicitly handle invalid values (e.g., negative seconds). Ensure that the
    /// `seconds` parameter is non-negative to prevent unexpected behavior.
    pub fn with_periodic_application(mut self, seconds: f32) -> EffectBuilder {
        self.effect_builder.period =
            Some(EffectPeriodicTimer(Timer::from_seconds(seconds, Repeating)));
        self.effect_builder
    }
    /// Returns the `EffectBuilder` associated with the current instance.
    ///
    /// This method provides direct access to the `EffectBuilder` to allow
    /// for continuous application or further chaining of effects within
    /// the system.
    ///
    /// # Returns
    ///
    /// * `EffectBuilder` - The `EffectBuilder` instance contained within
    ///   the current instance, enabling additional operations or effect handling.
    pub fn with_continuous_application(self) -> EffectBuilder {
        self.effect_builder
    }
}

pub struct GameEffectDurationBuilder {
    effect_builder: EffectBuilder,
}

impl GameEffectDurationBuilder {
    pub fn with_instant_application(self) -> EffectBuilder {
        self.effect_builder
    }
    pub fn with_duration(mut self, seconds: f32) -> GameEffectPeriodBuilder {
        self.effect_builder.duration =
            EffectDurationPolicy::Temporary(Duration::from_secs_f32(seconds));
        GameEffectPeriodBuilder {
            effect_builder: self.effect_builder,
        }
    }
    pub fn with_permanent_duration(mut self) -> GameEffectPeriodBuilder {
        self.effect_builder.duration = EffectDurationPolicy::Permanent;
        GameEffectPeriodBuilder {
            effect_builder: self.effect_builder,
        }
    }
}

#[derive(Event)]
pub struct ApplyEffectEvent {
    pub target: Entity,
    pub source: Entity,
    pub handle: Handle<GameEffect>,
}

impl ApplyEffectEvent {
    fn apply_instant_effect(
        &self,
        _commands: &mut Commands,
        actors: &mut Query<(Option<&EffectTargetedBy>, ActorEntityMut), Without<Effect>>,
        effect: &GameEffect,
    ) {
        debug!("Applying instant effect to {}", self.target);

        effect
            .modifiers
            .iter()
            .for_each(|modifier| match modifier.origin() {
                ModTarget::Target => {
                    let (_, mut target) = actors.get_mut(self.target).unwrap();
                    modifier.apply(&mut target);
                }
                ModTarget::Source => {
                    let (_, mut source) = actors.get_mut(self.source).unwrap();
                    modifier.apply(&mut source);
                }
            })
    }

    fn apply_temporary_effect(
        &self,
        mut commands: &mut Commands,
        effect: &GameEffect,
        actors: &mut Query<(Option<&EffectTargetedBy>, ActorEntityMut), Without<Effect>>,
        effects: &mut Query<&Effect>,
        add_stack_event: &mut EventWriter<OnAddStackEffect>,
    ) {
        debug!("Applying duration effect to {}", self.target);

        // We want to know whether an effect with the same handle already exists on the actor
        let (optional_effects, _) = actors.get_mut(self.target).unwrap();
        let effects_on_actor = match optional_effects {
            None => {
                vec![]
            }
            Some(effects_on_actor) => {
                let effects = effects_on_actor.iter().filter_map(|effect_entity| {
                    let other_effect = effects.get(effect_entity).unwrap();
                    if other_effect.0.id() == self.handle.id() {
                        Some(effect_entity)
                    } else {
                        None
                    }
                });
                effects.collect::<Vec<_>>()
            }
        };

        match effect.stacking_policy {
            EffectStackingPolicy::None => {
                // Continue spawning effect
                debug!("Stacking policy is None");
            }
            EffectStackingPolicy::Add { .. } | EffectStackingPolicy::Override => {
                debug!("Stacking policy is Add or Override");
                if effects_on_actor.len() > 0 {
                    debug!("Effect already exists on actor. Adding stacks per definition.");
                    add_stack_event.write(OnAddStackEffect {
                        effect_entity: *effects_on_actor.first().unwrap(),
                        handle: self.handle.clone(),
                    });
                    return;
                }
            }
        }

        let mut effect_commands = commands.spawn_empty();
        let effect_entity = effect_commands.id();
        for effect_fn in &effect.effect_fn {
            effect_fn(&mut effect_commands, self.target);
        }

        // Spawns the effect entity
        effect_commands.insert((
            EffectTarget(self.target),
            EffectSource(self.source),
            Effect(self.handle.clone()),
        ));

        match effect.duration {
            EffectDurationPolicy::Temporary(duration) => {
                effect_commands.insert(EffectDuration(Timer::new(duration, TimerMode::Once)));
            }
            _ => {}
        }

        // Add the periodic effect component
        if let Some(period) = &effect.period {
            effect_commands.insert(period.clone());
        }

        // Prepare entity commands
        for effect_mod in &effect.effect_modifiers {
            let (_, target) = actors.get_mut(self.target).unwrap();
            effect_mod.spawn(&mut commands, target.as_readonly());
        }

        // Spawn effect modifiers
        effect
            .modifiers
            .iter()
            .for_each(|modifier| match modifier.origin() {
                ModTarget::Target => {
                    let (_, target) = actors.get_mut(self.target).unwrap();
                    let mod_entity = modifier.spawn(commands, target.as_readonly());
                    commands
                        .entity(mod_entity)
                        .insert(ModifierOf(effect_entity));
                }
                ModTarget::Source => {
                    let (_, source) = actors.get_mut(self.source).unwrap();
                    let mod_entity = modifier.spawn(commands, source.as_readonly());
                    commands
                        .entity(mod_entity)
                        .insert(ModifierOf(effect_entity));
                }
            })
    }
}

pub(crate) fn observe_effect_application(
    trigger: Trigger<ApplyEffectEvent>,
    mut actors: Query<(Option<&EffectTargetedBy>, ActorEntityMut), Without<Effect>>,
    mut effects: Query<&Effect>,
    effect_assets: Res<Assets<GameEffect>>,
    mut event_writer: EventWriter<OnAddStackEffect>,
    mut commands: Commands,
) {
    assert_ne!(Entity::PLACEHOLDER, trigger.target);
    assert_ne!(Entity::PLACEHOLDER, trigger.source);

    let effect = effect_assets.get(&trigger.handle).unwrap();

    match effect.duration {
        EffectDurationPolicy::Instant => {
            trigger
                .event()
                .apply_instant_effect(&mut commands, &mut actors, effect);
        }
        EffectDurationPolicy::Permanent | EffectDurationPolicy::Temporary(_) => {
            trigger.event().apply_temporary_effect(
                &mut commands,
                effect,
                &mut actors,
                &mut effects,
                &mut event_writer,
            );
        }
    }
}

#[derive(Event)]
pub struct OnAddStackEffect {
    pub effect_entity: Entity,
    pub handle: Handle<GameEffect>,
}

pub(crate) fn read_add_stack_event(
    mut event_reader: EventReader<OnAddStackEffect>,
    mut stacks: Query<&mut Stacks, With<Effect>>,
    mut durations: Query<&mut EffectDuration, With<Effect>>,
    effect_assets: Res<Assets<GameEffect>>,
) {
    for ev in event_reader.read() {
        let effect_definition = match effect_assets.get(&ev.handle) {
            Some(effect) => effect,
            None => {
                panic!(
                    "Failed to find effect definition for handle: {:?}",
                    ev.handle
                );
            }
        };

        apply_stacking_policy(
            &effect_definition.stacking_policy,
            ev.effect_entity,
            &mut stacks,
            &mut durations,
        );
    }
}
