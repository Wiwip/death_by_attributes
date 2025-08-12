use crate::attributes::Attribute;
use crate::prelude::{AttributeModifier, Mod};
use bevy::ecs::component::Mutable;
use bevy::prelude::{Component, Entity, Query};

// /// Aggregates all modifiers for a given entity, returning a combined ModAggregator
/*pub fn collect_entity_modifiers<'a, T: Component<Mutability = Mutable> + Attribute>(
    entity: Entity,
    //modifiers_query: &Query<&Modifiers>,
    attribute_modifier_query: &Query<&AttributeModifier<T>>,
) -> impl Iterator<Item = Mod> {
    // Try to get modifiers for the entity
    modifiers_query.get(entity)
        .into_iter()
        .flat_map(|modifiers| &modifiers.0)
        .filter_map(|&modifier_entity| {
            attribute_modifier_query.get(modifier_entity).ok()
        })
        .map(|modifier| modifier.modifier)
}*/
