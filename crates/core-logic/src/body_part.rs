use std::collections::HashMap;

use bevy_ecs::prelude::*;

use rand::seq::SliceRandom;
use strum::EnumIter;

/// The body parts an entity has, grouped by type.
#[derive(Component)]
pub struct BodyParts(HashMap<BodyPartType, Vec<Entity>>);

/// An error returned when an entity is not a body part.
pub struct NotABodyPartError(pub Entity);

impl BodyParts {
    /// Creates a set of body parts from the provided body part entities.
    /// If any of the provided entities are not body parts, `Err(NotABodyPartError)` will be returned.
    pub fn new(parts: &[Entity], world: &World) -> Result<BodyParts, NotABodyPartError> {
        todo!() //TODO
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
    pub fn get<'w>(body_part_type: &BodyPartType, entity: Entity, world: &'w World) -> Vec<Entity> {
        world
            .get::<BodyParts>(entity)
            .and_then(|parts| parts.0.get(body_part_type).cloned())
            .unwrap_or_default()
    }

    /// Gets a random body part from the provided entity, if it has any body parts.
    pub fn random_weighted(entity: Entity, world: &World) -> Option<Entity> {
        let mut rng = rand::thread_rng();
        world
            .get::<BodyParts>(entity)
            //TODO allow different weights
            .map(|b| {
                b.0.values()
                    .flatten()
                    .collect::<Vec<&Entity>>()
                    .choose(&mut rng)
                    .map(|e| **e)
            })
            .unwrap_or_default()
    }
}
