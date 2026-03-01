use std::any::{Any, TypeId};
use crate::actors::SpawnActorCommand;
use crate::assets::{ActorDef, EffectDef};
use crate::effect::global_effect::{GlobalActor, GlobalEffects};
use crate::effect::{ApplyEffectEvent, EffectTargeting};
use bevy::ecs::system::SystemParam;
use bevy::prelude::*;
use bevy::reflect::TypeRegistryArc;
use express_it::context::{Accessor, ReadContext, ScopeId, WriteContext};
use express_it::expr::ExpressionError;
use crate::{AppAttributeBindings, AttributesMut, AttributesRef};
use crate::modifier::Who;

#[derive(SystemParam)]
pub struct EffectContext<'w, 's> {
    commands: Commands<'w, 's>,
    global_actor: Query<'w, 's, Entity, With<GlobalActor>>,
    global_effects: ResMut<'w, GlobalEffects>,
    effects: ResMut<'w, Assets<EffectDef>>,
    actors: ResMut<'w, Assets<ActorDef>>,
}

impl<'s, 'w> EffectContext<'w, 's> {
    pub fn add_effect(&mut self, effect: EffectDef) -> Handle<EffectDef> {
        self.effects.add(effect)
    }

    pub fn apply_effect_to_target(
        &mut self,
        target: Entity,
        source: Entity,
        handle: &Handle<EffectDef>,
    ) {
        self.commands.trigger(ApplyEffectEvent {
            entity: target,
            targeting: EffectTargeting::new(source, target),
            handle: handle.clone(),
        });
    }

    pub fn apply_effect_to_self(&mut self, source: Entity, handle: &Handle<EffectDef>) {
        self.apply_effect_to_target(source, source, handle);
    }

    pub fn apply_dynamic_effect_to_target(
        &mut self,
        target: Entity,
        source: Entity,
        effect: EffectDef,
    ) -> Handle<EffectDef> {
        let handle = self.effects.add(effect);

        self.commands.trigger(ApplyEffectEvent {
            entity: target,
            targeting: EffectTargeting::new(source, target),
            handle: handle.clone(),
        });
        handle
    }

    pub fn apply_dynamic_effect_to_self(
        &mut self,
        source: Entity,
        effect: EffectDef,
    ) -> Handle<EffectDef> {
        self.apply_dynamic_effect_to_target(source, source, effect)
    }

    pub fn add_actor(&mut self, actor: ActorDef) -> Handle<ActorDef> {
        self.actors.add(actor)
    }

    pub fn spawn_actor(&mut self, handle: &Handle<ActorDef>) -> EntityCommands<'_> {
        let mut entity_commands = self.commands.spawn_empty();
        entity_commands.queue(SpawnActorCommand {
            handle: handle.clone(),
        });
        entity_commands
    }

    pub fn add_spawn_actor(&mut self, actor: ActorDef) -> EntityCommands<'_> {
        let handle = self.actors.add(actor);
        self.spawn_actor(&handle)
    }

    pub fn insert_actor(&mut self, entity: Entity, handle: &Handle<ActorDef>) {
        self.commands.entity(entity).queue(SpawnActorCommand {
            handle: handle.clone(),
        });
    }

    pub fn add_global_effect(&mut self, handle: Handle<EffectDef>) {
        self.global_effects.push(handle);
    }

    /// Gets or create the global effect actor.
    /// Global effects are attached to this actor and applied to all existing actors.
    /// This actor can serve as a game state tracker, and the effects can depend on its attributes.
    pub fn get_global_actor(&mut self) -> Entity {
        self.global_actor.single().unwrap()
    }

    pub fn spawn_global_effects(&mut self, target_actor: Entity) {
        let global_actor = self.get_global_actor();
        let effects: Vec<_> = self.global_effects.clone();

        for handle in effects.iter() {
            self.apply_effect_to_target(target_actor, global_actor, &handle);
        }
    }
}


pub struct BevyContextMut<'w, 's> {
    pub source_actor: &'w mut AttributesMut<'w, 's>,
    pub target_actor: Option<&'w mut AttributesMut<'w, 's>>,
    pub owner: &'w mut AttributesMut<'w, 's>,

    pub type_registry: TypeRegistryArc,
    pub type_bindings: AppAttributeBindings,
}

impl<'w, 's> BevyContextMut<'w, 's> {
    pub fn entity(&self, who: Who) -> Entity {
        match who {
            Who::Target => match &self.target_actor {
                None => self.source_actor.id(),
                Some(actor) => actor.id(),
            },
            Who::Source => self.source_actor.id(),
            Who::Owner => self.owner.id(),
        }
    }

    pub fn attribute_mut(&mut self, who: Who) -> &mut AttributesMut<'w, 's> {
        match who {
            Who::Target => {
                if let Some(target) = self.target_actor.as_deref_mut() {
                    target
                } else {
                    self.source_actor
                }
            }
            Who::Source => self.source_actor,
            Who::Owner => self.owner,
        }
    }
}

