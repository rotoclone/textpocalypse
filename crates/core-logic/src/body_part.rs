use std::collections::HashMap;

use bevy_ecs::prelude::*;
use itertools::Itertools;

use rand::seq::SliceRandom;
use strum::EnumIter;

/// The body parts an entity has.
#[derive(Component)]
pub struct BodyParts(HashMap<BodyPartType, Vec<BodyPart>>);

impl BodyParts {
    /// Creates a set of the provided body parts.
    pub fn new(body_parts: Vec<BodyPart>) -> BodyParts {
        BodyParts(
            body_parts
                .into_iter()
                .into_group_map_by(|body_part| body_part.body_part_type.clone()),
        )
    }
}

/// A single body part of an entity.
#[derive(Debug, Clone)]
pub struct BodyPart {
    /// The display name of the body part.
    pub name: String,
    /// The type of body part this is.
    pub body_part_type: BodyPartType,
    /// Amount to multiply damage by for attacks that hit this body part.
    /// TODO should this be stored somewhere else?
    pub damage_multiplier: f32,
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

impl BodyPart {
    /// Gets all the body parts of a type from the provided entity.
    pub fn get<'w>(
        body_part_type: &BodyPartType,
        entity: Entity,
        world: &'w World,
    ) -> Vec<&'w BodyPart> {
        todo!() //TODO
    }

    /// Gets a random body part from the provided entity, if it has any body parts.
    pub fn random_weighted(entity: Entity, world: &World) -> Option<&BodyPart> {
        let mut rng = rand::thread_rng();
        world
            .get::<BodyParts>(entity)
            //TODO allow different weights
            .map(|b| {
                b.0.values()
                    .flatten()
                    .collect::<Vec<&BodyPart>>()
                    .choose(&mut rng)
                    // convert &&BodyPart to &BodyPart
                    .map(|body_part| *body_part)
            })
            .unwrap_or_default()
    }
}
