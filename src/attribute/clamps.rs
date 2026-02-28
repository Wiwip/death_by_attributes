use crate::actors::Actor;
use crate::assets::ActorDef;
use crate::attributes::AttributeQueryData;
use crate::condition::BevyContext;
use crate::prelude::*;
use crate::{AppAttributeBindings, AttributesRef, CurrentValueChanged};
use bevy::prelude::*;
use express_it::expr::Expr;

#[derive(Component, Default, Debug, Clone, Reflect)]
#[reflect(Component, from_reflect = false)]
pub struct Clamp<T: Attribute> {
    pub min_limit: T::Property,
    pub max_limit: T::Property,
}

impl<T> Clamp<T>
where
    T: Attribute,
{
    pub fn new() -> Self {
        Self {
            min_limit: T::Property::default(),
            max_limit: T::Property::default(),
        }
    }
}

/// When the Source attribute changes, we update the bounds of the target attribute
pub fn update_clamps<T: Attribute>(
    trigger: On<CurrentValueChanged<T>>,
    mut set: ParamSet<(Query<(&Actor, AttributesRef)>, Query<&mut Clamp<T>>)>,
    actor_assets: Res<Assets<ActorDef>>,
    type_registry: Res<AppTypeRegistry>,
    type_bindings: Res<AppAttributeBindings>,
) -> Result<(), BevyError> {
    let (min_value, max_value) = {
        let p0 = set.p0();
        let (actor_handle, attribute_ref) = p0.get(trigger.entity)?;
        let actor_def = actor_assets
            .get(&actor_handle.0)
            .ok_or("Missing actor asset.")?;

        let actor_context = BevyContext {
            source_actor: &attribute_ref,
            target_actor: &attribute_ref,
            owner: &attribute_ref,
            type_registry: type_registry.0.clone(),
            type_bindings: type_bindings.clone(),
        };

        let Some(clamp_exprs) = actor_def.clamp_exprs.get(&T::ID) else {
            return Ok(());
        };

        let (min_expr, max_expr) = clamp_exprs
            .downcast_ref::<(Expr<T::Property>, Expr<T::Property>)>()
            .ok_or("Failed downcast expressions.")?;

        let min_value = min_expr.eval_dyn(&actor_context)?;
        let max_value = max_expr.eval_dyn(&actor_context)?;

        (min_value, max_value)
    };

    let mut clamps = set.p1();
    let mut clamp = clamps.get_mut(trigger.entity)?;

    // Multiply the source value by the limit to get the derived limit
    clamp.min_limit = min_value;
    clamp.max_limit = max_value;
    Ok(())
}

pub fn apply_clamps<T>(
    mut query: Query<(AttributeQueryData<T>, &Clamp<T>), (Changed<T>, Changed<Clamp<T>>)>,
) where
    T: Attribute,
{
    fn clamp_partial<V: Copy + PartialOrd>(value: V, min: V, max: V) -> V {
        let value = if value < min { min } else { value };
        if value > max { max } else { value }
    }

    for (mut attribute_data, clamp) in query.iter_mut() {
        let base = attribute_data.attribute.base_value();
        let clamped = clamp_partial(base, clamp.min_limit, clamp.max_limit);

        if clamped != base {
            attribute_data.attribute.set_base_value(clamped);
            // Base changed => recompute current from cached calculator.
            attribute_data.update_attribute_from_cache();
        }
    }
}
