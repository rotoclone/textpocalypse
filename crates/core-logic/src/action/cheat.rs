use std::{collections::HashSet, sync::LazyLock};

use bevy_ecs::prelude::*;
use regex::Regex;

use crate::{
    component::{ActionEndNotification, AfterActionPerformNotification},
    input_parser::{CommandParseError, InputParseError, InputParser},
    notification::VerifyResult,
    resource::{AttributeNameCatalog, SkillNameCatalog},
    vital_change::{ValueChangeOperation, VitalChangeMessageParams, VitalChangeVisualizationType},
    ActionTag, BasicTokens, BeforeActionNotification, CommandTarget, Description, MessageCategory,
    MessageDelay, MessageFormat, NoTokens, Notification, Stat, Stats, VerifyActionNotification,
    VitalChange, VitalType, World, Xp, XpAwardNotification,
};

use super::{Action, ActionInterruptResult, ActionNotificationSender, ActionResult};

const CHEAT_VERB_NAME: &str = "cheat";
const CHEAT_FORMAT: &str = "%<>% <>";
const COMMAND_CAPTURE: &str = "command";
const ARGS_CAPTURE: &str = "args";

static CHEAT_PATTERN: LazyLock<Regex> =
    LazyLock::new(|| Regex::new("^%(?P<command>.*)%( (?P<args>.*))?").unwrap());

pub struct CheatParser;

impl InputParser for CheatParser {
    fn parse(&self, input: &str, _: Entity, _: &World) -> Result<Box<dyn Action>, InputParseError> {
        if let Some(captures) = CHEAT_PATTERN.captures(input) {
            if let Some(command_match) = captures.name(COMMAND_CAPTURE) {
                return Ok(Box::new(CheatAction {
                    command: command_match.as_str().to_string(),
                    args: captures
                        .name(ARGS_CAPTURE)
                        .map(|args_match| {
                            args_match
                                .as_str()
                                .split(",")
                                .map(|s| s.to_string())
                                .collect::<Vec<String>>()
                        })
                        .unwrap_or_default(),
                    notification_sender: ActionNotificationSender::new(),
                }));
            } else {
                return Err(InputParseError::CommandParseError {
                    verb: CHEAT_VERB_NAME.to_string(),
                    error: CommandParseError::MissingTarget,
                });
            }
        }

        Err(InputParseError::UnknownCommand)
    }

    fn get_input_formats(&self) -> Vec<String> {
        vec![CHEAT_FORMAT.to_string()]
    }

    fn get_input_formats_for(&self, _: Entity, _: Entity, _: &World) -> Option<Vec<String>> {
        None
    }
}

/// Lets an entity do something they're not allowed to do.
#[derive(Debug)]
pub struct CheatAction {
    pub command: String,
    pub args: Vec<String>,
    pub notification_sender: ActionNotificationSender<Self>,
}

