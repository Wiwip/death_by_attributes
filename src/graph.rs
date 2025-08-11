use crate::condition::BoxExtractor;
use crate::prelude::*;
use bevy::ecs::system::SystemParam;
use bevy::prelude::*;
use petgraph::visit::Visitable;
use petgraph::visit::{GraphBase, IntoNeighbors};
use std::collections::HashSet;
use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};

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
        effect_entity: Entity,
    },
    ScalarModifier {
        modifier_entity: Entity,
        target_attribute: AttributeTypeId,
    },
    DerivedModifier {
        modifier_entity: Entity,
        dependency_entity: Entity,
        target_attribute: AttributeTypeId,
        extractor: BoxExtractor,
    },
}

// Lightweight wrapper that implements petgraph traits
#[derive(SystemParam)]
pub struct QueryGraphAdapter<'w, 's> {
    dependencies: Query<'w, 's, (Entity, &'static Effects)>,
}

impl GraphBase for QueryGraphAdapter<'_, '_> {
    type NodeId = Entity;
    type EdgeId = (Entity, Entity);
}

impl IntoNeighbors for &QueryGraphAdapter<'_, '_> {
    type Neighbors = std::vec::IntoIter<Entity>;

    fn neighbors(self, node: Self::NodeId) -> Self::Neighbors {
        match self.dependencies.get(node) {
            Ok((_, effects)) => effects.iter().collect::<Vec<Entity>>().into_iter(),
            Err(_) => vec![].into_iter(), // No child entities
        }
    }
}

impl Visitable for QueryGraphAdapter<'_, '_> {
    type Map = HashSet<Entity>;

    fn visit_map(&self) -> Self::Map {
        HashSet::new()
    }

    fn reset_map(&self, map: &mut Self::Map) {
        map.clear();
    }
}

/*#[derive(Component)]
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
        intensity: f32,
    ) -> NodeIndex {
        let target_idx = *self.entities.get(&target).unwrap();
        let effect_idx = self.graph.add_node(NodeType::Effect { effect_entity });

        self.entities.insert(effect_entity, effect_idx);

        // The effect applies to the entity
        self.graph.add_edge(effect_idx, target_idx, intensity);

        effect_idx
    }

    pub fn add_modifier_to(
        &mut self,
        modifier_entity: Entity,
        attribute: AttributeTypeId,
        entity: Entity,
    ) -> NodeIndex {
        let modifier_idx = self.graph.add_node(NodeType::ScalarModifier {
            modifier_entity,
            target_attribute: attribute,
        });
        self.graph
            .add_edge(modifier_idx, self.entities[&entity], 1.0);
        modifier_idx
    }

    pub fn add_derived_modifier_to<S: Attribute, T: Attribute>(
        &mut self,
        effect_entity: Entity,
        modifier_entity: Entity,
        dependency_entity: Entity,
    ) -> NodeIndex {
        let target_attribute = S::attribute_type_id();
        let extractor = BoxExtractor::new(AttributeExtractor::<T>::new());

        let modifier_idx = self.graph.add_node(NodeType::DerivedModifier {
            modifier_entity,
            target_attribute,
            dependency_entity,
            extractor,
        });

        self.graph
            .add_edge(modifier_idx, self.entities[&effect_entity], 1.0);
        modifier_idx
    }

    pub fn calculate_attribute_value<T: Attribute>(
        &self,
        base_value: f64,
        context: &Query<AttributesRef>,
    ) -> f64 {
        let target_attribute_id = T::attribute_type_id();
        let modifiers = self.collect_modifiers::<T>(&target_attribute_id, context);
        let calculator = AttributeCalculator::from(&modifiers);
        calculator.eval(base_value)
    }

    fn collect_modifiers<T: Attribute>(
        &self,
        attribute_id: &AttributeTypeId,
        context: &Query<AttributesRef>,
    ) -> Vec<Mod> {
        let mut final_modifiers = Vec::new();

        // Start recursive traversal from the entity root
        self.traverse_and_collect::<T>(
            self.entity_idx,
            attribute_id,
            context,
            1.0, // initial intensity
            &mut final_modifiers,
        );

        final_modifiers
    }

    fn traverse_and_collect<T: Attribute>(
        &self,
        node_idx: NodeIndex,
        attribute_id: &AttributeTypeId,
        context: &Query<AttributesRef>,
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

                    self.traverse_and_collect::<T>(
                        child_idx,
                        attribute_id,
                        context,
                        child_intensity,
                        final_modifiers,
                    );
                }
            }

            Some(NodeType::Effect { effect_entity }) => {
                // We skip inactive effects
                if !self.is_effect_active(context, *effect_entity).unwrap() {
                    return;
                }

                let Ok(effect_entity_ref) = context.get(*effect_entity) else {
                    panic!("Graph error: Effect {} not found", effect_entity);
                };
                let stacks = effect_entity_ref.get::<Stacks>().unwrap();

                // Multiply intensity and continue
                let effect_intensity = accumulated_intensity * (stacks.0 as f64);
                // Find modifiers that point TO this effect (incoming edges)
                for edge in self.graph.edges_directed(node_idx, petgraph::Incoming) {
                    let child_idx = edge.source(); // Use source instead of target
                    let edge_weight = *edge.weight() as f64;
                    let child_intensity = effect_intensity * edge_weight;

                    self.traverse_and_collect::<T>(
                        child_idx,
                        attribute_id,
                        context,
                        child_intensity,
                        final_modifiers,
                    );
                }
            }

            Some(NodeType::ScalarModifier {
                modifier_entity,
                target_attribute,
            }) => {
                // Leaf node - apply modifier if it matches
                if target_attribute == attribute_id {
                    let mod_entity_ref = context.get(*modifier_entity).unwrap();
                    let attribute_modifier = mod_entity_ref.get::<AttributeModifier<T>>().unwrap();

                    let amplified_modifier = attribute_modifier.modifier * accumulated_intensity;
                    final_modifiers.push(amplified_modifier);
                }
            }

            Some(NodeType::DerivedModifier {
                modifier_entity,
                target_attribute,
                dependency_entity,
                extractor,
            }) => {
                if target_attribute == attribute_id {
                    let mod_entity_ref = context.get(*modifier_entity).unwrap();
                    let attribute_modifier = mod_entity_ref
                        .get::<AttributeModifier<T>>()
                        .unwrap()
                        .modifier;

                    let Ok(other_entity_ref) = context.get(*dependency_entity) else {
                        error!("DerivedModifier: Entity {} not found", dependency_entity);
                        return;
                    };

                    let Ok(attribute_current_value) = extractor.0.extract_value(&other_entity_ref)
                    else {
                        error!(
                            "DerivedModifier: Could not extract value from entity {}",
                            dependency_entity
                        );
                        return;
                    };

                    println!("{} / {}", attribute_modifier, attribute_current_value);

                    let amplified_modifier =
                        attribute_modifier * accumulated_intensity * attribute_current_value;
                    final_modifiers.push(amplified_modifier);
                }
            }
            _ => {
                error!("Unknown node type");
            }
        }
    }

    /// Remove a node and all its edges
    pub fn remove_effect(&mut self, effect_entity: Entity) -> Option<NodeIndex> {
        if let Some(&node_idx) = self.entities.get(&effect_entity) {
            self.entities.remove(&effect_entity);
            self.graph.remove_node(node_idx);
            Some(node_idx)
        } else {
            None
        }
    }

    fn is_effect_active(
        &self,
        context: &Query<AttributesRef>,
        effect_entity: Entity,
    ) -> Result<bool, QueryEntityError> {
        let entity = context.get(effect_entity)?;
        Ok(!entity.contains::<EffectInactive>())
    }
}*/

#[cfg(test)]
mod tests {
    use super::*;
    use crate::actors::ActorBuilder;
    use crate::assets::ActorDef;
    use crate::context::EffectContext;
    use crate::{AttributesPlugin, ReflectAccessAttribute};
    use crate::{attribute, init_attribute};

