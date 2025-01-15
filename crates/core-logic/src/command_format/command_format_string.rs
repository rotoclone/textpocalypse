use itertools::Itertools;
use std::{collections::HashMap, fmt::Display};

use bevy_ecs::prelude::*;

use crate::Description;

use super::{CommandPartId, UntypedCommandPartId};

pub struct CommandFormatStringPart {
    pub id: Option<UntypedCommandPartId>,
    pub part_type: CommandFormatStringPartType,
}

/// Describes what to include in the format string for a part.
#[derive(Default, Debug, PartialEq, Eq, Clone)]
pub enum CommandFormatStringPartType {
    /// This part shouldn't be included in the format string.
    #[default]
    Nothing,
    /// A literal string, for parts of the format that must be entered literally to be matched (e.g. "get", "look", etc.)
    Literal(String),
    /// A placeholder for a target, (e.g. "thing", "target", etc.)
    Placeholder(String),
}

pub struct CommandFormatString {
    parts: Vec<CommandFormatStringPart>,
    filled_placeholders: HashMap<UntypedCommandPartId, String>,
}

impl CommandFormatString {
    /// Creates a new format string with no placeholders filled in.
    pub fn new(parts: Vec<CommandFormatStringPart>) -> CommandFormatString {
        CommandFormatString {
            parts,
            filled_placeholders: HashMap::new(),
        }
    }

    /// Fills the placeholder for the part with the provided ID with the name of the provided entity.
    /// Does nothing if the entity has no name.
    pub fn with_targeted_entity(
        mut self,
        id: CommandPartId<Entity>,
        entity: Entity,
        world: &World,
    ) -> Self {
        if let Some(name) = Description::get_name(entity, world) {
            self.filled_placeholders.insert(id.into(), name);
        }

        self
    }
}

impl Display for CommandFormatString {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let string = self
            .parts
            .iter()
            .filter_map(|part| match &part.part_type {
                CommandFormatStringPartType::Nothing => None,
                CommandFormatStringPartType::Literal(l) => Some(l.to_string()),
                CommandFormatStringPartType::Placeholder(p) => Some(
                    part.id
                        .as_ref()
                        .and_then(|id| self.filled_placeholders.get(id).cloned())
                        .unwrap_or_else(|| format!("<{p}>")),
                ),
            })
            .join("");

        string.fmt(f)
    }
}
