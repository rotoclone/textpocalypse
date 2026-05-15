use std::{collections::HashSet, sync::LazyLock};

use bevy_ecs::prelude::*;
use core_logic_derive::ActionBoilerplate;
use nonempty::nonempty;

use crate::{
    ActionTag, AttackType, ChosenWeapon, InternalMessageCategory, MessageCategory, MessageDelay,
    WeaponMessages, check_for_hit,
    combat_utils::AttackCommandFormats,
    command_format::one_of_literal_part,
    component::Weapon,
    find_weapon, handle_begin_attack, handle_damage, handle_hit_error, handle_miss,
    handle_weapon_unusable_error,
    input_parser::{InputParseError, InputParser},
    parse_attack_input,
};

use super::{Action, ActionInterruptResult, ActionNotificationSender, ActionResult};

static COMMAND_FORMATS: LazyLock<AttackCommandFormats<AttackAction>> = LazyLock::new(|| {
    AttackCommandFormats::new_can_attack_self(one_of_literal_part(nonempty!["attack", "kill", "k"]))
});

pub struct AttackParser;

impl InputParser for AttackParser {
    fn parse(
        &self,
        input: &str,
        source_entity: Entity,
        world: &World,
    ) -> Result<Box<dyn Action>, InputParseError> {
        let attack = parse_attack_input(input, source_entity, &COMMAND_FORMATS, world)?;

        Ok(Box::new(AttackAction {
            target: attack.target,
            weapon: attack.weapon,
            notification_sender: ActionNotificationSender::new(),
        }))
    }

    fn get_input_formats(&self) -> Vec<String> {
        COMMAND_FORMATS.get_input_formats()
    }

    fn get_input_formats_for(&self, entity: Entity, _: Entity, world: &World) -> Vec<String> {
        COMMAND_FORMATS.get_input_formats_for(entity, world)
    }
}

/// Makes an entity attack another entity.
#[derive(Debug, ActionBoilerplate)]
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

        let (mut result_builder, range) = if target == performing_entity {
            let weapon = world
                .get::<Weapon>(weapon_entity)
                .expect("weapon should be a weapon");
            (ActionResult::builder(), *weapon.ranges.optimal.start())
        } else {
            handle_begin_attack(performing_entity, target, world)
        };

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
                    );
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
                );
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