    attribute!(Health);
    attribute!(Mana);
    attribute!(Strength);

    #[derive(Resource)]
    struct Definitions {
        actor_def: Handle<ActorDef>,
    }

    #[test]
    fn test_simple_modifier() {
        let mut app = App::new();
        init_app(&mut app);

        app.add_systems(
            Startup,
            (
                create_test_actor,
                |mut ctx: EffectContext, def: Res<Definitions>| {
                    let test_actor = ctx.spawn_actor(&def.actor_def).id();

                    let effect = EffectBuilder::permanent()
                        .modify::<Health>(Mod::Add(10.0), Who::Source)
                        .modify::<Mana>(Mod::More(1.0), Who::Source)
                        .build();

                    ctx.apply_dynamic_effect_to_self(test_actor, effect);
                },
            )
                .chain(),
        );

        app.add_systems(Update, |actor: Query<(&Health, &Mana)>| {
            let (health, mana) = actor.single().unwrap();
            assert!((health.current_value - 110.0).abs() < f64::EPSILON);
            assert!((mana.current_value - 2000.0).abs() < f64::EPSILON);
        });

        app.update();
    }

    fn init_app(app: &mut App) {
        app.add_plugins(MinimalPlugins)
            .add_plugins(AssetPlugin::default())
            .add_plugins(AttributesPlugin)
            .add_plugins((
                init_attribute::<Health>,
                init_attribute::<Mana>,
                init_attribute::<Strength>,
            ));
    }

    fn create_test_actor(mut actor_assets: ResMut<Assets<ActorDef>>, mut commands: Commands) {
        let actor_template = actor_assets.add(
            ActorBuilder::new()
                .with::<Strength>(12.0)
                .with::<Health>(100.0)
                .with::<Mana>(1000.0)
                .build(),
        );
        commands.insert_resource(Definitions {
            actor_def: actor_template,
        });
    }
}
