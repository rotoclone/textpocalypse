use std::collections::HashSet;

use bevy_ecs::prelude::*;
use log::debug;

/// The description of an entity.
#[derive(Component, Debug)]
pub struct Description {
    /// The name of the entity.
    pub name: String,
    /// The name to use when referring to the entity as part of a room description.
    pub room_name: String,
    /// The article to use when referring to the entity (usually "a" or "an")
    pub article: Option<String>,
    /// The alternate names of the entity.
    pub aliases: HashSet<String>,
    /// The description of the entity.
    pub description: String,
}

impl Description {
    /// Determines whether the provided input refers to the entity with this description.
    pub fn matches(&self, input: &str) -> bool {
        debug!("Checking if {input:?} matches {self:?}");
        self.name.eq_ignore_ascii_case(input)
            || self.room_name.eq_ignore_ascii_case(input)
            || self.aliases.contains(input)
    }
}
