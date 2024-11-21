use std::collections::HashSet;

use bevy_ecs::prelude::*;

use rand::seq::SliceRandom;
use strum::EnumIter;

/// The body parts an entity has.
#[derive(Component)]
pub struct BodyParts(pub HashSet<Entity>);

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
    pub fn get<'w>(
        body_part_type: &BodyPartType,
        entity: Entity,
        world: &'w World,
    ) -> Vec<&'w BodyPart> {
        todo!() //TODO
    }

    /// Gets a random body part from the provided entity, if it has any body parts.
    pub fn random_weighted(entity: Entity, world: &World) -> Option<(Entity, &BodyPart)> {
        let mut rng = rand::thread_rng();
        world
            .get::<BodyParts>(entity)
            //TODO allow different weights
            .map(|b| {
                b.0.iter()
                    .map(|e| {
                        (
                            *e,
                            world
                                .get::<BodyPart>(*e)
                                .expect("body part should be a body part"),
                        )
                    })
                    .collect::<Vec<(Entity, &BodyPart)>>()
                    .choose(&mut rng)
                    .map(|tuple| *tuple)
            })
            .unwrap_or_default()
    }
}
