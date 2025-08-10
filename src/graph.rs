use crate::AttributesMut;
use crate::prelude::*;
use bevy::ecs::component::ComponentId;
use bevy::platform::collections::HashMap;
use bevy::prelude::*;
use petgraph::algo::toposort;
pub(crate) use petgraph::data::Build;
use petgraph::graph::NodeIndex;
use petgraph::prelude::*;
use petgraph::{Directed, Graph};
use std::any::{type_name, type_name_of_val, TypeId};
use std::collections::hash_map::DefaultHasher;
use std::fmt::Formatter;
use std::hash::{Hash, Hasher};
use crate::inspector::pretty_type_name_str;

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

/// Attributes are Components and inserted on Entities.
/// - Derived attributes could be used
/// Effects are spawned as child of entities
/// - Effects can modify the modifiers till now, such as an intensity metric or stacks
/// Modifiers are added to Effects as Vec<Mod>
/// - Modifiers must apply to an attribute

#[derive(Debug)]
pub enum NodeType {
    Entity,
    Effect {
        intensity: f64,
        stacks: u32,
    },
    Modifier {
        attribute: AttributeTypeId,
        modifier: Mod,
    },
}

#[derive(Component)]
pub struct EntityGraph {
    pub entity_idx: NodeIndex,
    pub graph: StableGraph<NodeType, f32, Directed>,
    pub entities: HashMap<Entity, NodeIndex>,
}

impl EntityGraph {
    pub fn new(entity: Entity) -> Self {
        let mut graph = StableGraph::new();
        let entity_idx = graph.add_node(NodeType::Entity);
        let mut entities = HashMap::default();
        entities.insert(entity, entity_idx);

        Self {
            entity_idx,
            graph,
            entities,
        }
    }

    pub fn add_effect_to(
        &mut self,
        effect_entity: Entity,
        target: Entity,
        intensity: f64,
    ) -> NodeIndex {
        let target_idx = *self.entities.get(&target).unwrap();
        let effect_idx = self.graph.add_node(NodeType::Effect {
            intensity,
            stacks: 1,
        });

        self.entities.insert(effect_entity, effect_idx);

        // The effect applies to the entity
        self.graph.add_edge(effect_idx, target_idx, 1.0);

        effect_idx
    }

    pub fn add_modifier_to(
        &mut self,
        entity: Entity,
        attribute: AttributeTypeId,
        modifier: Mod,
    ) -> NodeIndex {
        let modifier_idx = self.graph.add_node(NodeType::Modifier {
            attribute,
            modifier,
        });
        self.graph
            .add_edge(modifier_idx, self.entities[&entity], 1.0);
        modifier_idx
    }

    pub fn calculate_attribute_value<T: Attribute>(&self, base_value: f64) -> f64 {
        let target_attribute_id = T::attribute_type_id();
        let modifiers = self.collect_modifiers_single_pass(&target_attribute_id);
        let calculator = AttributeCalculator::from(&modifiers);
        calculator.eval(base_value)
    }

    fn collect_modifiers_single_pass(&self, attribute_id: &AttributeTypeId) -> Vec<Mod> {
        let mut final_modifiers = Vec::new();

        // Start recursive traversal from the entity root
        self.traverse_and_collect(
            self.entity_idx,
            attribute_id,
            1.0, // initial intensity
            &mut final_modifiers,
        );

        final_modifiers
    }

    fn traverse_and_collect(
        &self,
        node_idx: NodeIndex,
        attribute_id: &AttributeTypeId,
        accumulated_intensity: f64,
        final_modifiers: &mut Vec<Mod>,
    ) {
        match self.graph.node_weight(node_idx) {
            Some(NodeType::Entity) => {
                // Find effects that point TO this entity (incoming edges)
                for edge in self.graph.edges_directed(node_idx, petgraph::Incoming) {
                    let child_idx = edge.source(); // Use source instead of target
                    let edge_weight = *edge.weight() as f64;
                    let child_intensity = accumulated_intensity * edge_weight;

                    self.traverse_and_collect(
                        child_idx,
                        attribute_id,
                        child_intensity,
                        final_modifiers,
                    );
                }
            }

            Some(NodeType::Effect { intensity, stacks }) => {
                // Multiply intensity and continue
                let effect_intensity = accumulated_intensity * intensity * (*stacks as f64);
                // Find modifiers that point TO this effect (incoming edges)
                for edge in self.graph.edges_directed(node_idx, petgraph::Incoming) {
                    let child_idx = edge.source(); // Use source instead of target
                    let edge_weight = *edge.weight() as f64;
                    let child_intensity = effect_intensity * edge_weight;

                    self.traverse_and_collect(
                        child_idx,
                        attribute_id,
                        child_intensity,
                        final_modifiers,
                    );
                }
            }

            Some(NodeType::Modifier {
                attribute,
                modifier,
            }) => {
                // Leaf node - apply modifier if it matches
                if attribute == attribute_id {
                    let amplified_modifier = *modifier * accumulated_intensity;
                    final_modifiers.push(amplified_modifier);
                }
            }
            _ => {
                error!("Unknown node type");
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ReflectAccessAttribute;
    use crate::attribute;
    use petgraph::dot::Dot;

    attribute!(Health);
    attribute!(Mana);

    #[test]
    fn test_graph() {
        let mut world = World::default();

        let player_entity = world.spawn_empty().id();
        let mut player_graph = EntityGraph::new(player_entity);

        let effect_entity_1 = world.spawn_empty().id();
        player_graph.add_effect_to(effect_entity_1, player_entity, 1.5);
        player_graph.add_modifier_to(effect_entity_1, Health::attribute_type_id(), Mod::Add(10.0));
        player_graph.add_modifier_to(effect_entity_1, Mana::attribute_type_id(), Mod::Add(100.0));
        player_graph.add_modifier_to(effect_entity_1, Mana::attribute_type_id(), Mod::More(1.00));

        let sub_effect_entity_1 = world.spawn_empty().id();
        player_graph.add_effect_to(sub_effect_entity_1, effect_entity_1, 2.0);
        player_graph.add_modifier_to(sub_effect_entity_1, Health::attribute_type_id(), Mod::Add(50.0));

        let effect_entity_2 = world.spawn_empty().id();
        player_graph.add_effect_to(effect_entity_2, player_entity, 1.0);
        player_graph.add_modifier_to(effect_entity_2, Health::attribute_type_id(), Mod::Add(10.0));

        println!("{:?}", Dot::new(&player_graph.graph));

        println!(
            "Health: {}",
            player_graph.calculate_attribute_value::<Health>(100.0)
        );
        println!("Mana: {}", player_graph.calculate_attribute_value::<Mana>(100.0));
    }
}
