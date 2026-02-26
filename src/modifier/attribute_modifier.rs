use crate::condition::BevyContext;
use crate::effect::{EffectSource, EffectTarget};
use crate::inspector::pretty_type_name;
use crate::modifier::calculator::ModOp;
use crate::modifier::{ModifierMarker, ModifierOf};
use crate::modifier::{ReflectAccessModifier, Who};
use crate::prelude::*;
use crate::systems::MarkNodeDirty;
use crate::{AppTypeIdBindings, AttributesRef, TypeIdBindings};
use bevy::prelude::*;
use express_it::expr::{Expr, ExprNode, SelectExprNodeImpl};
use std::collections::HashSet;
use std::fmt::Display;
use std::marker::PhantomData;

pub trait PersistentModifier: Send + Sync {
    fn spawn_persistent_modifier(
        &self,
        actor_entity: Entity,
        ctx: &BevyContext,
        type_bindings: &TypeIdBindings,
        commands: &mut EntityCommands,
    );
}

#[derive(Component, Clone, Reflect)]
#[reflect(Component, from_reflect = false)]
#[reflect(AccessModifier)]
#[require(ModifierMarker)]
pub struct AttributeModifier<T: Attribute> {
    #[reflect(ignore)]
    pub expr: Expr<T::Property>,
    pub value: T::Property,
    pub who: Who,
    pub operation: ModOp,
}

impl<T> AttributeModifier<T>
where
    T: Attribute + 'static,
{
    pub fn new(value: T::Property, modifier: ModOp, who: Who, expr: Expr<T::Property>) -> Self {
        Self {
            expr,
            value,
            who,
            operation: modifier,
        }
    }
}

impl<T> PersistentModifier for AttributeModifier<T>
where
    T: Attribute,
    T::Property: SelectExprNodeImpl<Property = T::Property>,
{
    fn spawn_persistent_modifier(
        &self,
        actor_entity: Entity,
        ctx: &BevyContext,
        type_bindings: &TypeIdBindings,
        commands: &mut EntityCommands,
    ) {
        let Ok(value) = self.expr.eval_dyn(ctx) else {
            error!(
                "{}: Could not resolve expression to spawn persistent modifier.",
                commands.id()
            );
            return;
        };

        let modifier = AttributeModifier::<T> {
            expr: self.expr.clone(),
            value,
            who: self.who,
            operation: self.operation,
        };
        let display = modifier.to_string();

        // Spawn the observer. Watches the actor for attribute value changes.
        let mut dependencies = HashSet::default();
        self.expr.inner.get_dependencies(&mut dependencies);
        for dependency in dependencies {
            let attr_dep = type_bindings
                .insert_dependency_map
                .get(&dependency.id)
                .unwrap();
            attr_dep(actor_entity, commands);
        }

        commands.insert((modifier, Name::new(format!("{}", display))));
    }
}

impl<T> Display for AttributeModifier<T>
where
    T: Attribute,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "Mod<{}>({}{}) {}",
            pretty_type_name::<T>(),
            self.operation,
            self.value,
            self.who,
        )
    }
}

#[derive(EntityEvent)]
pub struct RecalculateExpression {
    #[event_target]
    pub modifier_entity: Entity,
}

/// When the attribute changes, update the values of dependent AttributeModifier<T>.
pub fn update_modifier_when_dependencies_changed<T: Attribute>(
    trigger: On<RecalculateExpression>,
    mut modifiers: Query<(&mut AttributeModifier<T>, &ModifierOf)>,
    effects: Query<(&EffectSource, &EffectTarget)>,
    actors: Query<AttributesRef, Without<AttributeModifier<T>>>,
    type_registry: Res<AppTypeRegistry>,
    type_bindings: Res<AppTypeIdBindings>,
    mut commands: Commands,
) {
    let Ok((mut modifier, effect_id)) = modifiers.get_mut(trigger.modifier_entity) else {
        //error!("{}: Missing AttributeModifier<{}>", trigger.modifier_entity, pretty_type_name::<T>());
        return;
    };
    let (source, target) = effects.get(effect_id.0).unwrap();
    let [source_ref, target_ref] = actors.get_many([source.0, target.0]).unwrap();

    let context = BevyContext {
        target_actor: &target_ref,
        source_actor: &source_ref,
        owner: &source_ref,
        type_registry: type_registry.0.clone(),
        type_bindings: type_bindings.clone(),
    };

    let new_val = modifier.expr.eval_dyn(&context).unwrap();
    modifier.value = new_val;

    commands.trigger(MarkNodeDirty::<T> {
        entity: effect_id.0,
        phantom_data: Default::default(),
    });
}

/*impl<T: Attribute> Modifier for AttributeModifier<T> {
    fn apply_immediate(
        &self,
        context: &mut BevyContextMut,
        type_registry: TypeRegistryArc,
        type_bindings: AppTypeIdBindings,
    ) -> bool {
        let immutable_context = BevyContext {
            source_actor: &context.source_actor.as_readonly(),
            target_actor: &context.source_actor.as_readonly(),
            owner: &context.owner.as_readonly(),
            type_registry: type_registry.clone(),
            type_bindings: type_bindings.clone(),
        };

        let Ok(calc) = AttributeCalculator::<T>::convert(self, &immutable_context) else {
            return false;
        };
        let Some(attribute) = immutable_context.attribute_ref(Who::Target).get::<T>() else {
            return false;
        };
        let new_val = calc.eval(attribute.base_value());

        let attributes_mut = context.attribute_mut(self.who);
        // Apply the modifier
        if let Some(mut attribute) = attributes_mut.get_mut::<T>() {
            // Ensure that the modifier meaningfully changed the value before we trigger the event.
            let has_changed = new_val.are_different(attribute.current_value());
            if has_changed {
                attribute.set_base_value(new_val);
            }
            has_changed
        } else {
            panic!("Could not find attribute {}", type_name::<T>());
        }
    }

    fn apply_delayed(
        &self,
        source: Entity,
        target: Entity,
        effect: Entity,
        commands: &mut Commands,
    ) {
        commands.write_message(ApplyAttributeModifierMessage::<T> {
            source_entity: source,
            target_entity: target,
            effect_entity: effect,
            modifier: self.clone(),
        });
    }
}*/

/*impl<T: Attribute> Spawnable for AttributeModifier<T> {
    fn spawn(&self, commands: &mut EntityCommands) {
        commands.insert((
            AttributeModifier::<T> {
                value: self.value.clone(),
                who: self.who,
                operation: self.operation,
            },
            Name::new(format!("{}", self)),
        ));
    }
}*/