impl Action for CheatAction {
    fn perform(&mut self, performing_entity: Entity, world: &mut World) -> ActionResult {
        match self.command.as_str() {
            "give_xp" => give_xp(performing_entity, &self.args, world),
            "set_hp" => set_vital(
                performing_entity,
                &self.args,
                VitalType::Health,
                "set_hp",
                world,
            ),
            "set_satiety" => set_vital(
                performing_entity,
                &self.args,
                VitalType::Satiety,
                "set_satiety",
                world,
            ),
            "set_hydration" => set_vital(
                performing_entity,
                &self.args,
                VitalType::Hydration,
                "set_hydration",
                world,
            ),
            "set_energy" => set_vital(
                performing_entity,
                &self.args,
                VitalType::Energy,
                "set_energy",
                world,
            ),
            "set_stat" => set_stat(performing_entity, &self.args, world),
            x => ActionResult::error(performing_entity, format!("Unknown cheat command: {x}")),
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

fn give_xp(entity: Entity, args: &[String], world: &mut World) -> ActionResult {
    let target;
    let xp;
    if args.len() == 1 {
        target = entity;
        xp = &args[0];
    } else if args.len() == 2 {
        let target_name = &args[0];
        if let Some(t) = CommandTarget::parse(target_name).find_target_entity(entity, world) {
            target = t;
        } else {
            return ActionResult::error(entity, format!("Invalid target name: {target_name}",));
        }
        xp = &args[1];
    } else {
        return ActionResult::error(
            entity,
            "give_xp requires 1 number, or 1 target name and 1 number".to_string(),
        );
    };

    match xp.parse() {
        Ok(amount) => {
            Notification::send_no_contents(
                XpAwardNotification {
                    entity: target,
                    xp_to_add: Xp(amount),
                },
                world,
            );

            let target_name = Description::get_reference_name(target, Some(entity), world);

            ActionResult::message(
                entity,
                format!("Awarded {target_name} {amount} XP."),
                MessageCategory::System,
                MessageDelay::None,
                false,
            )
        }
        Err(e) => ActionResult::error(entity, format!("Error: {e}")),
    }
}

fn set_vital(
    entity: Entity,
    args: &[String],
    vital_type: VitalType,
    command_name: &str,
    world: &mut World,
) -> ActionResult {
    let target;
    let new_amount;
    if args.len() == 1 {
        target = entity;
        new_amount = &args[0];
    } else if args.len() == 2 {
        let target_name = &args[0];
        if let Some(t) = CommandTarget::parse(target_name).find_target_entity(entity, world) {
            target = t;
        } else {
            return ActionResult::error(entity, format!("Invalid target name: {target_name}",));
        }
        new_amount = &args[1];
    } else {
        return ActionResult::error(
            entity,
            format!("{command_name} requires 1 number, or 1 target name and 1 number"),
        );
    };

    match new_amount.parse() {
        Ok(amount) => {
            let mut message_params = vec![(
                VitalChangeMessageParams::Direct {
                    entity,
                    message: "Zorp, magic".to_string(),
                    category: MessageCategory::System,
                },
                VitalChangeVisualizationType::Full,
            )];

            if entity != target {
                message_params.push((
                    VitalChangeMessageParams::Direct {
                        entity: target,
                        message: "Zorp, magic".to_string(),
                        category: MessageCategory::System,
                    },
                    VitalChangeVisualizationType::Full,
                ));
            }
            VitalChange::<NoTokens> {
                entity: target,
                vital_type,
                operation: ValueChangeOperation::Set,
                amount,
                message_params,
            }
            .apply(world);

            let message = MessageFormat::new("Set ${target.name's} ${vital} to ${amount}.")
                .expect("message format should be valid")
                .interpolate(
                    entity,
                    &BasicTokens::new()
                        .with_entity("target".into(), target)
                        .with_string("vital".into(), vital_type.to_string())
                        .with_string("amount".into(), amount.to_string()),
                    world,
                )
                .expect("message should interpolate correctly");

            ActionResult::message(
                entity,
                message,
                MessageCategory::System,
                MessageDelay::None,
                false,
            )
        }
        Err(e) => ActionResult::error(entity, format!("Error: {e}")),
    }
}

fn set_stat(entity: Entity, args: &[String], world: &mut World) -> ActionResult {
    let target;
    let stat_name;
    let new_base_value;
    if args.len() == 2 {
        target = entity;
        stat_name = &args[0];
        new_base_value = &args[1];
    } else if args.len() == 3 {
        let target_name = &args[0];
        if let Some(t) = CommandTarget::parse(target_name).find_target_entity(entity, world) {
            target = t;
        } else {
            return ActionResult::error(entity, format!("Invalid target name: {target_name}",));
        }
        stat_name = &args[1];
        new_base_value = &args[2];
    } else {
        return ActionResult::error(
            entity,
            "set_stat requires 1 stat name and 1 number, or 1 target name and 1 stat name and 1 number".to_string(),
        );
    }

    let stat;
    if let Some(attribute) = AttributeNameCatalog::get_attribute(stat_name, world) {
        stat = Stat::Attribute(attribute);
    } else if let Some(skill) = SkillNameCatalog::get_skill(stat_name, world) {
        stat = Stat::Skill(skill);
    } else {
        return ActionResult::error(entity, format!("Invalid stat name: {stat_name}"));
    }

    match new_base_value.parse() {
        Ok(base_value) => {
            if let Some(mut stats) = world.get_mut::<Stats>(target) {
                match &stat {
                    Stat::Attribute(a) => stats.set_attribute(a, base_value),
                    Stat::Skill(s) => stats.set_skill(s, base_value),
                }

                let message =
                    MessageFormat::new("Set ${target.name's} base ${stat} to ${base_value}.")
                        .expect("message format should be valid")
                        .interpolate(
                            entity,
                            &BasicTokens::new()
                                .with_entity("target".into(), target)
                                .with_string("stat".into(), stat.to_string())
                                .with_string("base_value".into(), base_value.to_string()),
                            world,
                        )
                        .expect("message should interpolate correctly");

                ActionResult::message(
                    entity,
                    message,
                    MessageCategory::System,
                    MessageDelay::None,
                    false,
                )
            } else {
                let message = MessageFormat::new("${target.Name} ${target.you:have/has} no stats.")
                    .expect("message format should be valid")
                    .interpolate(
                        entity,
                        &BasicTokens::new().with_entity("target".into(), target),
                        world,
                    )
                    .expect("message should interpolate correctly");
                ActionResult::error(entity, message)
            }
        }
        Err(e) => ActionResult::error(entity, format!("Error: {e}")),
    }
}
