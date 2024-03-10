use bevy_ecs::prelude::*;
use lazy_static::lazy_static;
use regex::Regex;

use crate::{
    apply_body_part_damage_multiplier, check_for_hit, handle_begin_attack, handle_damage,
    handle_miss, handle_weapon_unusable_error, input_parser::InputParser,
    resource::WeaponTypeStatCatalog, Action, ActionEndNotification, ActionInterruptResult,
    ActionNotificationSender, ActionResult, AfterActionPerformNotification,
    BeforeActionNotification, BodyPart, CheckModifiers, CheckResult, CommandParseError,
    CommandTarget, Description, HitParams, InputParseError, IntegerExtensions,
    InternalMessageCategory, MessageCategory, MessageDelay, ParseCustomInput, Skill, Stats,
    VerifyActionNotification, VerifyResult, Vitals, VsCheckParams, VsParticipant, Weapon,
};

/// A component that provides special attack actions for fists.
#[derive(Component)]
pub struct FistActions;

impl ParseCustomInput for FistActions {
    fn get_parsers() -> Vec<Box<dyn InputParser>> {
        vec![Box::new(UppercutParser)]
    }
}

const UPPERCUT_VERB_NAME: &str = "uppercut";
const UPPERCUT_FORMAT: &str = "uppercut <>";
const NAME_CAPTURE: &str = "name";

/// The amount to modify the to hit bonus by for uppercuts.
const UPPERCUT_TO_HIT_MODIFIER: i16 = -2;

/// The multiplier for damage done by uppercuts.
const UPPERCUT_DAMAGE_MULTIPLIER: f32 = 1.2;

lazy_static! {
    static ref UPPERCUT_PATTERN: Regex = Regex::new("^(uppercut) (?P<name>.*)").unwrap();
}

struct UppercutParser;

impl InputParser for UppercutParser {
    fn parse(
        &self,
        input: &str,
        source_entity: Entity,
        world: &World,
    ) -> Result<Box<dyn Action>, InputParseError> {
        // TODO move this to a common place so it doesn't have to be repeated in every attack variation
        if let Some(captures) = UPPERCUT_PATTERN.captures(input) {
            if let Some(target_match) = captures.name(NAME_CAPTURE) {
                let target = CommandTarget::parse(target_match.as_str());
                if let Some(target_entity) = target.find_target_entity(source_entity, world) {
                    if world.get::<Vitals>(target_entity).is_some() {
                        // target exists and is attackable
                        return Ok(Box::new(UppercutAction {
                            target: target_entity,
                            notification_sender: ActionNotificationSender::new(),
                        }));
                    }
                    let target_name =
                        Description::get_reference_name(target_entity, Some(source_entity), world);
                    return Err(InputParseError::CommandParseError {
                        verb: UPPERCUT_VERB_NAME.to_string(),
                        error: CommandParseError::Other(format!("You can't attack {target_name}.")),
                    });
                }
                return Err(InputParseError::CommandParseError {
                    verb: UPPERCUT_VERB_NAME.to_string(),
                    error: CommandParseError::TargetNotFound(target),
                });
            } else {
                //TODO auto-target if entity is in combat with 1 other entity
            }
        }

        Err(InputParseError::UnknownCommand)
    }

    fn get_input_formats(&self) -> Vec<String> {
        vec![UPPERCUT_FORMAT.to_string()]
    }

    fn get_input_formats_for(&self, _: Entity, _: Entity, _: &World) -> Option<Vec<String>> {
        None
    }
}

#[derive(Debug)]
pub struct UppercutAction {
    target: Entity,
    notification_sender: ActionNotificationSender<Self>,
}

impl Action for UppercutAction {
    fn perform(&mut self, performing_entity: Entity, world: &mut World) -> ActionResult {
        let target = self.target;
        let result_builder = ActionResult::builder();

        let (mut result_builder, range) =
            handle_begin_attack(performing_entity, target, result_builder, world);

        let (weapon, weapon_entity) = Weapon::get_primary(performing_entity, world)
            .expect("attacking entity should have a weapon");

        let to_hit_modification =
            match weapon.calculate_to_hit_modification(performing_entity, range, world) {
                Ok(x) => x + UPPERCUT_TO_HIT_MODIFIER,
                Err(e) => {
                    return handle_weapon_unusable_error(
                        performing_entity,
                        target,
                        weapon_entity,
                        e,
                        result_builder,
                        world,
                    )
                }
            };

        let hit_params = match check_for_hit(
            performing_entity,
            target,
            weapon_entity,
            range,
            to_hit_modification as f32,
            world,
        ) {
            Ok(x) => x,
            Err(e) => {
                return handle_weapon_unusable_error(
                    performing_entity,
                    target,
                    weapon_entity,
                    e,
                    result_builder,
                    world,
                )
            }
        };

        if let Some(mut hit_params) = hit_params {
            hit_params.damage = hit_params.damage.mul_and_round(UPPERCUT_DAMAGE_MULTIPLIER);
            result_builder = handle_damage(hit_params, result_builder, world);
        } else {
            result_builder = handle_miss(
                performing_entity,
                target,
                weapon_entity,
                result_builder,
                world,
            );
        }

        result_builder.build_complete_should_tick(true)
    }

    fn interrupt(&self, performing_entity: Entity, _: &mut World) -> ActionInterruptResult {
        ActionInterruptResult::message(
            performing_entity,
            "You stop uppercutting.".to_string(),
            MessageCategory::Internal(InternalMessageCategory::Action),
            MessageDelay::None,
        )
    }

    fn may_require_tick(&self) -> bool {
        true
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
