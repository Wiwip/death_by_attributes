use crate::effect::AttributeDependents;
use crate::inspector::pretty_type_name;
use crate::math::{AbsDiff, SaturatingAttributes};
use crate::modifier::{AttributeCalculator, AttributeCalculatorCached};
use crate::systems::MarkNodeDirty;
use bevy::ecs::component::Mutable;
use bevy::ecs::query::QueryData;
use bevy::prelude::*;
use bevy::reflect::GetTypeRegistration;
use express_it::context::ScopeId;
use express_it::expr::{Expr, ExprNode, ExpressionError};
use express_it::float::FloatExprNode;
use express_it::frame::Assignment;
use num_traits::NumCast;
pub use num_traits::{
    AsPrimitive, Bounded, FromPrimitive, Num, NumAssign, NumAssignOps, NumOps, Saturating,
    SaturatingAdd, SaturatingMul, Zero,
};
use serde::Serialize;
use std::any::{Any, TypeId};
use std::collections::HashSet;
use std::fmt::Debug;
use std::fmt::Display;
use std::hash::Hasher;
use std::hash::{DefaultHasher, Hash};
use std::iter::Sum;
use std::marker::PhantomData;
use std::sync::Arc;

pub trait Value
where
    Self: Num + NumOps + NumAssign + NumAssignOps + NumCast,
    Self: Default + Copy + Clone + Debug + Display,
    Self: Send + Sync,
    Self: SaturatingAttributes<Output = Self> + Sum + Bounded + AbsDiff + PartialOrd,
    Self: FromPrimitive + AsPrimitive<f64> + Reflect,
{
}

impl<T> Value for T
where
    Self: Num + NumOps + NumAssign + NumAssignOps + NumCast,
    Self: Default + Copy + Clone + Debug + Display,
    Self: Send + Sync,
    Self: SaturatingAttributes<Output = Self> + Sum + Bounded + AbsDiff + PartialOrd,
    Self: FromPrimitive + AsPrimitive<f64> + Reflect,
{
}

pub trait Attribute
where
    Self: Component<Mutability = Mutable> + Copy + Clone + Debug + Display,
    Self: Reflect + TypePath + GetTypeRegistration,
{
    type Property: Value;
    type ExprType: ExprNode<Self::Property>;
    const ID: u64;

    fn new<T: Num + AsPrimitive<Self::Property> + Copy>(value: T) -> Self;
    fn base_value(&self) -> Self::Property;
    fn set_base_value(&mut self, value: Self::Property);
    fn current_value(&self) -> Self::Property;
    fn borrow_current_value(&self) -> &Self::Property;
    fn set_current_value(&mut self, value: Self::Property);
    // Helper to wrap attribute access in an Expression
    fn src() -> Expr<Self::Property, Self::ExprType>;
    fn dst() -> Expr<Self::Property, Self::ExprType>;
    fn parent() -> Expr<Self::Property, Self::ExprType>;
    fn lit(value: Self::Property) -> Expr<Self::Property, Self::ExprType>;
    // Expression helpers
    fn set(
        scope: impl Into<ScopeId>,
        expr: Expr<Self::Property, Self::ExprType>,
    ) -> Assignment<Self::Property, Self::ExprType>;
    fn add(
        scope: impl Into<ScopeId>,
        expr: Expr<Self::Property, Self::ExprType>,
    ) -> Assignment<Self::Property, Self::ExprType>;
}

#[macro_export]
macro_rules! attribute_impl {
    ( $StructName:ident, $ValueType:ty ) => {
        #[derive(
            bevy::prelude::Component,
            Clone,
            Copy,
            bevy::prelude::Reflect,
            Debug,
            serde::Serialize,
            serde::Deserialize,
        )]
        #[reflect(Component, AccessAttribute)]
        pub struct $StructName {
            base_value: $ValueType,
            current_value: $ValueType,
        }

        impl $crate::attributes::Attribute for $StructName {
            type Property = $ValueType;
            type ExprType = express_it::expr::SelectExprNode<$ValueType>;

            const ID: u64 = express_it::context::fnv1a64(stringify!($StructName));

            fn new<T>(value: T) -> Self
            where
                T: $crate::num_traits::Num + $crate::num_traits::AsPrimitive<Self::Property> + Copy,
            {
                Self {
                    base_value: value.as_(),
                    current_value: value.as_(),
                }
            }
            fn base_value(&self) -> $ValueType {
                self.base_value
            }
            fn set_base_value(&mut self, value: $ValueType) {
                self.base_value = value;
            }
            fn current_value(&self) -> $ValueType {
                self.current_value
            }
            fn borrow_current_value(&self) -> &$ValueType {
                &self.current_value
            }
            fn set_current_value(&mut self, value: $ValueType) {
                self.current_value = value;
            }
            fn src() -> express_it::expr::Expr<Self::Property, Self::ExprType> {
                express_it::expr::Expr::new(std::sync::Arc::new(Self::ExprType::Attribute(
                    express_it::context::Path::from_id($crate::modifier::Who::Source, Self::ID),
                )))
            }
            fn dst() -> express_it::expr::Expr<Self::Property, Self::ExprType> {
                express_it::expr::Expr::new(std::sync::Arc::new(Self::ExprType::Attribute(
                    express_it::context::Path::from_id($crate::modifier::Who::Target, Self::ID),
                )))
            }
            fn parent() -> express_it::expr::Expr<Self::Property, Self::ExprType> {
                express_it::expr::Expr::new(std::sync::Arc::new(Self::ExprType::Attribute(
                    express_it::context::Path::from_id($crate::modifier::Who::Owner, Self::ID),
                )))
            }
            fn lit(value: $ValueType) -> express_it::expr::Expr<Self::Property, Self::ExprType> {
                express_it::expr::Expr::<Self::Property, Self::ExprType>::new(std::sync::Arc::new(
                    Self::ExprType::Lit(value),
                ))
            }
            fn set(
                scope: impl Into<express_it::context::ScopeId>,
                expr: express_it::expr::Expr<Self::Property, Self::ExprType>,
            ) -> express_it::frame::Assignment<Self::Property, Self::ExprType> {
                express_it::frame::Assignment {
                    path: express_it::context::Path::from_id(
                        $crate::modifier::Who::Owner,
                        Self::ID,
                    ),
                    expr,
                }
            }
            fn add(
                scope: impl Into<express_it::context::ScopeId>,
                expr: express_it::expr::Expr<Self::Property, Self::ExprType>,
            ) -> express_it::frame::Assignment<Self::Property, Self::ExprType> {
                let get_expr = express_it::expr::Expr::new(std::sync::Arc::new(
                    Self::ExprType::Attribute(express_it::context::Path::from_id(
                        scope.into(),
                        Self::ID,
                    )),
                ));

                express_it::frame::Assignment {
                    path: express_it::context::Path::from_id(
                        $crate::modifier::Who::Owner,
                        Self::ID,
                    ),
                    expr: get_expr + expr,
                }
            }
        }

        impl std::fmt::Display for $StructName {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                write!(f, "{}: {}", stringify!($StructName), self.current_value)
            }
        }
    };
}

