use crate::attributes::{AttributeComponent, AttributeDef};
use crate::evaluators::MutatorEvaluator;
use crate::{AttributeEntityMut, Editable};
use bevy::animation::AnimationEvaluationError;
use bevy::ecs::component::Mutable;
use bevy::prelude::*;
use std::any::{TypeId, type_name};
use std::fmt::{Debug, Formatter};
use std::iter::Sum;
use std::marker::PhantomData;
use std::ops::{Add, AddAssign, DerefMut, Neg, SubAssign};

pub trait EvaluateMutator: Debug + Send + Sync + 'static {
    fn clone_value(&self) -> Box<dyn EvaluateMutator>;
    fn apply_mutator(&self, entity_mut: AttributeEntityMut);
    fn apply_aggregator(&self, entity_mut: AttributeEntityMut, aggregator: ModAggregator);
    fn update_current_value(&self, entity_mut: AttributeEntityMut, aggregator: ModAggregator);

    fn target(&self) -> TypeId;

    fn to_aggregator(&self) -> Result<ModAggregator, AnimationEvaluationError>;

    fn get_current_value(
        &self,
        entity_mut: AttributeEntityMut,
    ) -> Result<f32, AnimationEvaluationError>;
    fn get_base_value(
        &self,
        entity_mut: AttributeEntityMut,
    ) -> Result<f32, AnimationEvaluationError>;

    fn get_magnitude(&self) -> Result<f32, AnimationEvaluationError>;
}

pub struct Mutator<E> {
    _phantom: PhantomData<E>,
}

impl<E> Mutator<E> {
    pub fn new<A>(evaluator: E) -> MutatorDef<A, E> {
        MutatorDef {
            attribute: Default::default(),
            evaluator,
        }
    }
}

pub struct MutatorDef<P, C> {
    attribute: PhantomData<P>,
    evaluator: C,
}

impl<P, C> Debug for MutatorDef<P, C>
where
    P: AttributeComponent + Component<Mutability = Mutable>,
    C: MutatorEvaluator,
{
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        let attribute_name = type_name::<P>();
        f.debug_struct("MutatorDef")
            .field("attribute", &attribute_name)
            .field("evaluator", &self.evaluator)
            .finish()
    }
}

impl<P, C> EvaluateMutator for MutatorDef<P, C>
where
    P: Component<Mutability = Mutable> + AttributeComponent,
    C: MutatorEvaluator + Clone,
{
    fn clone_value(&self) -> Box<dyn EvaluateMutator> {
        Box::new(self.clone())
    }

    fn apply_mutator(&self, mut entity_mut: AttributeEntityMut) {
        let mut attribute = entity_mut.get_mut::<P>().unwrap();
        let attribute = attribute.deref_mut();
        if let Ok(aggregator) = self.evaluator.get_aggregator() {
            let new_value = aggregator.evaluate(attribute.get_attribute().base_value);
            attribute.get_attribute_mut().base_value = new_value;
        }
    }

    fn apply_aggregator(&self, mut entity_mut: AttributeEntityMut, aggregator: ModAggregator) {
        let Some(mut attribute) = entity_mut.get_mut::<P>() else {
            warn_once!("Error getting mutable attribute in apply_aggregator");
            return;
        };
        let attribute = attribute.deref_mut();

        let new_value = aggregator.evaluate(attribute.get_attribute().base_value);
        attribute.get_attribute_mut().base_value = new_value;
    }

    fn update_current_value(&self, mut entity_mut: AttributeEntityMut, aggregator: ModAggregator) {
        let mut attribute = entity_mut
            .get_mut::<P>()
            .expect("Error getting mutable attribute in update_current_value");
        let attribute = attribute.deref_mut();

        let new_value = aggregator.evaluate(attribute.get_attribute().base_value);
        attribute.get_attribute_mut().current_value = new_value;
    }

    fn target(&self) -> TypeId {
        TypeId::of::<P>()
    }

    fn to_aggregator(&self) -> std::result::Result<ModAggregator, AnimationEvaluationError> {
        self.evaluator.get_aggregator()
    }

    fn get_current_value(
        &self,
        mut entity_mut: AttributeEntityMut,
    ) -> Result<f32, AnimationEvaluationError> {
        let mut attribute = entity_mut.get_mut::<P>().unwrap();
        let attribute = attribute.deref_mut();
        Ok(attribute.get_attribute().get_current_value())
    }

    fn get_base_value(
        &self,
        mut entity_mut: AttributeEntityMut,
    ) -> Result<f32, AnimationEvaluationError> {
        let mut attribute = entity_mut.get_mut::<P>().unwrap();
        let attribute = attribute.deref_mut();
        Ok(attribute.get_attribute().get_base_value())
    }

    fn get_magnitude(&self) -> Result<f32, AnimationEvaluationError> {
        self.evaluator.get_magnitude()
    }
}

