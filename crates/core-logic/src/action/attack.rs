use bevy_ecs::prelude::*;
use lazy_static::lazy_static;
use regex::Regex;

use crate::{
    checks::{CheckModifiers, CheckResult, VsCheckParams, VsParticipant},
    component::{
        ActionEndNotification, AfterActionPerformNotification, CombatRange, CombatState, Location,
        Skill, Stats, Vitals, Weapon, WeaponUnusableError,
    },
    input_parser::{
        input_formats_if_has_component, CommandParseError, CommandTarget, InputParseError,
        InputParser,
    },
    is_living_entity,
    notification::{Notification, VerifyResult},
    resource::WeaponTypeStatCatalog,
    verb_forms::VerbForms,
    vital_change::{ValueChangeOperation, VitalChange, VitalType},
    BeforeActionNotification, BodyPart, Description, GameMessage, InternalMessageCategory,
    MessageCategory, MessageDelay, SurroundingsMessageCategory, VerifyActionNotification,
};

use super::{
    handle_enter_combat, Action, ActionInterruptResult, ActionNotificationSender, ActionResult,
    ActionResultBuilder, ThirdPersonMessage, ThirdPersonMessageLocation,
};

/// Multiplier applied to damage done to oneself.
const SELF_DAMAGE_MULT: f32 = 3.0;

/// Multiplier applied to damage done to the head.
const HEAD_DAMAGE_MULT: f32 = 1.2;

/// Multiplier applied to damage done to the torso.
const TORSO_DAMAGE_MULT: f32 = 1.0;

/// Multiplier applied to damage done to non-head and non-torso body parts.
const APPENDAGE_DAMAGE_MULT: f32 = 0.8;

