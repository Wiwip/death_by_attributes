use bevy::prelude::Component;

pub enum EffectStackingPolicy {
    None, // Each effect is independently added to the entity
    Add {
        count: u32,
        max_stack: u32,
    },
    Override, // The effect overrides previous applications
}

#[derive(Component)]
pub struct Stacks(pub u32);

impl Default for Stacks {
    fn default() -> Self {
        Self(1) // By default, a new effect has 1 stack
    }
}