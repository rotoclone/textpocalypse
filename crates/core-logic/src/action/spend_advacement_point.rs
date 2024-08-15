use std::{collections::HashSet, sync::LazyLock};

use bevy_ecs::prelude::*;
use regex::Regex;

use crate::{
    component::{ActionEndNotification, AfterActionPerformNotification},
    input_parser::{CommandParseError, InputParseError, InputParser},
    notification::VerifyResult,
    resource::{AttributeNameCatalog, SkillNameCatalog},
    ActionTag, Attribute, BeforeActionNotification, MessageCategory, MessageDelay, Skill, Stats,
    VerifyActionNotification, World,
};

use super::{Action, ActionInterruptResult, ActionNotificationSender, ActionResult};

const SPEND_ADVANCEMENT_POINT_VERB_NAME: &str = "spend";
const SPEND_SKILL_POINT_FORMAT: &str = "spend skill point on <>";
const SPEND_ATTRIBUTE_POINT_FORMAT: &str = "spend attribute point on <>";
const TARGET_CAPTURE: &str = "target";

static SPEND_SKILL_POINT_PATTERN: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new("^(spend skill point on|assign skill point to) (?P<target>.*)").unwrap()
});
static SPEND_ATTRIBUTE_POINT_PATTERN: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new("^(spend attribute point on|assign attribute point to) (?P<target>.*)").unwrap()
});

pub struct SpendAdvancementPointParser;

impl InputParser for SpendAdvancementPointParser {
    fn parse(
        &self,
        input: &str,
        _: Entity,
        world: &World,
    ) -> Result<Box<dyn Action>, InputParseError> {
        if let Some(captures) = SPEND_SKILL_POINT_PATTERN.captures(input) {
            // attempting to spend a skill point
            if let Some(skill_match) = captures.name(TARGET_CAPTURE) {
                // the name of a skill was provided
                let skill_name = skill_match.as_str();
                if let Some(skill) = SkillNameCatalog::get_skill(skill_name, world) {
                    // it's actually a skill that exists
                    return Ok(Box::new(SpendSkillPointAction {
                        skill,
                        notification_sender: ActionNotificationSender::new(),
                    }));
                } else {
                    // invalid skill name
                    return Err(InputParseError::CommandParseError {
                        verb: SPEND_ADVANCEMENT_POINT_VERB_NAME.to_string(),
                        error: CommandParseError::Other(format!("{skill_name} is not a skill.")),
                    });
                }
            } else {
                // no skill name provided
                return Err(InputParseError::CommandParseError {
                    verb: SPEND_ADVANCEMENT_POINT_VERB_NAME.to_string(),
                    error: CommandParseError::MissingTarget,
                });
            }
        } else if let Some(captures) = SPEND_ATTRIBUTE_POINT_PATTERN.captures(input) {
            // attempting to spend an attribute point
            if let Some(attribute_match) = captures.name(TARGET_CAPTURE) {
                // the name of an attribute was provided
                let attribute_name = attribute_match.as_str();
                if let Some(attribute) = AttributeNameCatalog::get_attribute(attribute_name, world)
                {
                    // it's actually an attribute that exists
                    return Ok(Box::new(SpendAttributePointAction {
                        attribute,
                        notification_sender: ActionNotificationSender::new(),
                    }));
                } else {
                    // invalid attribute name
                    return Err(InputParseError::CommandParseError {
                        verb: SPEND_ADVANCEMENT_POINT_VERB_NAME.to_string(),
                        error: CommandParseError::Other(format!(
                            "{attribute_name} is not an attribute."
                        )),
                    });
                }
            } else {
                // no attribute name provided
                return Err(InputParseError::CommandParseError {
                    verb: SPEND_ADVANCEMENT_POINT_VERB_NAME.to_string(),
                    error: CommandParseError::MissingTarget,
                });
            }
        }

        Err(InputParseError::UnknownCommand)
    }

    fn get_input_formats(&self) -> Vec<String> {
        vec![
            SPEND_SKILL_POINT_FORMAT.to_string(),
            SPEND_ATTRIBUTE_POINT_FORMAT.to_string(),
        ]
    }

    fn get_input_formats_for(
        &self,
        entity: Entity,
        _: Entity,
        world: &World,
    ) -> Option<Vec<String>> {
        let mut formats = None;

        if let Some(stats) = world.get::<Stats>(entity) {
            if stats.advancement.skill_points.available > 0 {
                formats
                    .get_or_insert_with(Vec::new)
                    .push(SPEND_SKILL_POINT_FORMAT.to_string());
            }

            if stats.advancement.attribute_points.available > 0 {
                formats
                    .get_or_insert_with(Vec::new)
                    .push(SPEND_ATTRIBUTE_POINT_FORMAT.to_string());
            }
        }

        formats
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
