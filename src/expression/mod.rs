mod attribute;

use crate::AttributeError;
use crate::condition::GameplayContext;
use crate::prelude::*;
use bevy::prelude::{Deref, DerefMut, Reflect};
use num_traits::Num;
use std::any::TypeId;
use std::marker::PhantomData;
use std::sync::Arc;

pub trait Expression: Send + Sync + 'static {
    type Out: Num;

    fn eval(&self, context: &GameplayContext) -> Result<Self::Out, AttributeError>;
}

pub trait IntoExpression {
    type Out: Num;

    fn into_expr(self) -> Expr<Self::Out>;
}

#[derive(Default, Debug, Clone, Reflect)]
pub enum Expr<P: Num> {
    #[default]
    Empty,
    Lit(P),
    Expr(AttributeExprRef<P>),
    Operation,
}

impl<P: Num + Send + Sync + Copy + 'static> Expression for Expr<P> {
    type Out = P;

    fn eval(&self, context: &GameplayContext) -> Result<Self::Out, AttributeError> {
        match self {
            Expr::Lit(value) => Ok(*value),
            Expr::Expr(expr_ref) => expr_ref.eval(context),
            Expr::Operation => {
                todo!()
            }
            Expr::Empty => {
                todo!()
            }
        }
    }
}

#[derive(Deref, DerefMut, Clone)]
pub struct AttributeExprRef<P: Num>(pub Arc<dyn Expression<Out = P>>);

impl<P: Num> std::fmt::Debug for AttributeExprRef<P> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("AttributeExprRef").finish()
    }
}

pub enum AttributeExpr<T> {
    Source,
    Target,
    Parent,
    _Phantom(PhantomData<T>),
}

impl<T: Attribute> Expression for AttributeExpr<T> {
    type Out = T::Property;

    fn eval(&self, context: &GameplayContext) -> Result<T::Property, AttributeError> {
        let entity = match self {
            AttributeExpr::Source => context.source_actor,
            AttributeExpr::Target => context.target_actor,
            AttributeExpr::Parent => context.owner,
            _ => return Err(AttributeError::PhantomQuery),
        };

        Ok(entity
            .get::<T>()
            .ok_or(AttributeError::AttributeNotPresent(TypeId::of::<T>()))?
            .current_value())
    }
}

#[derive(Debug)]
pub struct AddExpression<T1, T2> {
    lhs: T1,
    rhs: T2,
}

impl<T1, T2> Expression for AddExpression<T1, T2>
where
    T1: Expression,
    T2: Expression,
    T1::Out: Num + From<T2::Out>,
{
    type Out = T1::Out;

    fn eval(&self, context: &GameplayContext) -> Result<Self::Out, AttributeError> {
        Ok(self.lhs.eval(context)? + self.rhs.eval(context)?.into())
    }
}

impl<P> std::ops::Add for Expr<P>
where
    P: Num + Send + Sync + Copy + 'static,
{
    type Output = Expr<P>;

    fn add(self, rhs: Self) -> Self::Output {
        let add = AddExpression { lhs: self, rhs };

        let expr = AttributeExprRef(Arc::new(add));

        Expr::Expr(expr)
    }
}

impl std::ops::Add<Expr<u32>> for u32 {
    type Output = Expr<u32>;

    fn add(self, rhs: Expr<u32>) -> Self::Output {
        Expr::Lit(self) + rhs
    }
}

impl std::ops::Add<u32> for Expr<u32> {
    type Output = Expr<u32>;

    fn add(self, rhs: u32) -> Self::Output {
        self + Expr::Lit(rhs)
    }
}

/*impl<T1, T2> AttributeExpr for AddExpression<T1, T2>
where
    T1: AttributeExpr,
    T2: AttributeExpr,
    T1::Output: Num + From<T2::Output>,
{
    type Output = T1::Output;

    fn eval(&self, context: &EffectContext) -> Result<Self::Output, AttributeError> {
        Ok(self.lhs.eval(context)? + self.rhs.eval(context)?.into())
    }
}*/

