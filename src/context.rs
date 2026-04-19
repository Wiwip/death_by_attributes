use crate::actors::SpawnActorCommand;
use crate::assets::{ActorDef, EffectDef};
use crate::effect::global_effect::{GlobalActor, GlobalEffects};
use crate::effect::{ApplyEffectEvent, EffectTargeting};
use crate::modifier::{AbilitySubject, EffectSubject};
use crate::registry::Registry;
use crate::registry::actor_registry::ActorToken;
use crate::{AppAttributeBindings, AttributesMut, AttributesRef};
use bevy::ecs::system::SystemParam;
use bevy::prelude::*;
use bevy::reflect::TypeRegistryArc;
use express_it::context::{Path, ReadContext, WriteContext};
use express_it::expr::{ExprSchema, ExpressionError};
use std::any::Any;

#[derive(SystemParam)]
pub struct EffectContext<'w, 's> {
    commands: Commands<'w, 's>,
    global_actor: Query<'w, 's, Entity, With<GlobalActor>>,
    global_effects: ResMut<'w, GlobalEffects>,
    registry: Registry<'w>,
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

    pub fn spawn_actor(&mut self, token: &ActorToken) -> EntityCommands<'_> {
        let handle = self.registry.actor(&token);

        let mut entity_commands = self.commands.spawn_empty();
        entity_commands.queue(SpawnActorCommand { handle });
        entity_commands
    }

    pub fn spawn_actor_from_handle(&mut self, handle: &Handle<ActorDef>) -> EntityCommands<'_> {
        let mut entity_commands = self.commands.spawn_empty();
        entity_commands.queue(SpawnActorCommand {
            handle: handle.clone(),
        });
        entity_commands
    }

    pub fn add_spawn_actor(&mut self, actor: ActorDef) -> EntityCommands<'_> {
        let handle = self.actors.add(actor);
        self.spawn_actor_from_handle(&handle)
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

pub struct EffectExprContextMut<'w, 's> {
    pub source_actor: &'w mut AttributesMut<'w, 's>,
    pub target_actor: Option<&'w mut AttributesMut<'w, 's>>,
    pub owner: &'w mut AttributesMut<'w, 's>,

    pub type_registry: TypeRegistryArc,
    pub type_bindings: AppAttributeBindings,
}

impl<'w, 's> EffectExprContextMut<'w, 's> {
    pub fn entity(&self, who: EffectSubject) -> Entity {
        match who {
            EffectSubject::Target => match &self.target_actor {
                None => self.source_actor.id(),
                Some(actor) => actor.id(),
            },
            EffectSubject::Source => self.source_actor.id(),
            EffectSubject::Effect => self.owner.id(),
        }
    }

    pub fn attribute_mut(&mut self, who: EffectSubject) -> &mut AttributesMut<'w, 's> {
        match who {
            EffectSubject::Target => {
                if let Some(target) = self.target_actor.as_deref_mut() {
                    target
                } else {
                    self.source_actor
                }
            }
            EffectSubject::Source => self.source_actor,
            EffectSubject::Effect => self.owner,
        }
    }
}

