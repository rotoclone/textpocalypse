use bevy_ecs::prelude::*;
use lazy_static::lazy_static;
use rand::seq::SliceRandom;
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
    ActionQueue, BeforeActionNotification, BodyPart, DefaultAttack, Description, EquipAction,
    EquippedItems, GameMessage, InnateWeapon, InternalMessageCategory, MessageCategory,
    MessageDelay, MessageFormat, SurroundingsMessageCategory, VerifyActionNotification,
    WeaponHitMessageTokens,
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
const WEAPON_CAPTURE: &str = "weapon";

lazy_static! {
    static ref ATTACK_PATTERN: Regex = Regex::new("^(attack|kill|k)( (?P<name>.*))?").unwrap();
    static ref ATTACK_PATTERN_WITH_WEAPON: Regex =
        Regex::new("^(attack|kill|k)( (?P<name>.*))? (with|using) (?P<weapon>.*)").unwrap();
}

pub struct AttackParser;

impl InputParser for AttackParser {
    fn parse(
        &self,
        input: &str,
        source_entity: Entity,
        world: &World,
    ) -> Result<Box<dyn Action>, InputParseError> {
        let attack = parse_attack_input::<DefaultAttack>(
            input,
            source_entity,
            &ATTACK_PATTERN,
            &ATTACK_PATTERN_WITH_WEAPON,
            NAME_CAPTURE,
            WEAPON_CAPTURE,
            ATTACK_VERB_NAME,
            world,
        )?;

        Ok(Box::new(AttackAction {
            target: attack.target,
            weapon: attack.weapon,
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
    pub weapon: Entity,
    pub notification_sender: ActionNotificationSender<Self>,
}

impl Action for AttackAction {
    fn perform(&mut self, performing_entity: Entity, world: &mut World) -> ActionResult {
        let target = self.target;
        let weapon_entity = self.weapon;
        let result_builder = ActionResult::builder();

        if target == performing_entity {
            let weapon = world
                .get::<Weapon>(weapon_entity)
                .expect("weapon should be a weapon");
            let hit_message_format = weapon.messages.hit.choose(&mut rand::thread_rng())
            .cloned()
            .unwrap_or_else(|| MessageFormat::new("${attacker.Name} ${attacker.you:hit/hits} ${target.themself} with ${weapon.name}.").expect("message format should be valid"));
            let hit_message_tokens = WeaponHitMessageTokens {
                attacker: performing_entity,
                target,
                weapon: weapon_entity,
                body_part: BodyPart::Head.to_string(),
            };

            match weapon.calculate_damage(
                performing_entity,
                *weapon.ranges.optimal.start(),
                true,
                world,
            ) {
                Ok(damage) => {
                    VitalChange {
                        entity: performing_entity,
                        vital_type: VitalType::Health,
                        operation: ValueChangeOperation::Subtract,
                        amount: damage as f32 * SELF_DAMAGE_MULT,
                        message: Some(
                            hit_message_format
                                .interpolate(performing_entity, &hit_message_tokens, world)
                                .expect("self hit message should interpolate properly"),
                        ),
                    }
                    .apply(world);

                    return ActionResult::builder()
                        .with_third_person_message(
                            Some(performing_entity),
                            ThirdPersonMessageLocation::SourceEntity,
                            ThirdPersonMessage::new(
                                MessageCategory::Surroundings(SurroundingsMessageCategory::Action),
                                MessageDelay::Short,
                                hit_message_format,
                                hit_message_tokens,
                            ),
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

        let weapon = world
            .get::<Weapon>(weapon_entity)
            .expect("weapon should be a weapon");

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
            result_builder = handle_damage::<DefaultAttack>(hit_params, result_builder, world);
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

/// Verifies that the attacker has the weapon they're trying to attack with.
pub fn verify_attacker_wielding_weapon(
    notification: &Notification<VerifyActionNotification, AttackAction>,
    world: &World,
) -> VerifyResult {
    let performing_entity = notification.notification_type.performing_entity;
    let weapon_entity = notification.contents.weapon;

    if EquippedItems::is_equipped(performing_entity, weapon_entity, world) {
        return VerifyResult::valid();
    }

    // if at least one hand is empty, treat it as being an innate weapon
    if let Some(equipped_items) = world.get::<EquippedItems>(performing_entity) {
        if equipped_items.get_num_hands_free(world) > 0 {
            if let Some((_, innate_weapon_entity)) = InnateWeapon::get(performing_entity, world) {
                if weapon_entity == innate_weapon_entity {
                    return VerifyResult::valid();
                }
            }
        }
    }

    let weapon_name =
        Description::get_reference_name(weapon_entity, Some(performing_entity), world);

    VerifyResult::invalid(
        performing_entity,
        GameMessage::Error(format!("You don't have {weapon_name} equipped.")),
    )
}

/// Queues an action to equip the weapon the attacker is trying to attack with, if they don't already have it equipped.
pub fn equip_before_attack(
    notification: &Notification<BeforeActionNotification, AttackAction>,
    world: &mut World,
) {
    let performing_entity = notification.notification_type.performing_entity;
    let weapon_entity = notification.contents.weapon;

    if EquippedItems::is_equipped(performing_entity, weapon_entity, world) {
        // the weapon is already equipped, no need to do anything
        return;
    }

    // if the weapon is an innate weapon, and the attacker has no free hands, unequip something
    if let Some((_, innate_weapon_entity)) = InnateWeapon::get(performing_entity, world) {
        if weapon_entity == innate_weapon_entity {
            let items_to_unequip =
                EquippedItems::get_items_to_unequip_to_free_hands(performing_entity, 1, world);
            for item in items_to_unequip {
                ActionQueue::queue_first(
                    world,
                    performing_entity,
                    Box::new(EquipAction {
                        target: item,
                        should_be_equipped: false,
                        notification_sender: ActionNotificationSender::new(),
                    }),
                );
            }
            return;
        }
    }

    // the weapon isn't an innate weapon, and it's not equipped, so try to equip it
    ActionQueue::queue_first(
        world,
        performing_entity,
        Box::new(EquipAction {
            target: weapon_entity,
            should_be_equipped: true,
            notification_sender: ActionNotificationSender::new(),
        }),
    );
}
