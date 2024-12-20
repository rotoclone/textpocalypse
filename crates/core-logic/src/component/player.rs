use std::{
    collections::HashMap,
    time::{Duration, SystemTime},
};

use bevy_ecs::prelude::*;
use flume::Sender;
use log::warn;
use strum::IntoEnumIterator;

use crate::{
    GameMessage, InternalMessageCategory, MessageCategory, SurroundingsMessageCategory, Time,
};

/// A unique identifier for a player.
#[derive(Debug, Clone, Copy, Hash, PartialEq, Eq)]
pub struct PlayerId(pub u32);

impl PlayerId {
    /// Increments this player ID.
    pub fn increment(self) -> PlayerId {
        PlayerId(self.0 + 1)
    }
}

#[derive(Component)]
pub struct Player {
    /// The unique ID of the player.
    pub id: PlayerId,
    /// The sender to use to send messages to the player.
    sender: Sender<(GameMessage, Time)>,
    /// Filter for messages to send to the player.
    pub message_filter: MessageFilter,
    /// The time this player last sent a command.
    pub last_command_time: SystemTime,
}

impl Player {
    /// Creates a player with the provided ID and message sender.
    pub fn new(id: PlayerId, sender: Sender<(GameMessage, Time)>) -> Player {
        Player {
            id,
            sender,
            message_filter: MessageFilter::new(),
            last_command_time: SystemTime::now(),
        }
    }

    /// Sends a message to this player. Logs and ignores any errors from the message sender.
    pub fn send_message(&self, message: GameMessage, time: Time) {
        if self.message_filter.accept(&message) {
            if let Err(e) = self.sender.send((message, time)) {
                warn!("Error sending message to player {:?}: {}", self.id, e);
            }
        }
    }

    /// Determines whether this player is AFK.
    pub fn is_afk(&self, afk_timeout: Option<Duration>) -> bool {
        if let Some(afk_timeout) = afk_timeout {
            match SystemTime::now().duration_since(self.last_command_time) {
                Ok(elapsed) => elapsed >= afk_timeout,
                Err(_) => false,
            }
        } else {
            false
        }
    }
}

/// Keeps track of messages to be filtered.
pub struct MessageFilter {
    /// The number of active filters for different message categories.
    categories: HashMap<MessageCategory, usize>,
}

impl MessageFilter {
    /// Creates a message filter that doesn't filter any messages.
    pub fn new() -> MessageFilter {
        MessageFilter {
            categories: HashMap::new(),
        }
    }

    /// Adds filters for all `MessageCategory::Internal` messages.
    #[expect(unused)]
    pub fn filter_all_internal(&mut self) -> &mut Self {
        for category in InternalMessageCategory::iter() {
            self.filter(MessageCategory::Internal(category));
        }

        self
    }

    /// Removes filters for all `MessageCategory::Internal` messages.
    #[expect(unused)]
    pub fn unfilter_all_internal(&mut self) -> &mut Self {
        for category in InternalMessageCategory::iter() {
            self.unfilter(MessageCategory::Internal(category));
        }

        self
    }

    /// Adds filters for all `MessageCategory::Surroundings` messages.
    pub fn filter_all_surroundings(&mut self) -> &mut Self {
        self.filter_all_surroundings_except(&[])
    }

    /// Adds filters for all `MessageCategory::Surroundings` messages except ones in the provided list.
    pub fn filter_all_surroundings_except(
        &mut self,
        exceptions: &[SurroundingsMessageCategory],
    ) -> &mut Self {
        for category in SurroundingsMessageCategory::iter() {
            if !exceptions.contains(&category) {
                self.filter(MessageCategory::Surroundings(category));
            }
        }

        self
    }

    /// Removes filters for all `MessageCategory::Surroundings` messages.
    pub fn unfilter_all_surroundings(&mut self) -> &mut Self {
        self.unfilter_all_surroundings_except(&[])
    }

    /// Removes filters for all `MessageCategory::Surroundings` messages except ones in the provided list.
    pub fn unfilter_all_surroundings_except(
        &mut self,
        exceptions: &[SurroundingsMessageCategory],
    ) -> &mut Self {
        for category in SurroundingsMessageCategory::iter() {
            if !exceptions.contains(&category) {
                self.unfilter(MessageCategory::Surroundings(category));
            }
        }

        self
    }

    /// Adds a filter for messages in the provided category.
    pub fn filter(&mut self, category: MessageCategory) -> &mut Self {
        *self.categories.entry(category).or_insert(0) += 1;

        self
    }

    /// Removes a filter for messages in the provided category.
    pub fn unfilter(&mut self, category: MessageCategory) -> &mut Self {
        let active_filters = self.categories.entry(category).or_insert(0);
        *active_filters = active_filters.saturating_sub(1);

        self
    }

    /// Returns `false` if the message should be filtered, `true` otherwise.
    pub fn accept(&self, message: &GameMessage) -> bool {
        let category = match message {
            GameMessage::Message { category, .. } => category,
            _ => return true,
        };

        if self.categories.get(category).unwrap_or(&0) > &0 {
            return false;
        }

        true
    }
}
