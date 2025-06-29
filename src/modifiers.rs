use crate::Dirty;
use crate::attributes::AttributeComponent;
use crate::effects::EffectOf;
use bevy::ecs::component::Mutable;
use bevy::prelude::*;
use std::any::{TypeId, type_name};
use std::fmt::Debug;
use std::fmt::Display;
use std::iter::Sum;
use std::marker::PhantomData;
use std::ops::{Add, AddAssign, Mul};

#[derive(Component, Copy, Clone, Debug, Reflect)]
pub struct Modifier<T> {
    #[reflect(ignore)]
    _phantom: PhantomData<T>,
    pub value: ModAggregator<T>,
}

impl<T: 'static> Modifier<T> {
    pub fn new(value: f64, mod_type: ModType) -> Self {
        Self {
            _phantom: Default::default(),
            value: ModAggregator::new(value, mod_type),
        }
    }

    pub fn target(&self) -> TypeId {
        TypeId::of::<T>()
    }
}

impl<T> Default for Modifier<T> {
    fn default() -> Self {
        Self {
            _phantom: Default::default(),
            value: ModAggregator::default(),
        }
    }
}

/// Spawns a modifier entity attached to an effect
pub struct ModifierCommand<C> {
    pub(crate) effect_entity: Entity,
    pub(crate) actor_entity: Entity,
    pub(crate) modifier: Modifier<C>,
    pub(crate) observer: Option<Observer>,
}

impl<C> Command for ModifierCommand<C>
where
    C: AttributeComponent + Component<Mutability = Mutable>,
{
    fn apply(self, world: &mut World) -> () {
        assert_ne!(Entity::PLACEHOLDER, self.effect_entity);
        assert_ne!(Entity::PLACEHOLDER, self.actor_entity);
        // We attach an observer to the mutator targeting the parent entity
        let mut entity = world.spawn((
            Name::new(format!("{}", type_name::<C>())),
            self.modifier,
            ModAggregator::<C>::default(),
            EffectOf(self.effect_entity),
            Dirty::<C>::default(),
        ));
        if let Some(observer) = self.observer {
            entity.insert(observer);
        }
    }
}

#[derive(Default, Debug, Clone, Copy, Reflect)]
pub enum ModType {
    #[default]
    Additive,
    Multiplicative,
    Overrule,
}

#[derive(Component, Copy, Reflect)]
pub struct ModAggregator<T> {
    phantom_data: PhantomData<T>,
    pub additive: f64,
    pub multi: f64,
    pub overrule: Option<f64>,
}

impl<T> Default for ModAggregator<T> {
    fn default() -> Self {
        Self {
            phantom_data: Default::default(),
            additive: 0.0,
            multi: 0.0,
            overrule: None,
        }
    }
}

impl<T> ModAggregator<T> {
    pub(crate) fn new(magnitude: f64, mod_type: ModType) -> ModAggregator<T> {
        match mod_type {
            ModType::Additive => ModAggregator::<T>::additive(magnitude),
            ModType::Multiplicative => ModAggregator::<T>::multiplicative(magnitude),
            ModType::Overrule => ModAggregator::<T>::overrule(magnitude),
        }
    }

    pub fn evaluate(&self, value: f64) -> f64 {
        match self.overrule {
            None => (value + self.additive) * (1.0 + self.multi),
            Some(value) => value,
        }
    }

    pub fn additive(value: f64) -> Self {
        ModAggregator::<T> {
            phantom_data: PhantomData,
            additive: value,
            multi: 0.0,
            overrule: None,
        }
    }
    pub fn multiplicative(value: f64) -> Self {
        ModAggregator::<T> {
            phantom_data: PhantomData,
            additive: 0.0,
            multi: value,
            overrule: None,
        }
    }
    pub fn overrule(value: f64) -> Self {
        ModAggregator::<T> {
            phantom_data: PhantomData,
            additive: 0.0,
            multi: 0.0,
            overrule: Some(value),
        }
    }
}

impl<T> Add for &ModAggregator<T> {
    type Output = ModAggregator<T>;

    fn add(self, rhs: Self) -> Self::Output {
        ModAggregator::<T> {
            phantom_data: PhantomData,
            additive: self.additive + rhs.additive,
            multi: self.multi + rhs.multi,
            overrule: self.overrule.or(rhs.overrule),
        }
    }
}

impl<T> Add<ModAggregator<T>> for &mut ModAggregator<T> {
    type Output = ModAggregator<T>;

    fn add(self, rhs: ModAggregator<T>) -> Self::Output {
        ModAggregator::<T> {
            phantom_data: PhantomData,
            additive: self.additive + rhs.additive,
            multi: self.multi + rhs.multi,
            overrule: self.overrule.or(rhs.overrule),
        }
    }
}

impl<T> Mul<ModAggregator<T>> for ModAggregator<T> {
    type Output = Self;

    fn mul(self, rhs: ModAggregator<T>) -> Self::Output {
        Self {
            phantom_data: Default::default(),
            additive: self.additive * rhs.additive,
            multi: self.multi * rhs.multi,
            overrule: self.overrule.or(rhs.overrule),
        }
    }
}

impl<T> Mul<f64> for ModAggregator<T> {
    type Output = Self;

    fn mul(self, rhs: f64) -> Self::Output {
        Self {
            phantom_data: Default::default(),
            additive: self.additive * rhs,
            multi: self.multi * rhs,
            overrule: self.overrule,
        }
    }
}

impl<T> AddAssign for ModAggregator<T> {
    fn add_assign(&mut self, rhs: ModAggregator<T>) {
        self.additive += rhs.additive;
        self.multi += rhs.multi;
        self.overrule = self.overrule.or(rhs.overrule);
    }
}

impl<T> AddAssign for &mut ModAggregator<T> {
    fn add_assign(&mut self, rhs: &mut ModAggregator<T>) {
        self.additive += rhs.additive;
        self.multi += rhs.multi;
        self.overrule = self.overrule.or(rhs.overrule);
    }
}

impl<T> Add for ModAggregator<T> {
    type Output = Self;

    fn add(self, rhs: Self) -> Self::Output {
        ModAggregator::<T> {
            phantom_data: PhantomData,
            additive: self.additive + rhs.additive,
            multi: self.multi + rhs.multi,
            overrule: self.overrule.or(rhs.overrule),
        }
    }
}

impl<T> Clone for ModAggregator<T> {
    fn clone(&self) -> Self {
        ModAggregator::<T> {
            phantom_data: Default::default(),
            additive: self.additive,
            multi: self.multi,
            overrule: self.overrule,
        }
    }
}

impl<T> Sum for ModAggregator<T> {
    fn sum<I: Iterator<Item = Self>>(iter: I) -> Self {
        iter.fold(
            Self {
                phantom_data: PhantomData,
                additive: 0.0,
                multi: 0.0,
                overrule: None,
            },
            |a, b| Self {
                phantom_data: PhantomData,
                additive: a.additive + b.additive,
                multi: a.multi + b.multi,
                overrule: a.overrule.or(b.overrule),
            },
        )
    }
}

impl<T> Display for ModAggregator<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "+{:.1} x{:.1} (or {:?})",
            self.additive, self.multi, self.overrule
        )
    }
}

impl<T> Debug for ModAggregator<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ModAggregator")
            .field("additive", &self.additive)
            .field("multiplicative", &self.multi)
            .field("overrule", &self.overrule)
            .finish()
    }
}
