use itertools::Itertools;
use std::{collections::HashMap, fmt::Display};

use bevy_ecs::prelude::*;

use crate::Description;

use super::{CommandPartId, UntypedCommandPartId};

pub struct CommandFormatDescriptionPart {
    pub id: Option<UntypedCommandPartId>,
    pub part_type: CommandFormatDescriptionPartType,
}

/// Describes what to include in the format description for a part.
#[derive(Default, Debug, PartialEq, Eq, Clone)]
pub enum CommandFormatDescriptionPartType {
    /// This part shouldn't be included in the format string.
    #[default]
    Nothing,
    /// A literal string, for parts of the format that must be entered literally to be matched (e.g. "get", "look", etc.)
    Literal(String),
    /// A placeholder for a target, (e.g. "thing", "target", etc.)
    Placeholder(String),
}

/// Describes the format of a command.
pub struct CommandFormatDescription {
    parts: Vec<CommandFormatDescriptionPart>,
    filled_placeholders: HashMap<UntypedCommandPartId, String>,
}

impl CommandFormatDescription {
    /// Creates a new format description with no placeholders filled in.
    pub fn new(parts: Vec<CommandFormatDescriptionPart>) -> CommandFormatDescription {
        CommandFormatDescription {
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

impl Display for CommandFormatDescription {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let string = self
            .parts
            .iter()
            .filter_map(|part| match &part.part_type {
                CommandFormatDescriptionPartType::Nothing => None,
                CommandFormatDescriptionPartType::Literal(l) => Some(l.to_string()),
                CommandFormatDescriptionPartType::Placeholder(p) => Some(
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
