use bevy_ecs::prelude::*;

use std::collections::{HashMap, HashSet};

use crate::{
    can_receive_messages, send_message, Container, GameMessage, InterpolationError, Invisible,
    Location, MessageCategory, MessageDecoration, MessageDelay, MessageFormat, MessageTokens,
};

/// A message with receivers determined dynamically.
pub struct DynamicMessage<T: MessageTokens> {
    /// The format of the message.
    message_format: MessageFormat<T>,
    /// The tokens to use to interpolate the message.
    message_tokens: T,
    /// The category of the message.
    category: MessageCategory,
    /// The delay of the message.
    delay: MessageDelay,
    /// A list of entities to send the message to, if it should be sent to a limited set of them.
    receivers_override: Option<HashSet<Entity>>,
    /// A list of entities to not send the message to.
    receivers_to_exclude: HashSet<Entity>,
    /// Whether to send the message to the entity that generated it.
    send_to_source_entity: bool,
    /// The decorations to include alongside the message.
    decorations: Vec<MessageDecoration>,
}

impl<T: MessageTokens> DynamicMessage<T> {
    /// Creates a message with no specific receivers set, which will be sent to the source entity.
    ///
    /// The category will be converted into in internal message category for the message sent to the source entity.
    pub fn new(
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
            send_to_source_entity: true,
            decorations: Vec::new(),
        }
    }

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
                        message_map.insert(
                            *entity,
                            self.to_game_message_for(*entity, source_entity, world)?,
                        );
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
        source_entity: Option<Entity>,
        world: &World,
    ) -> Result<GameMessage, InterpolationError> {
        let category = if Some(pov_entity) == source_entity {
            self.category.into_internal()
        } else {
            self.category
        };

        Ok(GameMessage::Message {
            content: self
                .message_format
                .interpolate(pov_entity, &self.message_tokens, world)?,
            category,
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
