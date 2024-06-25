use bevy_ecs::prelude::*;
use voca_rs::Voca;

use std::collections::{HashMap, HashSet};

use crate::{
    can_receive_messages, send_message, Container, Description, GameMessage, InterpolationError,
    Invisible, Location, MessageCategory, MessageDecoration, MessageDelay, MessageFormat,
    MessageTokens, Pronouns,
};

/// A message with receivers determined dynamically.
///
/// TODO do these fields all need to be public?
pub struct DynamicMessage<T: MessageTokens> {
    /// The format of the message.
    pub message_format: MessageFormat<T>,
    /// The tokens to use to interpolate the message.
    pub message_tokens: T,
    /// The category of the message.
    pub category: MessageCategory,
    /// The delay of the message.
    pub delay: MessageDelay,
    /// A list of entities to send the message to, if it should be sent to a limited set of them.
    pub receivers_override: Option<HashSet<Entity>>,
    /// A list of entities to not send the message to.
    pub receivers_to_exclude: HashSet<Entity>,
    /// Whether to send the message to the entity that generated it.
    pub send_to_source_entity: bool,
    /// The decorations to include alongside the message.
    pub decorations: Vec<MessageDecoration>,
}

impl<T: MessageTokens> DynamicMessage<T> {
    /// Creates a message with no specific receivers set, which won't be sent to the source entity.
    pub fn new_third_person(
        category: MessageCategory,
        delay: MessageDelay,
        message_format: MessageFormat<T>,
        message_tokens: T,
    ) -> DynamicMessage<T> {
        DynamicMessage {
            message_format,
            message_tokens,
            category,
            delay,
            receivers_override: None,
            receivers_to_exclude: HashSet::new(),
            send_to_source_entity: false,
            decorations: Vec::new(),
        }
    }

    /// Sets this message to be only sent to the provided entity.
    /// The provided entity will only receive the message if they already would have without this method being called.
    ///
    /// If an entity is passed to both `only_send_to` and `do_not_send_to`, the entity will not receive the message.
    ///
    /// Calling this multiple times will override any previous calls.
    pub fn only_send_to(mut self, entity: Entity) -> DynamicMessage<T> {
        self.receivers_override = Some(HashSet::from([entity]));

        self
    }

    /// Sets this message to be only sent to the provided entities.
    /// The provided entities will only receive the message if they already would have without this method being called.
    ///
    /// If an entity is passed to both `only_send_to_entities` and `do_not_send_to_entities`, the entity will not receive the message.
    ///
    /// Calling this multiple times will override any previous calls.
    pub fn only_send_to_entities(mut self, entities: &[Entity]) -> DynamicMessage<T> {
        self.receivers_override = Some(entities.iter().cloned().collect());

        self
    }

    /// Prevents this message from being sent to the provided entity.
    ///
    /// If an entity is passed to both `only_send_to` and `do_not_send_to`, the entity will not receive the message.
    ///
    /// Calling this multiple times will add more entities to not send messages to.
    pub fn do_not_send_to(mut self, entity: Entity) -> DynamicMessage<T> {
        self.receivers_to_exclude.insert(entity);

        self
    }

    /// Prevents this message from being sent to the provided entities.
    ///
    /// If an entity is passed to both `only_send_to_entities` and `do_not_send_to_entities`, the entity will not receive the message.
    ///
    /// Calling this multiple times will add more entities to not send messages to.
    pub fn do_not_send_to_entities(mut self, entities: &[Entity]) -> DynamicMessage<T> {
        self.receivers_to_exclude.extend(entities);

        self
    }

    /// Adds a decoration to the mesage.
    ///
    /// Calling this multiple times will add more decorations to the message.
    pub fn with_decoration(mut self, decoration: MessageDecoration) -> DynamicMessage<T> {
        self.decorations.push(decoration);

        self
    }

    /// Sends the message(s). No messages will be sent to `source_entity` if provided.
    pub fn send(
        self,
        source_entity: Option<Entity>,
        message_location: DynamicMessageLocation,
        world: &World,
    ) {
        for (entity, message) in self
            .into_game_messages(source_entity, message_location, world)
            .expect("message interpolation should not fail")
        {
            send_message(world, entity, message);
        }
    }

