use crate::{MetaCache};
use crate::attributes::AttributeComponent;
use crate::evaluators::MutatorEvaluator;
use crate::mutators::{EffectMutators, Mutator};
use crate::mutators::mutator::{ModAggregator, ModType};
use bevy::ecs::component::Mutable;
use bevy::log::{debug, info, warn};
use bevy::prelude::{Component, Event, Observer, Query, Reflect, RelationshipTarget, ResMut, Trigger};
use std::any::{TypeId, type_name};
use std::fmt::{Debug, Display, Formatter};
use std::marker::PhantomData;

/// Retrieves the value of an [AttributeDef][`crate::attributes::AttributeDef`] on an Attribute [`Component`]
/// to determine the magnitude of the evaluated mutator.
///
#[derive(Reflect)]
pub struct MetaEvaluator<A>
where
    A: Component<Mutability = Mutable> + AttributeComponent,
{
    magnitude: Option<f32>,
    scale: f32,
    _phantom: PhantomData<A>,
    mod_type: ModType,
}

impl<A> MetaEvaluator<A>
where
    A: Component<Mutability = Mutable> + AttributeComponent,
{
    pub fn new(scale: f32, mod_type: ModType) -> Self {
        Self {
            magnitude: None,
            mod_type,
            _phantom: Default::default(),
            scale,
        }
    }
}

impl<A> Clone for MetaEvaluator<A>
where
    A: Component<Mutability = Mutable> + AttributeComponent,
{
    fn clone(&self) -> Self {
        Self {
            magnitude: self.magnitude,
            scale: self.scale,
            _phantom: Default::default(),
            mod_type: self.mod_type,
        }
    }
}

impl<A> Debug for MetaEvaluator<A>
where
    A: AttributeComponent + Component<Mutability = Mutable>,
{
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("MetaEvaluator")
            .field("target", &type_name::<A>())
            .field("magnitude", &self.magnitude)
            .field("scale", &self.scale)
            .field("mod_type", &self.mod_type)
            .finish()
    }
}

impl<A> Display for MetaEvaluator<A>
where
    A: AttributeComponent + Component<Mutability = Mutable>,
{
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{} [{:?}*{}] {:?}",
            &type_name::<A>(),
            &self.magnitude,
            &self.scale,
            &self.mod_type
        )
    }
}

impl<A> MutatorEvaluator for MetaEvaluator<A>
where
    A: Component<Mutability = Mutable> + AttributeComponent,
{
    fn get_magnitude(&self) -> f32 {
        let Some(magnitude) = self.magnitude else {
            return 0.0;
        }; //.expect(format!("No magnitude set {}", type_name_of_val(self)).as_str());
        magnitude * self.scale
    }

    fn set_magnitude(&mut self, magnitude: f32) {
        self.magnitude = Some(magnitude)
    }

    fn get_aggregator(&self) -> ModAggregator {
        match self.mod_type {
            ModType::Additive => ModAggregator::additive(self.get_magnitude()),
            ModType::Multiplicative => ModAggregator::multiplicative(self.get_magnitude()),
            ModType::Overrule => ModAggregator::overrule(self.get_magnitude()),
        }
    }

    fn get_observer<O: Event, T: Component<Mutability = Mutable> + AttributeComponent>(
        &self,
    ) -> Option<Observer> {
        Some(Observer::new(meta_mutator_update::<A, O, T>))
    }
}

fn meta_mutator_update<A, O: Event, C>(
    trigger: Trigger<O>,
    mut attributes: Query<(&A, &mut C)>,
    entities: Query<&EffectMutators>,
    mut mutators: Query<&mut Mutator>,
    mut cached_mutations: ResMut<MetaCache>,
) where
    A: Component<Mutability = Mutable> + AttributeComponent, // The source attribute
    C: Component<Mutability = Mutable> + AttributeComponent, // The target attribute
{
    debug!(
        "meta_mutator_update: {} [{} from {}]",
        type_name::<A>(),
        trigger.target(),
        trigger.observer()
    );
    let actor_entity = trigger.target();
    let mutator_entity = trigger.observer();

    println!("A {}", type_name::<A>());
    println!("C {}", type_name::<C>());

    let Ok((src_attribute, mut target_attribute)) = attributes.get_mut(actor_entity) else {
        warn!(
            "Could not retrieve attribute {} from entity {}.",
            type_name::<A>(),
            actor_entity
        );
        return;
    };

    let Ok(mut mutator) = mutators.get_mut(mutator_entity) else {
        return;
    };

    let new_val = src_attribute.current_value();
    mutator.0.set_magnitude(new_val);

    info!(
        "MetaMutator: [{}] changed to [{}]",
        type_name::<A>(),
        new_val,
    );

    let (_, aggregator) = cached_mutations
        .entry((actor_entity, TypeId::of::<A>()))
        .or_insert((mutator.clone(), ModAggregator::default()));
    *aggregator = mutator.to_aggregator();

    // Get the mutators attached to the actor and update the attribute given their values
    let Ok(effect_mutators) = entities.get(actor_entity) else {
        return;
    };

    let mut aggregator = ModAggregator::default();
    for mutator_entity in effect_mutators.iter() {
        let Ok(mutator) = mutators.get(mutator_entity) else {
            continue;
        };

        aggregator += mutator.to_aggregator();
    }

    target_attribute.set_base_value(aggregator.evaluate(0.0));
}
