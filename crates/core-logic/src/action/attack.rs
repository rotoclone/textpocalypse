use std::{collections::HashSet, sync::LazyLock};

use bevy_ecs::prelude::*;
use rand::seq::SliceRandom;
use regex::Regex;

use crate::{
    body_part::BodyPartType,
    check_for_hit,
    component::{ActionEndNotification, AfterActionPerformNotification, Vitals, Weapon},
    find_weapon, handle_begin_attack, handle_damage, handle_hit_error, handle_miss,
    handle_weapon_unusable_error,
    input_parser::{input_formats_if_has_component, InputParseError, InputParser},
    notification::VerifyResult,
    parse_attack_input,
    vital_change::{
        ValueChangeOperation, VitalChange, VitalChangeMessageParams, VitalChangeVisualizationType,
        VitalType,
    },
    ActionTag, AttackType, BeforeActionNotification, BodyPart, ChosenWeapon, DynamicMessage,
    DynamicMessageLocation, InternalMessageCategory, MessageCategory, MessageDelay, MessageFormat,
    NoTokens, SurroundingsMessageCategory, VerifyActionNotification, WeaponHitMessageTokens,
    WeaponMessages,
};

use super::{Action, ActionInterruptResult, ActionNotificationSender, ActionResult};

/// Multiplier applied to damage done to oneself.
const SELF_DAMAGE_MULT: f32 = 3.0;

const ATTACK_VERB_NAME: &str = "attack";
const ATTACK_FORMAT: &str = "attack <>";
const NAME_CAPTURE: &str = "name";
const WEAPON_CAPTURE: &str = "weapon";

static ATTACK_PATTERN: LazyLock<Regex> =
    LazyLock::new(|| Regex::new("^(attack|kill|k)( (?P<name>.*))?").unwrap());
static ATTACK_PATTERN_WITH_WEAPON: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new("^(attack|kill|k)( (?P<name>.*))? (with|using) (?P<weapon>.*)").unwrap()
});

pub struct AttackParser;

impl InputParser for AttackParser {
    fn parse(
        &self,
        input: &str,
        source_entity: Entity,
        world: &World,
    ) -> Result<Box<dyn Action>, InputParseError> {
        let attack = parse_attack_input::<AttackAction>(
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
    pub weapon: ChosenWeapon,
    pub notification_sender: ActionNotificationSender<Self>,
}

impl Action for AttackAction {
    fn perform(&mut self, performing_entity: Entity, world: &mut World) -> ActionResult {
        let target = self.target;
        let weapon_entity = match find_weapon::<AttackAction>(performing_entity, self.weapon, world)
        {
            Ok(e) => e,
            Err(r) => return r,
        };
        let result_builder = ActionResult::builder();

        if target == performing_entity {
            if let Some(body_part_entity) =
                BodyPart::get(&BodyPartType::Head, target, world).first()
            {
                let weapon = world
                    .get::<Weapon>(weapon_entity)
                    .expect("weapon should be a weapon");
                let hit_message_format = weapon.default_attack_messages.regular_hit.choose(&mut rand::thread_rng())
                    .cloned()
                    .unwrap_or_else(|| MessageFormat::new("${attacker.Name} ${attacker.you:hit/hits} ${target.themself} in the ${body_part.plain_name} with ${weapon.name}.").expect("message format should be valid"));
                let hit_message_tokens = WeaponHitMessageTokens {
                    attacker: performing_entity,
                    target,
                    weapon: weapon_entity,
                    body_part: *body_part_entity,
                };

                match weapon.calculate_damage(
                    performing_entity,
                    *weapon.ranges.optimal.start(),
                    true,
                    world,
                ) {
                    Ok(damage) => {
                        let message = hit_message_format
                            .interpolate(performing_entity, &hit_message_tokens, world)
                            .expect("self hit message should interpolate properly");
                        VitalChange::<NoTokens> {
                            entity: performing_entity,
                            vital_type: VitalType::Health,
                            operation: ValueChangeOperation::Subtract,
                            amount: damage as f32 * SELF_DAMAGE_MULT,
                            message_params: vec![(
                                VitalChangeMessageParams::Direct {
                                    entity: performing_entity,
                                    message,
                                    category: MessageCategory::Internal(
                                        InternalMessageCategory::Action,
                                    ),
                                },
                                VitalChangeVisualizationType::Full,
                            )],
                        }
                        .apply(world);

                        return result_builder
                            .with_dynamic_message(
                                Some(performing_entity),
                                DynamicMessageLocation::SourceEntity,
                                DynamicMessage::new_third_person(
                                    MessageCategory::Surroundings(
                                        SurroundingsMessageCategory::Action,
                                    ),
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

        if let Some(hit_params) = hit_params {
            result_builder = handle_damage::<AttackAction>(hit_params, result_builder, world);
        } else {
            result_builder = handle_miss::<AttackAction>(
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

impl AttackType for AttackAction {
    fn can_perform_with(_: Entity, _: &World) -> bool {
        true
    }

    fn get_messages(weapon_entity: Entity, world: &World) -> Option<&WeaponMessages> {
        world
            .get::<Weapon>(weapon_entity)
            .map(|weapon| &weapon.default_attack_messages)
    }

    fn get_target(&self) -> Entity {
        self.target
    }

    fn get_weapon(&self) -> ChosenWeapon {
        self.weapon
    }
}
