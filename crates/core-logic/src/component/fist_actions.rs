use std::{collections::HashSet, sync::LazyLock};

use bevy_ecs::prelude::*;
use rand::{seq::SliceRandom, thread_rng};

use crate::{
    body_part::BodyPartType,
    check_for_hit,
    combat_utils::{self, AttackCommandFormats},
    command_format::literal_part,
    find_weapon, handle_begin_attack, handle_damage, handle_hit_error, handle_miss,
    handle_weapon_unusable_error,
    input_parser::{InputParseError, InputParser},
    notification::ReturningNotificationHandlers,
    parse_attack_input, Action, ActionEndNotification, ActionInterruptResult,
    ActionNotificationSender, ActionResult, ActionTag, AfterActionPerformNotification, AttackType,
    BasicTokens, BeforeActionNotification, BodyPart, ChosenWeapon, Description, DynamicMessage,
    DynamicMessageLocation, IntegerExtensions, InternalMessageCategory, MessageCategory,
    MessageDelay, MessageFormat, NotificationHandlers, ParseCustomInput,
    SurroundingsMessageCategory, VerifyActionNotification, VerifyResult, Weapon, WeaponMessages,
};

/// A component that provides special attack actions for fists.
#[derive(Component)]
pub struct FistActions {
    /// Messages for the uppercut attack.
    pub uppercut_messages: WeaponMessages,
    /// Messages for the haymaker attack.
    pub haymaker_messages: WeaponMessages,
}

impl ParseCustomInput for FistActions {
    fn get_parsers() -> Vec<Box<dyn InputParser>> {
        vec![Box::new(UppercutParser), Box::new(HaymakerParser)]
    }
}

impl FistActions {
    /// Registers handlers for special fist attacks.
    pub fn register_handlers(world: &mut World) {
        ReturningNotificationHandlers::add_handler(
            combat_utils::verify_combat_action_valid::<UppercutAction>,
            world,
        );
        NotificationHandlers::add_handler(
            combat_utils::equip_before_attack::<UppercutAction>,
            world,
        );

        ReturningNotificationHandlers::add_handler(
            combat_utils::verify_combat_action_valid::<HaymakerAction>,
            world,
        );
        NotificationHandlers::add_handler(
            combat_utils::equip_before_attack::<HaymakerAction>,
            world,
        );
    }
}

/// The amount to modify the to hit bonus by for uppercuts.
const UPPERCUT_TO_HIT_MODIFIER: i16 = -2;

/// The multiplier for damage done by uppercuts.
const UPPERCUT_DAMAGE_MULTIPLIER: f32 = 1.1;

static UPPERCUT_FORMATS: LazyLock<AttackCommandFormats<UppercutAction>> =
    LazyLock::new(|| AttackCommandFormats::new(literal_part("uppercut")));

struct UppercutParser;

impl InputParser for UppercutParser {
    fn parse(
        &self,
        input: &str,
        source_entity: Entity,
        world: &World,
    ) -> Result<Box<dyn Action>, InputParseError> {
        let attack = parse_attack_input(input, source_entity, &UPPERCUT_FORMATS, world)?;

        Ok(Box::new(UppercutAction {
            target: attack.target,
            weapon: attack.weapon,
            notification_sender: ActionNotificationSender::new(),
        }))
    }

    fn get_input_formats(&self) -> Vec<String> {
        UPPERCUT_FORMATS.get_input_formats()
    }

    fn get_input_formats_for(&self, entity: Entity, _: Entity, world: &World) -> Vec<String> {
        UPPERCUT_FORMATS.get_input_formats_for(entity, world)
    }
}

#[derive(Debug)]
pub struct UppercutAction {
    target: Entity,
    weapon: ChosenWeapon,
    notification_sender: ActionNotificationSender<Self>,
}

