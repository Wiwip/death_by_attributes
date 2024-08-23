use crate::effect::{GameEffect, GameEffectContainer};
use crate::modifiers::{MetaModifier, ScalarModifier};
use bevy::ecs::component::{Components, Tick};
use bevy::prelude::*;
use bevy::reflect::{ReflectFromPtr, ReflectMut, ReflectRef, TypeRegistryArc};
use std::any::TypeId;

pub trait GameAttributeMarker {}

#[derive(Reflect)]
pub struct GameAttribute {
    pub base_value: f32,
    pub current_value: f32,
}
