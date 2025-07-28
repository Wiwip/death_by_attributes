use bevy::prelude::*;

pub type EntityMutatorFn = dyn Fn(&mut EntityCommands) + Send + Sync;

pub struct EntityMutator {
    pub func: Box<EntityMutatorFn>,
}

impl EntityMutator {
    pub fn new(func: impl Fn(&mut EntityCommands) + Send + Sync + 'static) -> Self {
        Self {
            func: Box::new(func),
        }
    }

    pub fn apply(&self, entity_commands: &mut EntityCommands) {
        (self.func)(entity_commands);
    }
}
