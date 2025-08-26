use crate::assets::EffectDef;
use crate::attribute;
use crate::attributes::I16F16Proxy;
use crate::effect::timing::EffectDuration;
use crate::prelude::{Attribute, AttributeTypeId, Effect};
use crate::ReflectAccessAttribute;
use bevy::prelude::*;
use fixed::prelude::ToFixed;
use fixed::types::{I16F16, U32F0};
use serde::Serialize;

pub enum EffectStackingPolicy {
    None, // Each effect is independently added to the entity
    Add { count: U32F0, max_stack: U32F0 },
    RefreshDuration, // The effect overrides previous applications
                     //RefreshDurationWithOverflow, // The effect overrides previous applications
}

//attribute!(EffectIntensity, U16F16);

#[derive(bevy::prelude::Component, Clone, Copy, bevy::prelude::Reflect, Debug, Serialize)]
#[reflect(AccessAttribute)]
pub struct EffectIntensity {
    #[reflect(remote=I16F16Proxy)]
    base_value: I16F16,
    #[reflect(remote=I16F16Proxy)]
    current_value: I16F16,
}
impl Attribute for EffectIntensity {
    type Property = I16F16;

    fn new<T: ToFixed + Copy>(value: T) -> Self {
        Self {
            base_value: value.to_fixed(),
            current_value: value.to_fixed(),
        }
    }
    fn base_value(&self) -> Self::Property {
        self.base_value
    }
    fn set_base_value(&mut self, value: Self::Property) {
        self.base_value = value;
    }
    fn current_value(&self) -> Self::Property {
        self.current_value
    }
    fn set_current_value(&mut self, value: Self::Property) {
        self.current_value = value;
    }
    fn attribute_type_id() -> AttributeTypeId {
        AttributeTypeId::of::<Self>()
    }
}

impl Default for EffectIntensity {
    fn default() -> Self {
        EffectIntensity::new(1.0)
    }
}

attribute!(Stacks, U32F0);

impl Default for Stacks {
    fn default() -> Self {
        Stacks::new(1) // By default, a new effect has 1 stack
    }
}

impl Stacks {
    /// Applies the appropriate stacking policy to an effect
    pub fn apply_stacking_policy(
        policy: &EffectStackingPolicy,
        effect_entity: Entity,
        stacks: &mut Query<&mut Stacks, With<Effect>>,
        durations: &mut Query<&mut EffectDuration, With<Effect>>,
    ) {
        match policy {
            EffectStackingPolicy::Add { count, max_stack } => {
                // Apply additive stacking, increasing stack count up to max
                if let Ok(mut stack_count) = stacks.get_mut(effect_entity) {
                    let mut base_stacks = stack_count.base_value();
                    base_stacks += count;
                    stack_count
                        .set_base_value(base_stacks.clamp(1.to_fixed(), max_stack.to_fixed()));
                    stack_count
                        .set_current_value(base_stacks.clamp(1.to_fixed(), max_stack.to_fixed()));
                } else {
                    error!(
                        "Failed to find component Stacks for entity: {:?}",
                        effect_entity
                    );
                }
            }
            EffectStackingPolicy::RefreshDuration => {
                // Reset duration for overridden effects
                if let Ok(mut duration) = durations.get_mut(effect_entity) {
                    duration.reset();
                } else {
                    error!(
                        "Failed to find component EffectApplication for entity: {:?}",
                        effect_entity
                    );
                }
            }
            EffectStackingPolicy::None => {
                error!(
                    "Effect stacking should not be triggered for effect entity {:?} with incompatible policy (None)",
                    effect_entity
                );
            }
        }
    }
}

#[derive(Event)]
pub struct NotifyAddStackEvent {
    pub effect_entity: Entity,
    pub handle: Handle<EffectDef>,
}

pub(crate) fn read_add_stack_event(
    mut event_reader: EventReader<NotifyAddStackEvent>,
    mut stacks: Query<&mut Stacks, With<Effect>>,
    mut applications: Query<&mut EffectDuration, With<Effect>>,
    effect_assets: Res<Assets<EffectDef>>,
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

        Stacks::apply_stacking_policy(
            &effect_definition.stacking_policy,
            ev.effect_entity,
            &mut stacks,
            &mut applications,
        );
    }
}
