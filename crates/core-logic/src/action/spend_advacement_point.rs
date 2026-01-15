use std::{collections::HashSet, sync::LazyLock};

use bevy_ecs::prelude::*;
use nonempty::nonempty;

use crate::{
    command_format::{
        any_text_part_with_validator, literal_part, one_of_literal_part, CommandFormat,
        CommandPartId, CommandPartValidateError, CommandPartValidateResult, PartValidatorContext,
    },
    component::{ActionEndNotification, AfterActionPerformNotification},
    input_parser::{InputParseError, InputParser},
    notification::VerifyResult,
    resource::{AttributeNameCatalog, SkillNameCatalog},
    ActionTag, Attribute, BeforeActionNotification, MessageCategory, MessageDelay, Skill, Stats,
    VerifyActionNotification, World,
};

use super::{Action, ActionInterruptResult, ActionNotificationSender, ActionResult};

static ADVANCEMENT_TYPE_PART_ID: LazyLock<CommandPartId<String>> =
    LazyLock::new(|| CommandPartId::new("advancement_type"));

static SPEND_SKILL_POINT_FORMAT: LazyLock<CommandFormat> = LazyLock::new(|| {
    CommandFormat::new(one_of_literal_part(nonempty![
        "spend skill point on",
        "assign skill point to",
        "increase skill"
    ]))
    .then(literal_part(" ").always_include_in_errors())
    .then(
        any_text_part_with_validator(ADVANCEMENT_TYPE_PART_ID.clone(), validate_skill_name)
            .always_include_in_errors()
            .with_if_unparsed("which skill")
            .with_placeholder_for_format_string("skill"),
    )
});
static SPEND_ATTRIBUTE_POINT_FORMAT: LazyLock<CommandFormat> = LazyLock::new(|| {
    CommandFormat::new(one_of_literal_part(nonempty![
        "spend attribute point on",
        "assign attribute point to",
        "increase attribute"
    ]))
    .then(literal_part(" ").always_include_in_errors())
    .then(
        any_text_part_with_validator(ADVANCEMENT_TYPE_PART_ID.clone(), validate_attribute_name)
            .always_include_in_errors()
            .with_if_unparsed("which attribute")
            .with_placeholder_for_format_string("attribute"),
    )
});

/// Validates that the parsed value is the name of a skill, ignoring case.
fn validate_skill_name(
    context: &PartValidatorContext<String>,
    world: &World,
) -> CommandPartValidateResult {
    if SkillNameCatalog::get_skill(&context.parsed_value, world).is_some() {
        return CommandPartValidateResult::Valid;
    }

    CommandPartValidateResult::Invalid(CommandPartValidateError {
        details: Some(format!("'{}' is not a skill.", context.parsed_value)),
    })
}

/// Validates that the parsed value is the name of an attribute, ignoring case.
fn validate_attribute_name(
    context: &PartValidatorContext<String>,
    world: &World,
) -> CommandPartValidateResult {
    if AttributeNameCatalog::get_attribute(&context.parsed_value, world).is_some() {
        return CommandPartValidateResult::Valid;
    }

    CommandPartValidateResult::Invalid(CommandPartValidateError {
        details: Some(format!("'{}' is not an attribute.", context.parsed_value)),
    })
}

pub struct SpendSkillPointParser;
pub struct SpendAttributePointParser;

impl InputParser for SpendSkillPointParser {
    fn parse(
        &self,
        input: &str,
        source_entity: Entity,
        world: &World,
    ) -> Result<Box<dyn Action>, InputParseError> {
        let parsed = SPEND_SKILL_POINT_FORMAT.parse(input, source_entity, world)?;
        let skill_name = parsed.get(&ADVANCEMENT_TYPE_PART_ID);
        if let Some(skill) = SkillNameCatalog::get_skill(&skill_name, world) {
            Ok(Box::new(SpendSkillPointAction {
                skill,
                notification_sender: ActionNotificationSender::new(),
            }))
        } else {
            // this should never happen due to the validator, but ya never know
            Err(InputParseError::PostFormatParse(format!(
                "'{skill_name}' is not a skill."
            )))
        }
    }

