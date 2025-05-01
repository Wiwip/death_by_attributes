
use crate::{AttributeEntityMut, Editable};
use bevy::prelude::FromReflect;
use bevy::prelude::Reflect;
use bevy::reflect::Reflectable;
use std::fmt::Debug;
use crate::attributes::{AttributeAccessorMut, AttributeAccessorRef};

#[derive(Reflect, FromReflect)]
#[reflect(from_reflect = false)]
pub struct MetaMod<P, Q, C> {
    // The attribute selectors
    target_attribute: P,
    source_attribute: Q,

    // The function to evaluate the attribute
    evaluator: C,
}

impl<P, Q, C> MetaMod<P, Q, C>
where
    P: AttributeAccessorMut,
    Q: AttributeAccessorRef,
    C: EvaluateMetaMod<P::Property>,
{
    pub fn new(target_attribute: P, source_attribute: Q, evaluator: C) -> Self {
        MetaMod {
            target_attribute,
            source_attribute,
            evaluator,
        }
    }

    fn apply(&self, entity_mut: &mut AttributeEntityMut) {
        let entity_ref = entity_mut.as_readonly();
        let source = {
            let source = self.source_attribute.get(&entity_ref).unwrap();
            source.get_current_value()
        };

        let target = self.target_attribute.get_mut(entity_mut).unwrap();
        self.evaluator.evaluate(target, source);
    }
}

impl<P, Q, C> Clone for MetaMod<P, Q, C>
where
    C: Clone,
    P: Clone,
    Q: Clone,
{
    fn clone(&self) -> Self {
        Self {
            target_attribute: self.target_attribute.clone(),
            source_attribute: self.source_attribute.clone(),
            evaluator: self.evaluator.clone(),
        }
    }
}

pub trait EvaluateMetaMod<T>: Debug + Clone + Reflectable {
    fn evaluate(&self, target: &mut T, source: f32);
}

#[derive(Default, Debug, Clone, Reflect)]
struct MetaModEvaluator {
    magnitude: f32,
}

impl MetaModEvaluator {
    pub fn new() -> Self {
        Self {
            magnitude: 1.0,
        }
    }
}

impl<T: Editable> EvaluateMetaMod<T> for MetaModEvaluator {
    fn evaluate(&self, target: &mut T, source: f32) {
        target.set_base_value(source * self.magnitude)
    }
}

#[cfg(test)]
mod tests {
    use crate::attributes::AttributeMut;
use super::*;
    use crate::AttributeDef;
    use crate::GameAbilityContainer;
    use crate::GameEffectContainer;
    use crate::attributes::AttributeRef;
    use crate::*;
    use crate::{attribute};
    use bevy::ecs::system::RunSystemOnce;

    attribute!(Health);
    attribute!(HealthRegen);

    #[test]
    fn test_meta_attribute_world() {
        let mut world = World::default();
        let id = world.spawn((Health::new(0.0), HealthRegen::new(10.0))).id();

        let health = world.get::<Health>(id).unwrap();
        assert_eq!(health.base_value, 0.0);
        
        let _ = world.run_system_once(test_apply);

        let health = world.get::<Health>(id).unwrap();
        assert_eq!(health.base_value, 10.0);
        
        fn test_apply(mut query: Query<AttributeEntityMut>) {
            let health = attribute_mut!(Health);
            let health_regen = attribute_ref!(HealthRegen);
            let meta_mod = MetaMod::new(health, health_regen, MetaModEvaluator::new());
            
            for mut entity in query.iter_mut() {
                meta_mod.apply(&mut entity);
            }
        }
    }
}
