use std::collections::HashMap;

use bevy_ecs::prelude::*;

use rand_distr::{Distribution, WeightedError, WeightedIndex};
use strum::EnumIter;

/// The body parts an entity has.
#[derive(Component)]
pub struct BodyParts {
    /// Map of body part types to entities of that type.
    type_to_entities: HashMap<BodyPartType, Vec<Entity>>,
    /// Weights to use when randomly choosing a body part.
    weights: BodyPartWeights,
}

impl BodyParts {
    /// Gets all the body parts.
    pub fn get_all(&self) -> Vec<Entity> {
        self.type_to_entities.values().flatten().cloned().collect()
    }
}

/// An error building an instance of `BodyParts`.
pub enum BodyPartsConstructorError {
    /// No body parts were provided
    NoBodyParts,
    /// An entity is not a body part
    NotABodyPart(Entity),
    /// The provided weights are invalid
    InvalidWeights(WeightedError),
}

impl From<WeightedError> for BodyPartsConstructorError {
    fn from(value: WeightedError) -> Self {
        BodyPartsConstructorError::InvalidWeights(value)
    }
}

impl BodyParts {
    /// Creates a set of body parts from the provided body part entities.
    ///
    /// ## Errors
    /// * If `part_to_weight` is empty, `Err(BodyPartsConstructorError::NoBodyParts)` will be returned.
    /// * If any of the provided entities are not body parts, `Err(BodyPartsConstructorError::NotABodyPart)` will be returned.
    /// * If the provided weights are invalid (e.g. if any of them are negative, or they sum to 0), `Err(BodyPartsConstructorError::InvalidWeights)` will be returned.
    pub fn new(
        part_to_weight: HashMap<Entity, f32>,
        world: &World,
    ) -> Result<BodyParts, BodyPartsConstructorError> {
        if part_to_weight.is_empty() {
            return Err(BodyPartsConstructorError::NoBodyParts);
        }

        let mut type_to_entities: HashMap<BodyPartType, Vec<Entity>> = HashMap::new();
        for body_part_entity in part_to_weight.keys() {
            if let Some(body_part) = world.get::<BodyPart>(*body_part_entity) {
                type_to_entities
                    .entry(body_part.part_type.clone())
                    .or_default()
                    .push(*body_part_entity);
            } else {
                return Err(BodyPartsConstructorError::NotABodyPart(*body_part_entity));
            }
        }

        Ok(BodyParts {
            type_to_entities,
            weights: BodyPartWeights::new(part_to_weight)?,
        })
    }
}

#[derive(Component)]
pub struct BodyPartWeights {
    body_parts: Vec<Entity>,
    dist: WeightedIndex<f32>,
}

impl BodyPartWeights {
    /// Initializes the weights. Returns an error if the weights are invalid per `WeightedIndex::new`.
    /// Note: it is assumed that all the keys in the provided map are body parts.
    fn new(part_to_weight: HashMap<Entity, f32>) -> Result<BodyPartWeights, WeightedError> {
        let (body_parts, weights) = part_to_weight.iter().unzip();

        Ok(BodyPartWeights {
            body_parts,
            dist: WeightedIndex::new::<Vec<f32>>(weights)?,
        })
    }

    /// Gets a random body part entity.
    pub fn get_random(&self) -> Entity {
        self.body_parts[self.dist.sample(&mut rand::thread_rng())]
    }
}

/// A single body part of an entity.
#[derive(Debug, Clone, Component)]
pub struct BodyPart {
    /// The type of body part this is.
    pub part_type: BodyPartType,
}

/// Defines the different types of body part.
#[derive(Debug, Clone, PartialEq, Eq, Hash, EnumIter)] //TODO remove EnumIter?
pub enum BodyPartType {
    Head,
    Torso,
    LeftArm,
    RightArm,
    LeftHand,
    RightHand,
    LeftLeg,
    RightLeg,
    LeftFoot,
    RightFoot,
    Custom(String),
}

/// Amount to multiply damage by for attacks that hit this body part.
#[derive(Component, Debug)]
pub struct BodyPartDamageMultiplier(pub f32);

impl BodyPart {
    /// Gets all the body parts of a type from the provided entity.
    pub fn get(body_part_type: &BodyPartType, entity: Entity, world: &World) -> Vec<Entity> {
        world
            .get::<BodyParts>(entity)
            .and_then(|parts| parts.type_to_entities.get(body_part_type).cloned())
            .unwrap_or_default()
    }

    /// Gets a random body part from the provided entity, if it has any body parts.
    pub fn random_weighted(entity: Entity, world: &World) -> Option<Entity> {
        world
            .get::<BodyParts>(entity)
            .map(|b| b.weights.get_random())
    }
}