/// The fraction of a target's health that counts as a high amount of damage.
const HIGH_DAMAGE_THRESHOLD: f32 = 0.4;
/// The fraction of a target's health that counts as a low amount of damage.
const LOW_DAMAGE_THRESHOLD: f32 = 0.1;

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
                        let target_name = Description::get_reference_name(
                            target_entity,
                            Some(source_entity),
                            world,
                        );
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
        let mut result_builder = ActionResult::builder();

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
                            .add_possessive_adjective_pronoun(performing_entity)
                            .add_string(format!(" {weapon_name}.")),
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

        let range = CombatState::get_entities_in_combat_with(performing_entity, world)
            .get(&target)
            .copied()
            .unwrap_or_else(|| determine_starting_range(performing_entity, target, world));

        result_builder =
            handle_enter_combat(performing_entity, target, range, result_builder, world);

        let (weapon, weapon_entity) = Weapon::get_primary(performing_entity, world)
            .expect("attacking entity should have a weapon");
        let weapon_hit_verb = weapon.hit_verb.clone();

        // try to perform an initial attack
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

        let (to_hit_result, _) = Stats::check_vs(
            VsParticipant {
                entity: performing_entity,
                stat: WeaponTypeStatCatalog::get_stats(&weapon.weapon_type, world).primary,
                modifiers: CheckModifiers::modify_value(to_hit_modification as f32),
            },
            VsParticipant {
                entity: target,
                stat: Skill::Dodge.into(),
                modifiers: CheckModifiers::none(),
            },
            VsCheckParams::second_wins_ties(),
            world,
        );

        let body_part = BodyPart::random_weighted(world);
        let damage = if to_hit_result.succeeded() {
            let critical = to_hit_result == CheckResult::ExtremeSuccess;
            match weapon.calculate_damage(performing_entity, range, critical, world) {
                Ok(x) => Some(apply_body_part_damage_multiplier(x, body_part)),
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
        } else {
            None
        };

        if let Some(damage) = damage {
            result_builder = handle_damage(
                performing_entity,
                target,
                weapon_entity,
                &weapon_hit_verb,
                damage,
                body_part,
                result_builder,
                world,
            );
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

/// Determines the range the provided entities should begin combat at based on their weapons.
fn determine_starting_range(entity_1: Entity, entity_2: Entity, world: &World) -> CombatRange {
    let range_1 = Weapon::get_primary(entity_1, world)
        .map(|(weapon, _)| *weapon.ranges.usable.end())
        .unwrap_or(CombatRange::Longest);

    let range_2 = Weapon::get_primary(entity_2, world)
        .map(|(weapon, _)| *weapon.ranges.usable.end())
        .unwrap_or(CombatRange::Longest);

    range_1.max(range_2)
}

/// Applies a multiplier to the provided damage based on the provided body part.
fn apply_body_part_damage_multiplier(base_damage: u32, body_part: BodyPart) -> u32 {
    let mult = match body_part {
        BodyPart::Head => HEAD_DAMAGE_MULT,
        BodyPart::Torso => TORSO_DAMAGE_MULT,
        _ => APPENDAGE_DAMAGE_MULT,
    };

    (base_damage as f32 * mult)
        .round()
        .clamp(u32::MIN as f32, u32::MAX as f32) as u32
}

fn handle_weapon_unusable_error(
    entity: Entity,
    target: Entity,
    weapon_entity: Entity,
    error: WeaponUnusableError,
    result_builder: ActionResultBuilder,
    world: &World,
) -> ActionResult {
    let weapon_name = Description::get_reference_name(weapon_entity, Some(entity), world);
    let reason = match error {
        WeaponUnusableError::StatRequirementNotMet(requirement) => format!(
            "your {} is less than {:.1}",
            requirement.stat, requirement.min
        ),
        WeaponUnusableError::OutsideUsableRange { usable, actual } => {
            let distance_phrase = if actual < *usable.start() {
                "close to"
            } else {
                "far away from"
            };
            let target_name = Description::get_reference_name(target, Some(entity), world);
            format!("you are too {distance_phrase} {target_name}")
        }
    };

    result_builder
        .with_message(
            entity,
            format!("You can't use {weapon_name} because {reason}."),
            MessageCategory::Internal(InternalMessageCategory::Action),
            MessageDelay::Short,
        )
        .with_third_person_message(
            Some(entity),
            ThirdPersonMessageLocation::SourceEntity,
            ThirdPersonMessage::new(
                MessageCategory::Surroundings(SurroundingsMessageCategory::Action),
                MessageDelay::Short,
            )
            .add_name(entity)
            .add_string(" flails about uselessly with ")
            .add_name(weapon_entity)
            .add_string("."),
            world,
        )
        .build_complete_should_tick(false)
}

fn handle_damage(
    performing_entity: Entity,
    target: Entity,
    weapon_entity: Entity,
    weapon_hit_verb: &VerbForms,
    damage: u32,
    body_part: BodyPart,
    mut result_builder: ActionResultBuilder,
    world: &mut World,
) -> ActionResultBuilder {
    //TODO instead of having these messages in here, make them defined on the weapons themselves
    let target_health = world
        .get::<Vitals>(target)
        .map(|vitals| &vitals.health)
        .expect("target should have vitals");
    let damage_fraction = damage as f32 / target_health.get_max();
    let (hit_severity_first_person, hit_severity_third_person) =
        if damage_fraction >= HIGH_DAMAGE_THRESHOLD {
            ("mutilate", "mutilates")
        } else if damage_fraction > LOW_DAMAGE_THRESHOLD {
            ("hit", "hits")
        } else {
            ("barely scratch", "barely scratches")
        };

    result_builder = result_builder.with_post_effect(Box::new(move |w| {
        VitalChange {
            entity: target,
            vital_type: VitalType::Health,
            operation: ValueChangeOperation::Subtract,
            amount: damage as f32,
            message: Some(format!("Ow, your {body_part}!")),
        }
        .apply(w);
    }));

    let weapon_name =
        Description::get_reference_name(weapon_entity, Some(performing_entity), world);
    let target_name = Description::get_reference_name(target, Some(performing_entity), world);
    result_builder
        .with_message(
            performing_entity,
            format!(
                "You {} {}'s {} with a {} from {}.",
                hit_severity_first_person,
                target_name,
                body_part,
                weapon_hit_verb.second_person,
                weapon_name
            ),
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
            .add_name(performing_entity)
            .add_string(format!(" {hit_severity_third_person} "))
            .add_name(target)
            .add_string(format!(
                " in the {} with a {} from ",
                body_part, weapon_hit_verb.second_person
            ))
            .add_name(weapon_entity)
            .add_string("!"),
            world,
        )
}

fn handle_miss(
    performing_entity: Entity,
    target: Entity,
    weapon_entity: Entity,
    result_builder: ActionResultBuilder,
    world: &mut World,
) -> ActionResultBuilder {
    let weapon_name =
        Description::get_reference_name(weapon_entity, Some(performing_entity), world);
    let target_name = Description::get_reference_name(target, Some(performing_entity), world);
    result_builder
        .with_message(
            performing_entity,
            format!("You fail to hit {target_name} with {weapon_name}."),
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
            .add_name(performing_entity)
            .add_string(" fails to hit ")
            .add_name(target)
            .add_string(" with ")
            .add_name(weapon_entity)
            .add_string("."),
            world,
        )
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
