use std::{collections::HashSet, sync::LazyLock};

use bevy_ecs::prelude::*;
use nonempty::nonempty;

use crate::{
    any_text_part,
    command_format::PartParserContext,
    component::{ActionEndNotification, AfterActionPerformNotification},
    input_parser::{InputParseError, InputParser},
    literal_part,
    notification::VerifyResult,
    one_of_part, send_message, ActionTag, BasicTokens, BeforeActionNotification, CommandFormat,
    CommandPartId, DynamicMessage, DynamicMessageLocation, MessageCategory, MessageDelay,
    MessageFormat, SurroundingsMessageCategory, VerifyActionNotification, World,
};

use super::{Action, ActionInterruptResult, ActionNotificationSender, ActionResult};

static TEXT_PART_ID: LazyLock<CommandPartId<String>> = LazyLock::new(|| CommandPartId::new("text"));
static SAY_COMMAND_FORMAT: LazyLock<CommandFormat> = LazyLock::new(|| {
    CommandFormat::new(one_of_part(nonempty![
        literal_part("say ").always_include_in_errors().into(),
        literal_part("\"").never_include_in_errors().into()
    ]))
    .then(
        any_text_part(TEXT_PART_ID.clone())
            .with_if_missing("what")
            .with_placeholder_for_format_string("statement"),
    )
});

pub struct SayParser;

impl InputParser for SayParser {
    fn parse(
        &self,
        input: &str,
        source_entity: Entity,
        world: &World,
    ) -> Result<Box<dyn Action>, InputParseError> {
        //TODO use `?` instead
        let parsed = match SAY_COMMAND_FORMAT.parse(input, source_entity, world) {
            Ok(p) => p,
            Err(e) => {
                //TODO don't send message directly here
                send_message(
                    world,
                    source_entity,
                    e.into_message(
                        PartParserContext {
                            input: input.to_string(),
                            entering_entity: source_entity,
                            next_part: None,
                        },
                        world,
                    ),
                );
                return Err(InputParseError::UnknownCommand);
            }
        };

        Ok(Box::new(SayAction {
            text: parsed.get(&TEXT_PART_ID).to_string(),
            notification_sender: ActionNotificationSender::new(),
        }))
    }

    fn get_input_formats(&self) -> Vec<String> {
        vec![SAY_COMMAND_FORMAT.get_format_string().to_string()]
    }

    fn get_input_formats_for(&self, _: Entity, _: Entity, _: &World) -> Option<Vec<String>> {
        None
    }
}

/// Makes an entity say something.
#[derive(Debug)]
pub struct SayAction {
    pub text: String,
    pub notification_sender: ActionNotificationSender<Self>,
}

impl Action for SayAction {
    fn perform(&mut self, performing_entity: Entity, world: &mut World) -> ActionResult {
        let text = &self.text;

        ActionResult::builder()
            .with_dynamic_message(
                Some(performing_entity),
                DynamicMessageLocation::SourceEntity,
                DynamicMessage::new(
                    MessageCategory::Surroundings(SurroundingsMessageCategory::Speech),
                    MessageDelay::Short,
                    MessageFormat::new(
                        "${performing_entity.Name} ${performing_entity.you:say/says}, \"${text}\"",
                    )
                    .expect("message format should be valid"),
                    BasicTokens::new()
                        .with_entity("performing_entity".into(), performing_entity)
                        .with_string("text".into(), text.clone()),
                ),
                world,
            )
            .build_complete_no_tick(true)
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

    fn send_before_notification(
        &self,
        notification_type: BeforeActionNotification,
        world: &mut World,
    ) {
        self.notification_sender
            .send_before_notification(notification_type, self, world);
    }

    fn send_verify_notification(
        &self,
        notification_type: VerifyActionNotification,
        world: &mut World,
    ) -> VerifyResult {
        self.notification_sender
            .send_verify_notification(notification_type, self, world)
    }

    fn send_after_perform_notification(
        &self,
        notification_type: AfterActionPerformNotification,
        world: &mut World,
    ) {
        self.notification_sender
            .send_after_perform_notification(notification_type, self, world);
    }

    fn send_end_notification(&self, notification_type: ActionEndNotification, world: &mut World) {
        self.notification_sender
            .send_end_notification(notification_type, self, world);
    }
}