impl Action for UppercutAction {
    fn perform(&mut self, performing_entity: Entity, world: &mut World) -> ActionResult {
        let target = self.target;
        let weapon_entity =
            match find_weapon::<UppercutAction>(performing_entity, self.weapon, world) {
                Ok(e) => e,
                Err(r) => return r,
            };

        let (mut result_builder, range) = handle_begin_attack(performing_entity, target, world);

        let weapon = world
            .get::<Weapon>(weapon_entity)
            .expect("weapon should be a weapon");

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
                return handle_hit_error(
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
            if let Some(body_part_entity) =
                BodyPart::get(&BodyPartType::Head, target, world).choose(&mut thread_rng())
            {
                hit_params.damage = hit_params.damage.mul_and_round(UPPERCUT_DAMAGE_MULTIPLIER);
                hit_params.body_part = *body_part_entity;
                result_builder = handle_damage::<UppercutAction>(hit_params, result_builder, world);
            } else {
                // target doesn't have a head
                result_builder = handle_miss::<UppercutAction>(
                    performing_entity,
                    target,
                    weapon_entity,
                    result_builder,
                    world,
                );
            }
        } else {
            // missed
            result_builder = handle_miss::<UppercutAction>(
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

    fn get_tags(&self) -> HashSet<ActionTag> {
        [ActionTag::Combat].into()
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
    ) -> Vec<VerifyResult> {
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

impl AttackType for UppercutAction {
    fn can_perform_with(weapon_entity: Entity, world: &World) -> bool {
        world.get::<FistActions>(weapon_entity).is_some()
    }

    fn get_messages(weapon_entity: Entity, world: &World) -> Option<&WeaponMessages> {
        world
            .get::<FistActions>(weapon_entity)
            .map(|fist_actions| &fist_actions.uppercut_messages)
    }

    fn get_target(&self) -> Entity {
        self.target
    }

    fn get_weapon(&self) -> ChosenWeapon {
        self.weapon
    }
}

/// The amount to modify the to hit bonus by for haymakers.
const HAYMAKER_TO_HIT_MODIFIER: i16 = 2;

/// The multiplier for damage done by haymakers.
const HAYMAKER_DAMAGE_MULTIPLIER: f32 = 1.5;

/// The number of ticks to wait before a haymaker lands.
const HAYMAKER_CHARGE_TICKS: u16 = 1;

static HAYMAKER_FORMATS: LazyLock<AttackCommandFormats<HaymakerAction>> =
    LazyLock::new(|| AttackCommandFormats::new(literal_part("haymaker")));

struct HaymakerParser;

impl InputParser for HaymakerParser {
    fn parse(
        &self,
        input: &str,
        source_entity: Entity,
        world: &World,
    ) -> Result<Box<dyn Action>, InputParseError> {
        let attack = parse_attack_input(input, source_entity, &HAYMAKER_FORMATS, world)?;

        Ok(Box::new(HaymakerAction {
            target: attack.target,
            weapon: attack.weapon,
            charge_ticks_remaining: HAYMAKER_CHARGE_TICKS,
            notification_sender: ActionNotificationSender::new(),
        }))
    }

    fn get_input_formats(&self) -> Vec<String> {
        HAYMAKER_FORMATS.get_input_formats()
    }

    fn get_input_formats_for(&self, entity: Entity, _: Entity, world: &World) -> Vec<String> {
        HAYMAKER_FORMATS.get_input_formats_for(entity, world)
    }
}

#[derive(Debug)]
pub struct HaymakerAction {
    target: Entity,
    weapon: ChosenWeapon,
    charge_ticks_remaining: u16,
    notification_sender: ActionNotificationSender<Self>,
}

impl Action for HaymakerAction {
    fn perform(&mut self, performing_entity: Entity, world: &mut World) -> ActionResult {
        let target = self.target;
        let weapon_entity =
            match find_weapon::<HaymakerAction>(performing_entity, self.weapon, world) {
                Ok(e) => e,
                Err(r) => return r,
            };

        let (mut result_builder, range) = handle_begin_attack(performing_entity, target, world);

        let target_name =
            Description::get_reference_name(self.target, Some(performing_entity), world);

        if self.charge_ticks_remaining == HAYMAKER_CHARGE_TICKS {
            self.charge_ticks_remaining -= 1;

            return result_builder
                .with_message(
                    performing_entity,
                    format!("You face {target_name} and wind up for a haymaker."),
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
                            "${attacker.Name} faces ${target.name} and winds up for a haymaker.",
                        )
                        .expect("message format should be valid"),
                        BasicTokens::new()
                            .with_entity("attacker".into(), performing_entity)
                            .with_entity("target".into(), target),
                    ),
                    world,
                )
                .build_incomplete(true);
        } else if self.charge_ticks_remaining != 0 {
            self.charge_ticks_remaining -= 1;

            return result_builder
                .with_message(
                    performing_entity,
                    "You continue preparing for a haymaker.".to_string(),
                    MessageCategory::Internal(InternalMessageCategory::Action),
                    MessageDelay::Short,
                )
                .with_dynamic_message(
                    Some(performing_entity),
                    DynamicMessageLocation::SourceEntity,
                    DynamicMessage::new_third_person(
                        MessageCategory::Surroundings(SurroundingsMessageCategory::Action),
                        MessageDelay::Short,
                        MessageFormat::new("${attacker.Name} continues preparing for a haymaker.")
                            .expect("message format should be valid"),
                        BasicTokens::new().with_entity("attacker".into(), performing_entity),
                    ),
                    world,
                )
                .build_incomplete(true);
        }

        let weapon = world
            .get::<Weapon>(weapon_entity)
            .expect("weapon should be a weapon");

        let to_hit_modification =
            match weapon.calculate_to_hit_modification(performing_entity, range, world) {
                Ok(x) => x + HAYMAKER_TO_HIT_MODIFIER,
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
                return handle_hit_error(
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
            hit_params.damage = hit_params.damage.mul_and_round(HAYMAKER_DAMAGE_MULTIPLIER);
            result_builder = handle_damage::<HaymakerAction>(hit_params, result_builder, world);
        } else {
            result_builder = handle_miss::<HaymakerAction>(
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
            "You stop preparing for a haymaker.".to_string(),
            MessageCategory::Internal(InternalMessageCategory::Action),
            MessageDelay::None,
        )
    }

    fn may_require_tick(&self) -> bool {
        true
    }

    fn get_tags(&self) -> HashSet<ActionTag> {
        [ActionTag::Combat].into()
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
    ) -> Vec<VerifyResult> {
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

impl AttackType for HaymakerAction {
    fn can_perform_with(weapon_entity: Entity, world: &World) -> bool {
        world.get::<FistActions>(weapon_entity).is_some()
    }

    fn get_messages(weapon_entity: Entity, world: &World) -> Option<&WeaponMessages> {
        world
            .get::<FistActions>(weapon_entity)
            .map(|fist_actions| &fist_actions.haymaker_messages)
    }

    fn get_target(&self) -> Entity {
        self.target
    }

    fn get_weapon(&self) -> ChosenWeapon {
        self.weapon
    }
}
