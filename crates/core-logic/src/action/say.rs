use std::{collections::HashSet, sync::LazyLock};

use bevy_ecs::prelude::*;
use nonempty::nonempty;

use crate::{
    any_text_part,
    command_format::CommandParseError,
    component::{ActionEndNotification, AfterActionPerformNotification},
    input_parser::InputParser,
    literal_part,
    notification::VerifyResult,
    one_of_part, ActionTag, BasicTokens, BeforeActionNotification, CommandFormat, CommandPartId,
    DynamicMessage, DynamicMessageLocation, MessageCategory, MessageDelay, MessageFormat,
    SurroundingsMessageCategory, VerifyActionNotification, World,
};

use super::{Action, ActionInterruptResult, ActionNotificationSender, ActionResult};

static TEXT_PART_ID: LazyLock<CommandPartId<String>> = LazyLock::new(|| CommandPartId::new("text"));
//TODO somehow get just "say" (no ending space) to result in a "Say what?" error rather than "I don't understand that"
static SAY_FORMAT: LazyLock<CommandFormat> = LazyLock::new(|| {
    CommandFormat::new(
        one_of_part(nonempty![literal_part("say "), literal_part("\"")])
            .with_error_string_override("say "),
    )
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
    ) -> Result<Box<dyn Action>, CommandParseError> {
        let parsed = SAY_FORMAT.parse(input, source_entity, world)?;

        Ok(Box::new(SayAction {
            text: parsed.get(&TEXT_PART_ID).to_string(),
            notification_sender: ActionNotificationSender::new(),
        }))
    }

    fn get_input_formats(&self) -> Vec<String> {
        vec![SAY_FORMAT.get_format_description().to_string()]
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
