use crate::attributes::{AttributeDef, EditableAttribute};
use crate::{Editable, AttributeEntityMut};
use bevy::animation::AnimationEvaluationError;
use bevy::ecs::component::Mutable;
use bevy::platform::hash::Hashed;
use bevy::prelude::*;
use bevy::reflect::{TypeInfo, Typed};
use std::any::TypeId;
use std::fmt;
use std::fmt::{Debug, Formatter};
use std::iter::Sum;
use std::marker::PhantomData;
use std::ops::Add;
use crate::evaluators::AttributeModEvaluator;

pub type BoxEditableAttribute = Box<dyn EditableAttribute<Property = AttributeDef>>;

#[derive(Reflect, FromReflect)]
#[reflect(from_reflect = false)]
pub struct AttributeMod<P> {
    pub(crate) attribute_ref: P,
    pub(crate) magnitude: f32, // or an evaluator
    pub(crate) mod_type: ModType,
}

impl<P> AttributeMod<P>
where
    P: EditableAttribute,
{
    pub fn new(attribute_ref: P, magnitude: f32, mod_type: ModType) -> Self {
        Self {
            attribute_ref,
            magnitude,
            mod_type,
        }
    }
}

impl<P> Clone for AttributeMod<P>
where
    P: Clone,
{
    fn clone(&self) -> Self {
        Self {
            attribute_ref: self.attribute_ref.clone(),
            magnitude: self.magnitude,
            mod_type: self.mod_type,
        }
    }
}

#[derive(Clone)]
pub struct AttributeRef<C, A, F: Fn(&mut C) -> &mut A> {
    func: F,
    marker: PhantomData<(C, A)>,
    evaluator_id: Hashed<(TypeId, usize)>
}

impl<C: Typed, P, F: Fn(&mut C) -> &mut P + 'static> AttributeRef<C, P, F> {
    pub fn new_unchecked(func: F) -> Self {
        let field_index;
        if let TypeInfo::Struct(struct_info) = C::type_info() {
            field_index = struct_info
                .index_of("attribute")
                .expect("Field name should exist");
        } else if let TypeInfo::TupleStruct(struct_info) = C::type_info() {
            field_index = "attribute"
                .parse()
                .expect("Field name should be a valid tuple index");
            if field_index >= struct_info.field_len() {
                panic!("Field name should be a valid tuple index");
            }
        } else {
            panic!("Only structs are supported in `AnimatedField::new_unchecked`")
        }

        Self {
            func,
            marker: PhantomData,
            evaluator_id: Hashed::new((TypeId::of::<C>(), field_index)),
        }
    }
}

impl<C, A, F> EditableAttribute for AttributeRef<C, A, F>
where
    C: Component<Mutability = Mutable>,
    A: Editable + Clone + Sync + Debug,
    F: Fn(&mut C) -> &mut A + Send + Sync + 'static,
{
    type Property = A;

    fn get_mut<'a>(
        &self,
        entity: &'a mut AttributeEntityMut,
    ) -> Result<&'a mut A, AnimationEvaluationError> {
        let c = entity
            .get_mut::<C>()
            .ok_or_else(|| AnimationEvaluationError::ComponentNotPresent(TypeId::of::<C>()))?;

        Ok((self.func)(c.into_inner()))
    }

    fn evaluator_id(&self) -> Hashed<(TypeId, usize)> {
        self.evaluator_id
    }
}

/*

#[derive(Clone)]
pub struct ScalarModifier {
    pub target_attribute: TypeId,
    pub magnitude: f32,
    pub mod_type: ModifierType,
}

impl ScalarModifier {
    pub fn additive<M: Component + GameAttributeMarker>(magnitude: f32) -> ScalarModifier {
        ScalarModifier {
            target_attribute: TypeId::of::<M>(),
            magnitude,
            mod_type: ModifierType::Additive,
        }
    }

    pub fn multi<M: Component + GameAttributeMarker>(magnitude: f32) -> ScalarModifier {
        ScalarModifier {
            target_attribute: TypeId::of::<M>(),
            magnitude,
            mod_type: ModifierType::Multiplicative,
        }
    }

    pub fn overrule<M: Component + GameAttributeMarker>(magnitude: f32) -> ScalarModifier {
        ScalarModifier {
            target_attribute: TypeId::of::<M>(),
            magnitude,
            mod_type: ModifierType::Overrule,
        }
    }
}

impl Debug for ScalarModifier {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self.mod_type {
            ModifierType::Additive => {
                write!(f, "Add:{:.1}", self.magnitude)
            }
            ModifierType::Multiplicative => {
                write!(f, "Mul:{:.1}", self.magnitude)
            }
            ModifierType::Overrule => {
                write!(f, "Over:{:.1}", self.magnitude)
            }
        }
    }
}

#[derive(Clone, Copy)]
pub struct MetaModifier {
    pub target_attribute: TypeId,
    pub magnitude_attribute: TypeId,
    pub mod_type: ModifierType,
}

impl Debug for MetaModifier {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "MetaMod")
    }
}
*/
#[derive(Debug, Clone, Copy, Reflect)]
pub enum ModType {
    Additive,
    Multiplicative,
    Overrule,
}

#[derive(Default, Debug, Clone, Copy)]
pub struct ModAggregator {
    pub additive: f32,
    pub multiplicative: f32,
    pub overrule: Option<f32>,
}

impl ModAggregator {
    pub fn get_current_value(&self, base_value: f32) -> f32 {
        match self.overrule {
            None => (base_value + self.additive) * (1.0 + self.multiplicative),
            Some(value) => value,
        }
    }
}

impl From<&AttributeMod<f32>> for ModAggregator {
    fn from(value: &AttributeMod<f32>) -> Self {
        let mut aggregator = ModAggregator::default();

        match value.mod_type {
            ModType::Additive => aggregator.additive += value.magnitude,
            ModType::Multiplicative => aggregator.multiplicative += value.magnitude,
            ModType::Overrule => aggregator.overrule = Some(value.magnitude),
        }
        aggregator
    }
}

impl Add for &ModAggregator {
    type Output = ModAggregator;

    fn add(self, rhs: Self) -> Self::Output {
        ModAggregator {
            additive: self.additive + rhs.additive,
            multiplicative: self.additive + rhs.additive,
            overrule: self.overrule.or(rhs.overrule),
        }
    }
}

impl Sum for ModAggregator {
    fn sum<I: Iterator<Item = Self>>(iter: I) -> Self {
        iter.fold(
            Self {
                additive: 0.0,
                multiplicative: 0.0,
                overrule: None,
            },
            |a, b| Self {
                additive: a.additive + b.additive,
                multiplicative: a.multiplicative + b.multiplicative,
                overrule: a.overrule.or(b.overrule),
            },
        )
    }
}
