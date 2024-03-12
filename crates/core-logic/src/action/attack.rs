use bevy_ecs::prelude::*;
use lazy_static::lazy_static;
use regex::Regex;

use crate::{
    check_for_hit,
    component::{ActionEndNotification, AfterActionPerformNotification, Location, Vitals, Weapon},
    handle_begin_attack, handle_damage, handle_miss, handle_weapon_unusable_error,
    input_parser::{input_formats_if_has_component, InputParseError, InputParser},
    is_living_entity,
    notification::{Notification, VerifyResult},
    parse_attack_input,
    vital_change::{ValueChangeOperation, VitalChange, VitalType},
    BeforeActionNotification, Description, GameMessage, InternalMessageCategory, MessageCategory,
    MessageDelay, SurroundingsMessageCategory, VerifyActionNotification,
};

use super::{
    Action, ActionInterruptResult, ActionNotificationSender, ActionResult, ThirdPersonMessage,
    ThirdPersonMessageLocation,
};

/// Multiplier applied to damage done to oneself.
const SELF_DAMAGE_MULT: f32 = 3.0;

const ATTACK_VERB_NAME: &str = "attack";
const ATTACK_FORMAT: &str = "attack <>";
const NAME_CAPTURE: &str = "name";

lazy_static! {
    static ref ATTACK_PATTERN: Regex = Regex::new("^(attack|kill|k)( (?P<name>.*))?").unwrap();
}

pub struct AttackParser;

impl InputParser for AttackParser {
    fn parse(
        &self,
        input: &str,
        source_entity: Entity,
        world: &World,
    ) -> Result<Box<dyn Action>, InputParseError> {
        let target_entity = parse_attack_input(
            input,
            source_entity,
            &ATTACK_PATTERN,
            NAME_CAPTURE,
            ATTACK_VERB_NAME,
            world,
        )?;

        Ok(Box::new(AttackAction {
            target: target_entity,
            notification_sender: ActionNotificationSender::new(),
        }))
    }

    fn get_input_formats(&self) -> Vec<String> {
        vec![ATTACK_FORMAT.to_string()]
    }

    fn get_input_formats_for(
        &self,
        entity: Entity,
        _: Entity,
        world: &World,
    ) -> Option<Vec<String>> {
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
        let result_builder = ActionResult::builder();

        if target == performing_entity {
            let (weapon, weapon_entity) = Weapon::get_primary(performing_entity, world)
                .expect("attacking entity should have a weapon");
            let weapon_hit_verb = weapon.hit_verb.clone();
            let weapon_name =
                Description::get_reference_name(weapon_entity, Some(performing_entity), world);

            match weapon.calculate_damage(
                performing_entity,
                *weapon.ranges.optimal.start(),
                true,
                world,
            ) {
                Ok(damage) => {
                    let third_person_hit_verb = weapon_hit_verb.third_person_singular;

                    VitalChange {
                        entity: performing_entity,
                        vital_type: VitalType::Health,
                        operation: ValueChangeOperation::Subtract,
                        amount: damage as f32 * SELF_DAMAGE_MULT,
                        message: Some(format!(
                            "You {} yourself with {}!",
                            weapon_hit_verb.second_person, weapon_name
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
                            .add_name(performing_entity)
                            .add_string(format!(" {third_person_hit_verb} "))
                            .add_reflexive_pronoun(performing_entity)
                            .add_string(" with ")
                            .add_name(weapon_entity)
                            .add_string("."),
                            world,
                        )
                        .build_complete_should_tick(true);
                }
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
            }
        }

        let (mut result_builder, range) =
            handle_begin_attack(performing_entity, target, result_builder, world);

        let (weapon, weapon_entity) = Weapon::get_primary(performing_entity, world)
            .expect("attacking entity should have a weapon");

        let to_hit_modification =
            match weapon.calculate_to_hit_modification(performing_entity, range, world) {
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

        if let Some(hit_params) = hit_params {
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

/// Verifies that the target is in the same room as the attacker.
pub fn verify_target_in_same_room(
    notification: &Notification<VerifyActionNotification, AttackAction>,
    world: &World,
) -> VerifyResult {
    let performing_entity = notification.notification_type.performing_entity;
    let target = notification.contents.target;
    let target_name = Description::get_reference_name(target, Some(performing_entity), world);

    let attacker_location = world.get::<Location>(performing_entity);
    let target_location = world.get::<Location>(target);

    if attacker_location.is_none()
        || target_location.is_none()
        || attacker_location != target_location
    {
        return VerifyResult::invalid(
            performing_entity,
            GameMessage::Error(format!("{target_name} is not here.")),
        );
    }

    VerifyResult::valid()
}

/// Verifies that the target is alive.
pub fn verify_target_alive(
    notification: &Notification<VerifyActionNotification, AttackAction>,
    world: &World,
) -> VerifyResult {
    let performing_entity = notification.notification_type.performing_entity;
    let target = notification.contents.target;
    let target_name = Description::get_reference_name(target, Some(performing_entity), world);

    if is_living_entity(target, world) {
        return VerifyResult::valid();
    }

    VerifyResult::invalid(
        performing_entity,
        GameMessage::Error(format!("{target_name} is not alive.")),
    )
}

// Verifies that the attacker has some kind of weapon
pub fn verify_attacker_has_weapon(
    notification: &Notification<VerifyActionNotification, AttackAction>,
    world: &World,
) -> VerifyResult {
    let performing_entity = notification.notification_type.performing_entity;
    if Weapon::get_primary(performing_entity, world).is_none() {
        VerifyResult::invalid(
            performing_entity,
            GameMessage::Error("You don't have a weapon to attack with.".to_string()),
        )
    } else {
        VerifyResult::valid()
    }
}