impl WriteContext for BevyContextMut<'_, '_> {
    fn write(
        &mut self,
        access: &dyn Accessor,
        value: Box<dyn Any + Send + Sync>,
    ) -> Result<(), ExpressionError> {
        let who: Who = access
            .scope()
            .0
            .try_into()
            .map_err(|_| ExpressionError::InvalidPath)?;

        let any_to_reflect = {
            let bindings = self.type_bindings.internal.read().unwrap();
            *bindings.convert.get(&access.path()).unwrap()
        };

        let type_id = *self
            .type_bindings
            .internal
            .read()
            .unwrap()
            .map
            .get(&access.path())
            .expect("InvalidPath");

        let arc_type_registry = self.type_registry.clone();
        let registry = arc_type_registry.read();
        let type_registration = registry
            .get(type_id)
            .expect("Failed to get type registration");
        let reflect_component = type_registration
            .data::<ReflectComponent>()
            .expect("No reflect access attribute found");

        let actor = self.attribute_mut(who);
        let mut dyn_reflect = reflect_component.reflect_mut(actor).ok_or_else(|| {
            let short_name = type_registration
                .type_info()
                .type_path_table()
                .short_path()
                .to_string();
            debug!(
                "Requested type not present on actor: {}/{}",
                short_name, who
            );
            ExpressionError::FailedReflect("The entity has no component the requested type.".into())
        })?;

        let dyn_partial_reflect = dyn_reflect.reflect_path_mut("base_value").map_err(|err| {
            ExpressionError::FailedReflect(format!("Invalid reflect path: {err}").into())
        })?;

        let value_reflect = any_to_reflect(&*value).ok_or_else(|| {
            ExpressionError::FailedReflect("Type mismatch while converting expression value".into())
        })?;

        dyn_partial_reflect.apply(value_reflect);
        Ok(())
    }
}

pub struct BevyContext<'w, 's> {
    pub source_actor: &'w AttributesRef<'w, 's>,
    pub target_actor: &'w AttributesRef<'w, 's>,
    pub owner: &'w AttributesRef<'w, 's>,

    pub type_registry: TypeRegistryArc,
    pub type_bindings: AppAttributeBindings,
}

impl BevyContext<'_, '_> {
    pub fn entity(&self, who: Who) -> Entity {
        match who {
            Who::Target => self.target_actor.id(),
            Who::Source => self.source_actor.id(),
            Who::Owner => self.owner.id(),
        }
    }

    pub fn attribute_ref(&self, who: Who) -> &AttributesRef<'_, '_> {
        match who {
            Who::Target => self.target_actor,
            Who::Source => self.source_actor,
            Who::Owner => self.owner,
        }
    }
}

impl ReadContext for BevyContext<'_, '_> {
    fn get_any(&self, access: &dyn Accessor) -> Result<&dyn Any, ExpressionError> {
        let who: Who = access
            .scope()
            .0
            .try_into()
            .map_err(|_| ExpressionError::InvalidPath)?;
        let actor = self.attribute_ref(who);

        let type_id = *self
            .type_bindings
            .internal
            .read()
            .unwrap()
            .map
            .get(&access.path())
            .ok_or(ExpressionError::InvalidPath)?;

        let registry = self.type_registry.read();
        let Some(type_registration) = registry.get(type_id) else {
            return Err(ExpressionError::FailedReflect(
                "Failed to get type registration".into(),
            ));
        };
        let Some(reflect_component) = type_registration.data::<ReflectComponent>() else {
            return Err(ExpressionError::FailedReflect(
                "No reflect access attribute found".into(),
            ));
        };
        let Some(dyn_reflect) = reflect_component.reflect(actor) else {
            let short_name = type_registration
                .type_info()
                .type_path_table()
                .short_path()
                .to_string();
            debug!("Requested type not present on actor: {}", short_name);
            return Err(ExpressionError::FailedReflect(
                "The entity has no component the requested type.".into(),
            ));
        };

        let read_base = {
            let bindings = self.type_bindings.internal.read().unwrap();
            bindings.base_ids.contains(&access.path())
        };

        let field = if read_base { "base_value" } else { "current_value" };

        let dyn_partial_reflect = dyn_reflect.reflect_path(field).map_err(|err| {
            ExpressionError::FailedReflect(format!("Invalid reflect path: {err}").into())
        })?;

        let dyn_path_reflect = dyn_partial_reflect.try_as_reflect().ok_or_else(|| {
            ExpressionError::FailedReflect(
                "Reflect value does not support further reflection".into(),
            )
        })?;

        Ok(dyn_path_reflect)
    }

    fn get_any_component(
        &self,
        scope: ScopeId,
        type_id: TypeId,
    ) -> std::result::Result<&dyn Any, ExpressionError> {
        let who: Who = scope
            .0
            .try_into()
            .map_err(|_| ExpressionError::InvalidPath)?;
        let actor = self.attribute_ref(who);

        let registry = self.type_registry.read();
        let Some(type_registration) = registry.get(type_id) else {
            return Err(ExpressionError::FailedReflect(
                "Failed to get type registration".into(),
            ));
        };
        let Some(reflect_component) = type_registration.data::<ReflectComponent>() else {
            return Err(ExpressionError::FailedReflect(
                "No reflect access attribute found".into(),
            ));
        };
        let Some(dyn_reflect) = reflect_component.reflect(actor) else {
            return Err(ExpressionError::FailedReflect(
                "The entity has no component the requested type.".into(),
            ));
        };

        Ok(dyn_reflect.as_any()) // Component (Attribute), not a primitive
    }
}