impl<P, C> Clone for MutatorDef<P, C>
where
    C: Clone,
{
    fn clone(&self) -> Self {
        Self {
            attribute: self.attribute.clone(),
            evaluator: self.evaluator.clone(),
        }
    }
}

pub type Mutators = Vec<StoredMutator>;

#[derive(Debug, Deref, DerefMut, TypePath)]
pub struct StoredMutator(pub Box<dyn EvaluateMutator>);

impl Clone for StoredMutator {
    fn clone(&self) -> Self {
        Self(EvaluateMutator::clone_value(&*self.0))
    }
}

impl StoredMutator {
    pub fn new(effect: impl EvaluateMutator) -> Self {
        Self(Box::new(effect))
    }
}

#[derive(Default, Debug, Clone, Copy, Reflect)]
pub enum ModType {
    #[default]
    Additive,
    Multiplicative,
    Overrule,
}

#[derive(Default, Debug, Clone, Copy)]
pub struct ModAggregator {
    pub additive: f32,
    pub multi: f32,
    pub overrule: Option<f32>,
}

impl ModAggregator {
    pub fn evaluate(self, value: f32) -> f32 {
        match self.overrule {
            None => (value + self.additive) * (1.0 + self.multi),
            Some(value) => value,
        }
    }

    pub fn additive(value: f32) -> Self {
        ModAggregator {
            additive: value,
            multi: 0.0,
            overrule: None,
        }
    }
    pub fn multiplicative(value: f32) -> Self {
        ModAggregator {
            additive: 0.0,
            multi: value,
            overrule: None,
        }
    }
    pub fn overrule(value: f32) -> Self {
        ModAggregator {
            additive: 0.0,
            multi: 0.0,
            overrule: Some(value),
        }
    }
}

impl Add for &ModAggregator {
    type Output = ModAggregator;

    fn add(self, rhs: Self) -> Self::Output {
        ModAggregator {
            additive: self.additive + rhs.additive,
            multi: self.additive + rhs.additive,
            overrule: self.overrule.or(rhs.overrule),
        }
    }
}

impl Add<ModAggregator> for &mut ModAggregator {
    type Output = ModAggregator;

    fn add(self, rhs: ModAggregator) -> Self::Output {
        ModAggregator {
            additive: self.additive + rhs.additive,
            multi: self.additive + rhs.additive,
            overrule: self.overrule.or(rhs.overrule),
        }
    }
}

impl AddAssign for ModAggregator {
    fn add_assign(&mut self, rhs: ModAggregator) {
        self.additive += rhs.additive;
        self.multi += rhs.multi;
        self.overrule = self.overrule.or(rhs.overrule);
    }
}

impl AddAssign for &mut ModAggregator {
    fn add_assign(&mut self, rhs: &mut ModAggregator) {
        self.additive += rhs.additive;
        self.multi += rhs.multi;
        self.overrule = self.overrule.or(rhs.overrule);
    }
}

impl Add for ModAggregator {
    type Output = Self;

    fn add(self, rhs: Self) -> Self::Output {
        ModAggregator {
            additive: self.additive + rhs.additive,
            multi: self.multi + rhs.multi,
            overrule: self.overrule.or(rhs.overrule),
        }
    }
}

impl SubAssign for ModAggregator {
    fn sub_assign(&mut self, rhs: Self) {
        self.additive -= rhs.additive;
        self.multi -= rhs.multi;
        self.overrule = self.overrule.or(rhs.overrule);
    }
}

impl Sum for ModAggregator {
    fn sum<I: Iterator<Item = Self>>(iter: I) -> Self {
        iter.fold(
            Self {
                additive: 0.0,
                multi: 0.0,
                overrule: None,
            },
            |a, b| Self {
                additive: a.additive + b.additive,
                multi: a.multi + b.multi,
                overrule: a.overrule.or(b.overrule),
            },
        )
    }
}

impl Neg for ModAggregator {
    type Output = Self;

    fn neg(self) -> Self::Output {
        Self {
            additive: -self.additive,
            multi: -self.multi,
            overrule: self.overrule,
        }
    }
}
