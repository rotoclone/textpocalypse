use bevy_ecs::prelude::*;
use lazy_static::lazy_static;
use regex::Regex;

use crate::{
    component::{
        ActionEndNotification, AfterActionPerformNotification, CombatState, Vitals, Weapon,
    },
    get_reference_name,
    input_parser::{
        input_formats_if_has_component, CommandParseError, CommandTarget, InputParseError,
        InputParser,
    },
    notification::VerifyResult,
    value_change::{ValueChange, ValueChangeOperation},
    BeforeActionNotification, InternalMessageCategory, MessageCategory, MessageDelay,
    SurroundingsMessageCategory, ValueType, VerifyActionNotification,
};

use super::{
    Action, ActionInterruptResult, ActionNotificationSender, ActionResult, ThirdPersonMessage,
    ThirdPersonMessageLocation,
};

const ATTACK_VERB_NAME: &str = "attack";
const ATTACK_FORMAT: &str = "attack <>";
const NAME_CAPTURE: &str = "name";

lazy_static! {
    static ref ATTACK_PATTERN: Regex = Regex::new("^(attack|kill|k) (?P<name>.*)").unwrap();
}

pub struct AttackParser;

impl InputParser for AttackParser {
    fn parse(
        &self,
        input: &str,
        source_entity: Entity,
        world: &World,
    ) -> Result<Box<dyn Action>, InputParseError> {
        if let Some(captures) = ATTACK_PATTERN.captures(input) {
            if let Some(target_match) = captures.name(NAME_CAPTURE) {
                let target = CommandTarget::parse(target_match.as_str());
                if let Some(target_entity) = target.find_target_entity(source_entity, world) {
                    if world.get::<Vitals>(target_entity).is_some() {
                        // target exists and is attackable
                        return Ok(Box::new(AttackAction {
                            target: target_entity,
                            notification_sender: ActionNotificationSender::new(),
                        }));
                    } else {
                        // target isn't attackable
                        let target_name =
                            get_reference_name(target_entity, Some(source_entity), world);
                        return Err(InputParseError::CommandParseError {
                            verb: ATTACK_VERB_NAME.to_string(),
                            error: CommandParseError::Other(format!(
                                "You can't attack {target_name}."
                            )),
                        });
                    }
                } else {
                    // target doesn't exist
                    return Err(InputParseError::CommandParseError {
                        verb: ATTACK_VERB_NAME.to_string(),
                        error: CommandParseError::TargetNotFound(target),
                    });
                }
            }
        }

        Err(InputParseError::UnknownCommand)
    }

    fn get_input_formats(&self) -> Vec<String> {
        vec![ATTACK_FORMAT.to_string()]
    }

    fn get_input_formats_for(&self, entity: Entity, world: &World) -> Option<Vec<String>> {
        input_formats_if_has_component::<Vitals>(entity, world, &[ATTACK_FORMAT])
    }
}

/// Makes an entity attack another entity.
#[derive(Debug)]
pub struct AttackAction {
    pub target: Entity,
    pub notification_sender: ActionNotificationSender<Self>,
}

impl Action for AttackAction {
    fn perform(&mut self, performing_entity: Entity, world: &mut World) -> ActionResult {
        let target = self.target;
        let target_name = get_reference_name(target, Some(performing_entity), world);
        let (weapon, weapon_name) = Weapon::get_primary(performing_entity, world)
            .expect("attacking entity should have a weapon");
        let weapon_name = weapon_name.clone();

        if target == performing_entity {
            let damage = weapon.calculate_damage(*weapon.optimal_ranges.start(), true);
            let third_person_hit_verb = weapon.hit_verb.third_person_singular.clone();

            ValueChange {
                entity: performing_entity,
                value_type: ValueType::Health,
                operation: ValueChangeOperation::Subtract,
                amount: damage as f32,
                message: Some(format!(
                    "You {} yourself with your {}!",
                    weapon.hit_verb.second_person, weapon_name
                )),
            }
            .apply(world);

            return ActionResult::builder()
                .with_third_person_message(
                    Some(performing_entity),
                    ThirdPersonMessageLocation::SourceEntity,
                    ThirdPersonMessage::new(
                        MessageCategory::Surroundings(SurroundingsMessageCategory::Action),
                        MessageDelay::Short,
                    )
                    .add_entity_name(performing_entity)
                    .add_string(format!(" {third_person_hit_verb} "))
                    .add_entity_reflexive_pronoun(performing_entity)
                    .add_string(" with ")
                    .add_entity_possessive_adjective_pronoun(performing_entity)
                    .add_string(format!(" {weapon_name}.")),
                    world,
                )
                .build_complete_should_tick(true);
        }

        CombatState::enter_combat(performing_entity, target, world);

        let mut result_builder = ActionResult::builder()
            .with_message(
                performing_entity,
                format!("You attack {target_name}!"),
                MessageCategory::Internal(InternalMessageCategory::Action),
                MessageDelay::Short,
            )
            .with_third_person_message(
                Some(performing_entity),
                ThirdPersonMessageLocation::SourceEntity,
                ThirdPersonMessage::new(
                    MessageCategory::Surroundings(SurroundingsMessageCategory::Action),
                    MessageDelay::Short,
                )
                .only_send_to(target)
                .add_entity_name(performing_entity)
                .add_string(" attacks you!"),
                world,
            )
            .with_third_person_message(
                Some(performing_entity),
                ThirdPersonMessageLocation::SourceEntity,
                ThirdPersonMessage::new(
                    MessageCategory::Surroundings(SurroundingsMessageCategory::Action),
                    MessageDelay::Short,
                )
                .do_not_send_to(target)
                .add_entity_name(performing_entity)
                .add_string(" attacks ")
                .add_entity_name(target)
                .add_string("!".to_string()),
                world,
            );

        //TODO try to hit the target

        result_builder.build_complete_should_tick(true)
    }

    fn interrupt(&self, performing_entity: Entity, _: &mut World) -> ActionInterruptResult {
        ActionInterruptResult::message(
            performing_entity,
            "You stop attacking.".to_string(),
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

// TODO verify the target is still in the same room as the performing entity and still alive

// TODO verify the attacker has some kind of weapon

// TODO before attacking, have the attacker equip their primary weapon if they don't have a weapon equipped