impl WriteContext for EffectExprContextMut<'_, '_> {
    fn write(
        &mut self,
        path: &Path,
        value: Box<dyn Any + Send + Sync>,
    ) -> Result<(), ExpressionError> {
        let who = EffectSubject::try_from(path)
            .map_err(|_| ExpressionError::InvalidPath(path.0.clone()))?;

        let (_, component, _) = split_path(&*path.0).expect("Wrong path in reflect path");

        let any_to_reflect = {
            let bindings = self.type_bindings.internal.read().unwrap();
            *bindings.convert.get(component).unwrap()
        };

        let reflect_component = {
            let registry_bindings = self.type_registry.read();
            let Some(type_registration) = registry_bindings.get_with_short_type_path(component) else {
                return Err(ExpressionError::FailedReflect(
                    "Failed to get type registration".into(),
                ));
            };

            type_registration
                .data::<ReflectComponent>()
                .expect("No reflect access attribute found")
                .clone()
        };

        let actor = self.attribute_mut(who);
        let mut dyn_reflect = reflect_component.reflect_mut(actor).ok_or_else(|| {
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

pub struct ActorExprSchema;
impl ExprSchema for ActorExprSchema {
    type Context<'w, 's>
        = ActorExprContext<'w, 's>
    where
        's: 'w;
}

pub struct ActorExprContext<'w, 's> {
    pub actor_context: &'w AttributesRef<'w, 's>,

    pub type_registry: TypeRegistryArc,
}

impl ReadContext for ActorExprContext<'_, '_> {
    fn get_any(&self, path: &Path) -> Result<&dyn Any, ExpressionError> {
        reflect_path(path, self.actor_context, &self.type_registry)
    }
}

pub struct EffectExprSchema;
impl ExprSchema for EffectExprSchema {
    type Context<'w, 's>
        = EffectExprContext<'w, 's>
    where
        's: 'w;
}

pub struct EffectExprContext<'w, 's> {
    pub source_actor: &'w AttributesRef<'w, 's>,
    pub target_actor: &'w AttributesRef<'w, 's>,
    pub effect_holder: &'w AttributesRef<'w, 's>,

    pub type_registry: TypeRegistryArc,
}

impl EffectExprContext<'_, '_> {
    pub fn attribute_ref(&self, who: EffectSubject) -> &AttributesRef<'_, '_> {
        match who {
            EffectSubject::Target => self.target_actor,
            EffectSubject::Source => self.source_actor,
            EffectSubject::Effect => self.effect_holder,
        }
    }
}

impl ReadContext for EffectExprContext<'_, '_> {
    fn get_any(&self, path: &Path) -> Result<&dyn Any, ExpressionError> {
        let who = EffectSubject::try_from(path)
            .map_err(|_| ExpressionError::InvalidPath(path.0.clone()))?;
        let actor = self.attribute_ref(who);

        reflect_path(path, actor, &self.type_registry)
    }
}

pub struct AbilityExprSchema;
impl ExprSchema for AbilityExprSchema {
    type Context<'w, 's>
        = AbilityExprContext<'w, 's>
    where
        's: 'w;
}

pub struct AbilityExprContext<'w, 's> {
    pub caster_ref: &'w AttributesRef<'w, 's>,
    pub ability_ref: &'w AttributesRef<'w, 's>,
    pub target_ref: &'w AttributesRef<'w, 's>,

    pub type_registry: TypeRegistryArc,
}

impl AbilityExprContext<'_, '_> {
    pub fn attribute_ref(&self, who: AbilitySubject) -> &AttributesRef<'_, '_> {
        match who {
            AbilitySubject::Ability => self.ability_ref,
            AbilitySubject::Caster => self.caster_ref,
            AbilitySubject::Target => self.target_ref,
        }
    }
}

impl ReadContext for AbilityExprContext<'_, '_> {
    fn get_any(&self, path: &Path) -> std::result::Result<&dyn Any, ExpressionError> {
        let who = AbilitySubject::try_from(path)
            .map_err(|_| ExpressionError::InvalidPath(path.0.clone()))?;

        let actor = self.attribute_ref(who);

        reflect_path(path, actor, &self.type_registry)
    }
}

pub fn split_path(path: &str) -> Result<(&str, &str, Option<&str>), &'static str> {
    let (subject, rest) = path.split_once('.').ok_or("missing . separator")?;
    let Some((component, value)) = rest.split_once('.') else {
        return Ok((subject, rest, None));
    };
    Ok((subject, component, Some(value)))
}

fn reflect_path<'a>(
    path: &Path,
    actor: &'a AttributesRef,
    type_registry: &'a TypeRegistryArc,
) -> Result<&'a dyn Any, ExpressionError> {
    let (_, component, value) = split_path(&*path.0).expect("Wrong path in reflect path");

    let registry_bindings = type_registry.read();
    let Some(type_registration) = registry_bindings.get_with_short_type_path(component) else {
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
        trace!("Requested type not present on actor: {}", short_name);
        return Err(ExpressionError::FailedReflect(
            "The entity has no component the requested type.".into(),
        ));
    };

    let Some(value) = value else {
        return Ok(dyn_reflect.as_any());
    };

    let dyn_partial_reflect = dyn_reflect.reflect_path(value).map_err(|err| {
        ExpressionError::FailedReflect(format!("Invalid reflect path: {err}").into())
    })?;

    let dyn_path_reflect = dyn_partial_reflect.try_as_reflect().ok_or_else(|| {
        ExpressionError::FailedReflect("Reflect value does not support further reflection".into())
    })?;

    Ok(dyn_path_reflect.as_any())
}
