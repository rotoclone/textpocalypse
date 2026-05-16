use std::{collections::HashSet, sync::LazyLock};

use bevy_ecs::prelude::*;
use core_logic_derive::ActionBoilerplate;
use nonempty::nonempty;

use crate::{
    command_format::{one_of_literal_part, CommandFormat},
    component::Stats,
    input_parser::{InputParseError, InputParser},
    ActionTag, GameMessage, StatsDescription, World,
};

use super::{Action, ActionInterruptResult, ActionNotificationSender, ActionResult};

static STATS_FORMAT: LazyLock<CommandFormat> =
    LazyLock::new(|| CommandFormat::new(one_of_literal_part(nonempty!["stats", "stat", "st"])));

pub struct StatsParser;

impl InputParser for StatsParser {
    fn parse(
        &self,
        input: &str,
        source_entity: Entity,
        world: &World,
    ) -> Result<Box<dyn Action>, InputParseError> {
        STATS_FORMAT.parse(input, source_entity, world)?;
        Ok(Box::new(StatsAction {
            notification_sender: ActionNotificationSender::new(),
        }))
    }

    fn get_input_formats(&self) -> Vec<String> {
        vec![STATS_FORMAT.get_format_description().to_string()]
    }

    fn get_input_formats_for(&self, _: Entity, _: Entity, _: &World) -> Vec<String> {
        Vec::new()
    }
}

/// Shows an entity its stats.
#[derive(ActionBoilerplate, Debug)]
struct StatsAction {
    notification_sender: ActionNotificationSender<Self>,
}

impl Action for StatsAction {
    fn perform(&mut self, performing_entity: Entity, world: &mut World) -> ActionResult {
        if let Some(stats) = world.get::<Stats>(performing_entity) {
            let message = GameMessage::Stats(StatsDescription::from_stats(stats, world));

            ActionResult::builder()
                .with_game_message(performing_entity, message)
                .build_complete_no_tick(true)
        } else {
            ActionResult::error(performing_entity, "You have no stats.".to_string())
        }
    }

    fn interrupt(&self, _: Entity, _: &mut World) -> ActionInterruptResult {
        ActionInterruptResult::none()
    }

    fn may_require_tick(&self) -> bool {
        false
    }

    fn get_tags(&self) -> HashSet<ActionTag> {
        [].into()
    }

    fn get_interaction_target(&self, _: &World) -> Option<Entity> {
        None
    }
}