    /// Builds messages for entities in `location`, excluding `source_entity` if provided.
    pub fn into_game_messages(
        self,
        source_entity: Option<Entity>,
        message_location: DynamicMessageLocation,
        world: &World,
    ) -> Result<HashMap<Entity, GameMessage>, InterpolationError> {
        let mut message_map = HashMap::new();

        let location = match message_location {
            DynamicMessageLocation::SourceEntity => source_entity
                .and_then(|e| world.get::<Location>(e))
                .map(|loc| loc.id),
            DynamicMessageLocation::Location(e) => Some(e),
        };

        if let Some(location) = location {
            if let Some(container) = world.get::<Container>(location) {
                for entity in container.get_entities_including_invisible() {
                    // entities shouldn't see messages generated by entities they can't see (for example, a hidden player entering or leaving a room)
                    let can_see_source_entity = source_entity
                        .map(|e| Invisible::is_visible_to(e, *entity, world))
                        .unwrap_or(true);

                    if (Some(*entity) != source_entity || self.send_to_source_entity)
                        && can_see_source_entity
                        && !self.receivers_to_exclude.contains(entity)
                        && can_receive_messages(world, *entity)
                    {
                        if let Some(receivers_override) = &self.receivers_override {
                            if !receivers_override.contains(entity) {
                                // the receiving entities were overridden and this one isn't on the list, so skip it
                                continue;
                            }
                        }
                        message_map.insert(*entity, self.to_game_message_for(*entity, world)?);
                    }
                }
            }
        }

        Ok(message_map)
    }

    /// Builds a message for the provided entity.
    fn to_game_message_for(
        &self,
        pov_entity: Entity,
        world: &World,
    ) -> Result<GameMessage, InterpolationError> {
        Ok(GameMessage::Message {
            content: self
                .message_format
                .interpolate(pov_entity, &self.message_tokens, world)?,
            category: self.category,
            delay: self.delay,
            decorations: self.decorations.clone(),
        })
    }
}

/// The location to send a dynamic message in.
pub enum DynamicMessageLocation {
    /// The location of the entity that caused the message to be sent.
    SourceEntity,
    /// A specific location.
    Location(Entity),
}

/// A part of a third-person message.
///
/// TODO remove
pub enum MessagePart {
    /// A literal string.
    String(String),
    /// A token to be interpolated for each message recipient.
    Token(MessageToken),
}

/// A token to be interpolated for each message recipient.
pub enum MessageToken {
    /// The name of an entity.
    Name(Entity),
    /// The personal subject pronoun of an entity (e.g. he, she, they).
    PersonalSubjectPronoun { entity: Entity, capitalized: bool },
    /// The personal object pronoun of an entity (e.g. him, her, them).
    PersonalObjectPronoun(Entity),
    /// The possessive adjective pronoun of an entity (e.g. his, her, their).
    PossessiveAdjectivePronoun(Entity),
    /// The reflexive pronoun of an entity (e.g. himself, herself, themself).
    ReflexivePronoun(Entity),
    /// The form of "to be" to use with an entity's personal subject pronoun (i.e. is/are).
    ToBeForm(Entity),
}

impl MessageToken {
    /// Resolves the token to a string.
    fn to_string(&self, pov_entity: Entity, world: &World) -> String {
        match self {
            MessageToken::Name(e) => Description::get_reference_name(*e, Some(pov_entity), world),
            MessageToken::PersonalSubjectPronoun {
                entity,
                capitalized,
            } => {
                let pronoun = Pronouns::get_personal_subject(*entity, Some(pov_entity), world);

                if *capitalized {
                    pronoun._capitalize(false)
                } else {
                    pronoun
                }
            }
            MessageToken::PersonalObjectPronoun(e) => {
                Pronouns::get_personal_object(*e, Some(pov_entity), world)
            }
            MessageToken::PossessiveAdjectivePronoun(e) => {
                Pronouns::get_possessive_adjective(*e, Some(pov_entity), world)
            }
            MessageToken::ReflexivePronoun(e) => {
                Pronouns::get_reflexive(*e, Some(pov_entity), world)
            }
            MessageToken::ToBeForm(e) => Pronouns::get_to_be_form(*e, world),
        }
    }
}