    fn get_input_formats(&self) -> Vec<String> {
        vec![SPEND_SKILL_POINT_FORMAT
            .get_format_description()
            .to_string()]
    }

    fn get_input_formats_for(&self, _: Entity, _: Entity, _: &World) -> Vec<String> {
        Vec::new()
    }
}

impl InputParser for SpendAttributePointParser {
    fn parse(
        &self,
        input: &str,
        source_entity: Entity,
        world: &World,
    ) -> Result<Box<dyn Action>, InputParseError> {
        let parsed = SPEND_ATTRIBUTE_POINT_FORMAT.parse(input, source_entity, world)?;
        let attribute_name = parsed.get(&ADVANCEMENT_TYPE_PART_ID);
        if let Some(attribute) = AttributeNameCatalog::get_attribute(&attribute_name, world) {
            Ok(Box::new(SpendAttributePointAction {
                attribute,
                notification_sender: ActionNotificationSender::new(),
            }))
        } else {
            // this should never happen due to the validator, but ya never know
            Err(InputParseError::PostFormatParse(format!(
                "'{attribute_name}' is not an attribute."
            )))
        }
    }

    fn get_input_formats(&self) -> Vec<String> {
        vec![SPEND_ATTRIBUTE_POINT_FORMAT
            .get_format_description()
            .to_string()]
    }

    fn get_input_formats_for(&self, _: Entity, _: Entity, _: &World) -> Vec<String> {
        Vec::new()
    }
}

/// Spends a skill point to increase a skill.
#[derive(Debug)]
pub struct SpendSkillPointAction {
    pub skill: Skill,
    pub notification_sender: ActionNotificationSender<Self>,
}

impl Action for SpendSkillPointAction {
    fn perform(&mut self, performing_entity: Entity, world: &mut World) -> ActionResult {
        if let Some(mut stats) = world.get_mut::<Stats>(performing_entity) {
            if stats.advancement.skill_points.available > 0 {
                stats.advancement.skill_points.available -= 1;
            } else {
                return ActionResult::error(
                    performing_entity,
                    "You don't have any skill points to spend.".to_string(),
                );
            }

            let new_value = stats.skills.get_base(&self.skill) + 1;
            stats.set_skill(&self.skill, new_value);

            let skill_name = SkillNameCatalog::get_name(&self.skill, world);
            return ActionResult::message(
                performing_entity,
                format!("Your base {skill_name} is now {new_value}."),
                MessageCategory::System,
                MessageDelay::None,
                false,
            );
        }

        ActionResult::error(
            performing_entity,
            "You don't have any stats to mess with.".to_string(),
        )
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

/// Spends an attribute point to increase an attribute.
#[derive(Debug)]
pub struct SpendAttributePointAction {
    pub attribute: Attribute,
    pub notification_sender: ActionNotificationSender<Self>,
}

impl Action for SpendAttributePointAction {
    fn perform(&mut self, performing_entity: Entity, world: &mut World) -> ActionResult {
        if let Some(mut stats) = world.get_mut::<Stats>(performing_entity) {
            if stats.advancement.attribute_points.available > 0 {
                stats.advancement.attribute_points.available -= 1;
            } else {
                return ActionResult::error(
                    performing_entity,
                    "You don't have any attribute points to spend.".to_string(),
                );
            }

            let new_value = stats.attributes.get(&self.attribute) + 1;
            stats.set_attribute(&self.attribute, new_value);

            let attribute_name = AttributeNameCatalog::get_name(&self.attribute, world).full;
            return ActionResult::message(
                performing_entity,
                format!("Your {attribute_name} is now {new_value}."),
                MessageCategory::System,
                MessageDelay::None,
                false,
            );
        }

        ActionResult::error(
            performing_entity,
            "You don't have any stats to mess with.".to_string(),
        )
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
