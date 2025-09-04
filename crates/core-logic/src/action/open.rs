use std::{collections::HashSet, sync::LazyLock};

use bevy_ecs::prelude::*;

use crate::{
    command_format::{
        entity_part_builder, literal_part, validate_parsed_value_has_component, CommandFormat,
        CommandPartId,
    },
    component::{ActionEndNotification, AfterActionPerformNotification, OpenState},
    input_parser::{input_formats_if_has_component, InputParseError, InputParser},
    notification::VerifyResult,
    ActionTag, BasicTokens, BeforeActionNotification, Description, DynamicMessage,
    DynamicMessageLocation, InternalMessageCategory, MessageCategory, MessageDelay, MessageFormat,
    SurroundingsMessageCategory, VerifyActionNotification,
};

use super::{Action, ActionInterruptResult, ActionNotificationSender, ActionResult};

static TARGET_PART_ID: LazyLock<CommandPartId<Entity>> =
    LazyLock::new(|| CommandPartId::new("target"));
static OPEN_FORMAT: LazyLock<CommandFormat> = LazyLock::new(|| {
    CommandFormat::new(literal_part("open"))
        .then(literal_part(" ").always_include_in_errors())
        .then(
            entity_part_builder(TARGET_PART_ID.clone())
                .with_validator(|context, world| {
                    validate_parsed_value_has_component::<OpenState>(context, "open", world)
                })
                .build()
                .always_include_in_errors()
                .with_if_unparsed("what")
                .with_placeholder_for_format_string("thing"),
        )
});
static CLOSE_FORMAT: LazyLock<CommandFormat> = LazyLock::new(|| {
    CommandFormat::new(literal_part("close"))
        .then(literal_part(" ").always_include_in_errors())
        .then(
            entity_part_builder(TARGET_PART_ID.clone())
                .with_validator(|context, world| {
                    validate_parsed_value_has_component::<OpenState>(context, "close", world)
                })
                .build()
                .always_include_in_errors()
                .with_if_unparsed("what")
                .with_placeholder_for_format_string("thing"),
        )
});

//TODO convert other multi-format commands to use multiple parsers and encapsulate getting all the sub-parsers into a function or something so they don't have to all be manually added in `lib.rs`
pub struct OpenParser;
pub struct CloseParser;

impl InputParser for OpenParser {
    fn parse(
        &self,
        input: &str,
        source_entity: Entity,
        world: &World,
    ) -> Result<Box<dyn Action>, InputParseError> {
        let parsed = OPEN_FORMAT.parse(input, source_entity, world)?;
        Ok(Box::new(OpenAction {
            target: parsed.get(&TARGET_PART_ID),
            should_be_open: true,
            notification_sender: ActionNotificationSender::new(),
        }))
    }

    fn get_input_formats(&self) -> Vec<String> {
        vec![OPEN_FORMAT.get_format_description().to_string()]
    }

    fn get_input_formats_for(&self, entity: Entity, _: Entity, world: &World) -> Vec<String> {
        input_formats_if_has_component::<OpenState>(
            entity,
            world,
            &[OPEN_FORMAT.get_format_description().with_targeted_entity(
                TARGET_PART_ID.clone(),
                entity,
                world,
            )],
        )
    }
}

impl InputParser for CloseParser {
    fn parse(
        &self,
        input: &str,
        source_entity: Entity,
        world: &World,
    ) -> Result<Box<dyn Action>, InputParseError> {
        let parsed = CLOSE_FORMAT.parse(input, source_entity, world)?;
        Ok(Box::new(OpenAction {
            target: parsed.get(&TARGET_PART_ID),
            should_be_open: false,
            notification_sender: ActionNotificationSender::new(),
        }))
    }

    fn get_input_formats(&self) -> Vec<String> {
        vec![CLOSE_FORMAT.get_format_description().to_string()]
    }

    fn get_input_formats_for(&self, entity: Entity, _: Entity, world: &World) -> Vec<String> {
        input_formats_if_has_component::<OpenState>(
            entity,
            world,
            &[CLOSE_FORMAT.get_format_description().with_targeted_entity(
                TARGET_PART_ID.clone(),
                entity,
                world,
            )],
        )
    }
}

/// Makes an entity open or close something.
#[derive(Debug)]
pub struct OpenAction {
    pub target: Entity,
    pub should_be_open: bool,
    pub notification_sender: ActionNotificationSender<Self>,
}

impl Action for OpenAction {
    fn perform(&mut self, performing_entity: Entity, world: &mut World) -> ActionResult {
        let state = match world.get::<OpenState>(self.target) {
            Some(s) => s,
            None => {
                if self.should_be_open {
                    return ActionResult::error(
                        performing_entity,
                        "You can't open that.".to_string(),
                    );
                } else {
                    return ActionResult::error(
                        performing_entity,
                        "You can't close that.".to_string(),
                    );
                }
            }
        };

        if state.is_open == self.should_be_open {
            if state.is_open {
                return ActionResult::message(
                    performing_entity,
                    "It's already open.".to_string(),
                    MessageCategory::Internal(InternalMessageCategory::Misc),
                    MessageDelay::Short,
                    false,
                );
            } else {
                return ActionResult::message(
                    performing_entity,
                    "It's already closed.".to_string(),
                    MessageCategory::Internal(InternalMessageCategory::Misc),
                    MessageDelay::Short,
                    false,
                );
            }
        }

        OpenState::set_open(self.target, self.should_be_open, world);

        let target_name =
            Description::get_reference_name(self.target, Some(performing_entity), world);
        let (open_or_close, opens_or_closes) = if self.should_be_open {
            ("open", "opens")
        } else {
            ("close", "closes")
        };

        ActionResult::builder()
            .with_message(
                performing_entity,
                format!("You {open_or_close} {target_name}."),
                MessageCategory::Internal(InternalMessageCategory::Action),
                MessageDelay::Short,
            )
            .with_dynamic_message(
                Some(performing_entity),
                DynamicMessageLocation::SourceEntity,
                DynamicMessage::new_third_person(
                    MessageCategory::Surroundings(SurroundingsMessageCategory::Action),
                    MessageDelay::Short,
                    MessageFormat::new(
                        "${performing_entity.Name} ${opens_or_closes} ${target.name}.",
                    )
                    .expect("message format should be valid"),
                    BasicTokens::new()
                        .with_entity("performing_entity".into(), performing_entity)
                        .with_string("opens_or_closes".into(), opens_or_closes.to_string())
                        .with_entity("target".into(), self.target),
                ),
                world,
            )
            .build_complete_should_tick(true)
    }

    fn interrupt(&self, performing_entity: Entity, _: &mut World) -> ActionInterruptResult {
        ActionInterruptResult::message(
            performing_entity,
            "You stop opening.".to_string(),
            MessageCategory::Internal(InternalMessageCategory::Action),
            MessageDelay::None,
        )
    }

    fn may_require_tick(&self) -> bool {
        true
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