#[macro_export]
macro_rules! attribute {
    ( $StructName:ident ) => {
        $crate::attribute_impl!($StructName, f32);
    };
    ( $StructName:ident, $ValueType:ty  ) => {
        $crate::attribute_impl!($StructName, $ValueType);
    };
}

#[macro_export]
macro_rules! tag {
    ( $StructName:ident ) => {
        #[derive(
            bevy::prelude::Component,
            bevy::prelude::Reflect,
            Default,
            Copy,
            Clone,
            Debug,
            serde::Serialize,
            serde::Deserialize,
        )]
        #[reflect(Component)]
        pub struct $StructName;
    };
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Reflect)]
pub struct AttributeTypeId(pub u64);

impl AttributeTypeId {
    pub fn of<T: TypePath>() -> Self {
        let mut hasher = DefaultHasher::new();
        T::type_path().hash(&mut hasher);
        Self(hasher.finish())
    }

    pub fn from_reflect(reflect: &dyn Reflect) -> Self {
        let mut hasher = DefaultHasher::new();
        reflect.reflect_type_path().hash(&mut hasher);
        Self(hasher.finish())
    }
}

#[derive(QueryData, Debug)]
#[query_data(mutable, derive(Debug))]
pub struct AttributeQueryData<T: Attribute + 'static> {
    pub entity: Entity,
    pub attribute: &'static mut T,
    pub calculator_cache: &'static mut AttributeCalculatorCached<T>,
}

impl<T: Attribute> AttributeQueryDataItem<'_, '_, T> {
    pub fn update_attribute(&mut self, calculator: &AttributeCalculator<T>) -> bool {
        let old_val = self.attribute.current_value();
        let new_val = calculator.eval(self.attribute.base_value());

        let has_changed = old_val.are_different(new_val);
        if has_changed {
            self.attribute.set_current_value(new_val);
        }
        has_changed
    }

    pub fn update_attribute_from_cache(&mut self) -> bool {
        let old_val = self.attribute.current_value();
        let new_val = self
            .calculator_cache
            .calculator
            .eval(self.attribute.base_value());

        let has_changed = old_val.are_different(new_val);
        if has_changed {
            self.attribute.set_current_value(new_val);
        }
        has_changed
    }
}

#[reflect_trait] // Generates a `ReflectMyTrait` type
pub trait AccessAttribute {
    fn access_base_value(&self) -> f64;
    fn access_current_value(&self) -> f64;
    fn any_current_value(&self) -> &dyn Any;
    fn name(&self) -> String;
}

impl<T> AccessAttribute for T
where
    T: Attribute,
{
    fn access_base_value(&self) -> f64 {
        self.base_value().as_()
    }
    fn access_current_value(&self) -> f64 {
        self.current_value().as_()
    }
    fn any_current_value(&self) -> &dyn Any {
        self.borrow_current_value()
    }
    fn name(&self) -> String {
        pretty_type_name::<T>()
    }
}

pub fn on_add_attribute<T: Attribute>(trigger: On<Insert, T>, mut commands: Commands) {
    commands.trigger(MarkNodeDirty::<T> {
        entity: trigger.event_target(),
        phantom_data: Default::default(),
    });
}

#[derive(EntityEvent)]
pub struct AttributeDependencyChanged<T> {
    pub entity: Entity,
    phantom_data: PhantomData<T>,
}

pub fn on_change_notify_attribute_dependencies<T: Attribute>(
    query: Query<&AttributeDependents<T>, Changed<T>>,
    mut commands: Commands,
) {
    for dependents in query.iter() {
        let unique_entities: HashSet<Entity> = dependents.iter().collect();
        let notify_targets: Vec<Entity> = unique_entities.into_iter().collect();

        notify_targets.iter().for_each(|target| {
            commands.trigger(AttributeDependencyChanged::<T> {
                entity: *target,
                phantom_data: Default::default(),
            });
        });
    }
}

pub fn on_change_notify_attribute_parents<T: Attribute>(
    query: Query<Entity, Changed<T>>,
    mut commands: Commands,
) {
    for entity in query.iter() {
        commands.trigger(MarkNodeDirty::<T> {
            entity,
            phantom_data: Default::default(),
        });
    }
}