/*
pub trait AttributeExpr: Send + Sync + std::fmt::Display + Debug + 'static {
    type Output: Num;

    fn eval(&self, context: &EffectContext) -> Result<Self::Output, AttributeError>;

    fn insert_dependency(
        &self,
        target: Entity,
        entity_commands: &mut EntityCommands,
        func: fn(Entity, Commands),
    );
}

pub trait IntoExpression {
    type Out: Num;

    fn into_expr(self) -> Value<Self::Out>;
}

/// A [Value] refers to an Attribute value.
/// It can be a literal value, or a reference to an Attribute.
#[derive(Deref, DerefMut)]
pub struct Value<P: Num>(pub Arc<dyn AttributeExpr<Output = P>>);

impl<P: Num + std::fmt::Display + Debug + Copy + Clone + Send + Sync + 'static> Default
    for Value<P>
{
    fn default() -> Self {
        Value(Arc::new(Lit(P::zero())))
    }
}

impl<P: Num + 'static> Clone for Value<P> {
    fn clone(&self) -> Self {
        Self(self.0.clone())
    }
}

impl<P: Num + 'static> Debug for Value<P> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}", self.0)
    }
}

impl<P: Num + 'static> std::fmt::Display for Value<P> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// An [AttributeValue] is a dynamic reference to an Attribute.
#[derive(Clone, Copy, Debug)]
pub struct AttributeValue<T: Attribute> {
    pub cached_value: T::Property,
    pub target: Who,
    pub phantom_data: PhantomData<T>,
}

impl<T: Attribute> std::fmt::Display for AttributeValue<T> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "AttributeValue({:.4},{})",
            self.cached_value, self.target
        )
    }
}

impl<T: Attribute> AttributeExpr for AttributeValue<T> {
    type Output = T::Property;

    fn eval(&self, context: &EffectContext) -> Result<Self::Output, AttributeError> {
        let attribute_ref = self.target.resolve_entity(context);
        Ok(attribute_ref
            .get::<T>()
            .ok_or(AttributeError::AttributeNotPresent(TypeId::of::<T>()))?
            .current_value())
    }

    /// Inserts a dependency on the target entity.
    /// This is used to ensure that the target entity is updated when the source attribute changes.
    /// The func serves as a trigger to MarkNodeDirty<T> on the attribute that must be recalculated
    fn insert_dependency(
        &self,
        target: Entity,
        entity_commands: &mut EntityCommands,
        func: fn(Entity, Commands),
    ) {
        entity_commands.insert(AttributeDependency::<T>::new(target));

        let mut observer = Observer::new(
            move |trigger: On<AttributeDependencyChanged<T>>, commands: Commands| {
                func(trigger.entity, commands);
            },
        );
        observer.watch_entity(entity_commands.id());
        entity_commands.commands().spawn(observer);
    }
}

impl<T: Attribute> IntoExpression for AttributeValue<T> {
    type Out = T::Property;

    fn into_expr(self) -> Value<Self::Out> {
        Value(Arc::new(AttributeValue::<T> {
            cached_value: Self::Out::zero(),
            phantom_data: Default::default(),
            target: Who::Target,
        }))
    }
}

/// A [Lit] is a static value.
#[derive(Deref, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Lit<T: Num>(pub T);

impl<T: Num + Clone + Copy + Send + Sync + 'static + std::fmt::Display> std::fmt::Display
    for Lit<T>
{
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        todo!()
    }
}

impl<T: Num + Clone + Copy + Send + Sync + 'static + std::fmt::Display> Debug for Lit<T> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        todo!()
    }
}

impl<T: Num + Clone + Copy + Send + Sync + 'static + std::fmt::Display> AttributeExpr for Lit<T> {
    type Output = T;

    fn eval(&self, _context: &EffectContext) -> Result<Self::Output, AttributeError> {
        Ok(self.0)
    }
    fn insert_dependency(
        &self,
        _: Entity,
        _: &mut EntityCommands<'_>,
        _: for<'a, 'b> fn(Entity, Commands<'a, 'b>),
    ) {
        // Empty implementation
    }
}

#[macro_export]
macro_rules! impl_into_value {
    ( $x:ty ) => {
        impl IntoExpression for $x {
            type Out = $x;

            fn into_expr(self) -> Value<$x> {
                Value(Arc::new(Lit(self)))
            }
        }
    };
}
*/

/*
#[derive(Debug)]
pub struct AddExpression<T1, T2> {
    lhs: T1,
    rhs: T2,
}

/*impl<T1, T2> AttributeExpr for AddExpression<T1, T2>
where
    T1: AttributeExpr,
    T2: AttributeExpr,
    T1::Output: Num + From<T2::Output>,
{
    type Output = T1::Output;

    fn eval(&self, context: &EffectContext) -> Result<Self::Output, AttributeError> {
        Ok(self.lhs.eval(context)? + self.rhs.eval(context)?.into())
    }
}

impl<T1, T2> std::ops::Add<AttributeValue<T1>> for AttributeValue<T2>
where
    T1: Attribute,
    T2: Attribute,
{
    type Output = AddExpression<AttributeValue<T2>, AttributeValue<T1>>;

    fn add(self, rhs: AttributeValue<T1>) -> AddExpression<AttributeValue<T2>, AttributeValue<T1>> {
        AddExpression { lhs: self, rhs }
    }
}*/
*/

#[macro_export]
macro_rules! impl_into_expr {
    ( $x:ty ) => {
        impl From<$x> for Expr<$x> {
            fn from(value: $x) -> Self {
                Expr::Lit(value)
            }
        }
    };
}

impl_into_expr!(i8);
impl_into_expr!(i16);
impl_into_expr!(i32);
impl_into_expr!(i64);
impl_into_expr!(i128);
impl_into_expr!(isize);

impl_into_expr!(u8);
impl_into_expr!(u16);
impl_into_expr!(u32);
impl_into_expr!(u64);
impl_into_expr!(u128);
impl_into_expr!(usize);

impl_into_expr!(f32);
impl_into_expr!(f64);

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ReflectAccessAttribute;
    use crate::{AttributesRef, attribute};
    use bevy::ecs::system::RunSystemOnce;
    use bevy::prelude::{Single, World};

    attribute!(Test, f32);
    attribute!(TestF64, f64);

    #[test]
    fn test() {
        let mut world = World::new();

        world.spawn((Test::new(100.0), TestF64::new(50.0)));

        world
            .run_system_once(|actor: Single<AttributesRef>| {
                /*let a = Test::value();
                let b = TestF64::value();

                //let _add_expr_inv = a + b;

                let a = AttributeValueExpr::<Test> {
                    who: Who::Target,
                    phantom_data: Default::default(),
                }
                .expr();
                let b = AttributeValueExpr::<Test> {
                    who: Who::Target,
                    phantom_data: Default::default(),
                }
                .expr();

                let add_expr = a + b;

                let context = GameplayContext {
                    target_actor: &actor,
                    source_actor: &actor,
                    owner: &actor,
                };

                let result = add_expr.eval(&context).unwrap();

                println!("Result: {}", result)*/
            })
            .unwrap();
    }
}
