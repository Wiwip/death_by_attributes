use death_by_attributes::abilities::GameAbilityContainer;
use death_by_attributes::attributes::AttributeDef;
use bevy::prelude::DerefMut;
use bevy::prelude::Deref;
use bevy::prelude::Reflect;
use bevy::prelude::Component;
use attributes_macro::Attribute;
use death_by_attributes::attributes::AttributeMut;
use std::any::TypeId;
use std::collections::HashMap;
use std::hash::Hash;
use bevy::asset::uuid::Uuid;
use bevy::platform::hash::Hashed;
use bevy::utils::{default, PreHashMap};
use death_by_attributes::{attribute, attribute_mut};
use death_by_attributes::attributes::StoredAttribute;
use death_by_attributes::effects::{ApplicableEffect, Effect, EffectHandle, StoredEffect};
use death_by_attributes::mutator::StoredMutator;

attribute!(Health);

fn main(){
    

}