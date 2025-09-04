use std::{collections::HashSet, sync::LazyLock};

use bevy_ecs::prelude::*;
use nonempty::nonempty;

use crate::{
    command_format::{
        entity_part_builder, literal_part, one_of_literal_part,
        validate_parsed_value_has_component, CommandFormat, CommandPartId,
    },
    component::{
        get_hands_to_equip, ActionEndNotification, ActionQueue, AfterActionPerformNotification,
        EquipError, EquippedItems, Item, Location, UnequipError,
    },
    find_wearing_entity, find_wielding_entity,
    input_parser::{input_formats_if_has_component, InputParseError, InputParser},
    notification::{Notification, VerifyResult},
    ActionTag, BasicTokens, BeforeActionNotification, Description, DynamicMessage,
    DynamicMessageLocation, GameMessage, InternalMessageCategory, MessageCategory, MessageDelay,
    MessageFormat, SurroundingsMessageCategory, VerifyActionNotification,
};

use super::{Action, ActionInterruptResult, ActionNotificationSender, ActionResult};

static TARGET_PART_ID: LazyLock<CommandPartId<Entity>> =
    LazyLock::new(|| CommandPartId::new("target"));
static EQUIP_FORMAT: LazyLock<CommandFormat> = LazyLock::new(|| {
    CommandFormat::new(one_of_literal_part(nonempty![
        "equip",
        "hold",
        "wield",
        "unholster",
        "take out",
    ]))
    .then(literal_part(" ").always_include_in_errors())
    .then(
        entity_part_builder(TARGET_PART_ID.clone())
            .with_validator(|context, world| {
                validate_parsed_value_has_component::<Item>(context, "equip", world)
            })
            .build()
            .always_include_in_errors()
            .with_if_unparsed("what")
            .with_literal_for_format_string("thing"),
    )
});
static UNEQUIP_FORMAT: LazyLock<CommandFormat> = LazyLock::new(|| {
    CommandFormat::new(one_of_literal_part(nonempty![
        "unequip", "unhold", "unwield", "holster", "put away",
    ]))
    .then(literal_part(" ").always_include_in_errors())
    .then(
        entity_part_builder(TARGET_PART_ID.clone())
            .with_validator(|context, world| {
                validate_parsed_value_has_component::<Item>(context, "unequip", world)
            })
            .build()
            .always_include_in_errors()
            .with_if_unparsed("what")
            .with_literal_for_format_string("thing"),
    )
});

//TODO split into 2 parsers
pub struct EquipParser;

impl InputParser for EquipParser {
    fn parse(
        &self,
        input: &str,
        source_entity: Entity,
        world: &World,
    ) -> Result<Box<dyn Action>, InputParseError> {
        match EQUIP_FORMAT.parse(input, source_entity, world) {
            Ok(parsed) => {
                return Ok(Box::new(EquipAction {
                    target: parsed.get(&TARGET_PART_ID),
                    should_be_equipped: true,
                    notification_sender: ActionNotificationSender::new(),
                }))
            }
            Err(e) => {
                if e.num_parts_matched() > 0 {
                    return Err(e.into());
                }
            }
        }

        let parsed = UNEQUIP_FORMAT.parse(input, source_entity, world)?;
        Ok(Box::new(EquipAction {
            target: parsed.get(&TARGET_PART_ID),
            should_be_equipped: false,
            notification_sender: ActionNotificationSender::new(),
        }))
    }

    fn get_input_formats(&self) -> Vec<String> {
        vec![
            EQUIP_FORMAT.get_format_description().to_string(),
            UNEQUIP_FORMAT.get_format_description().to_string(),
        ]
    }

    fn get_input_formats_for(&self, entity: Entity, _: Entity, world: &World) -> Vec<String> {
        input_formats_if_has_component::<Item>(
            entity,
            world,
            &[
                EQUIP_FORMAT.get_format_description().with_targeted_entity(
                    TARGET_PART_ID.clone(),
                    entity,
                    world,
                ),
                UNEQUIP_FORMAT
                    .get_format_description()
                    .with_targeted_entity(TARGET_PART_ID.clone(), entity, world),
            ],
        )
    }
}

/// Makes an entity equip or unequip an item.
#[derive(Debug)]
pub struct EquipAction {
    /// The entity to equip or unequip
    pub target: Entity,
    /// Whether the entity should be equipped or unequipped
    pub should_be_equipped: bool,
    pub notification_sender: ActionNotificationSender<Self>,
}

impl Action for EquipAction {
    fn perform(&mut self, performing_entity: Entity, world: &mut World) -> ActionResult {
        let target = self.target;
        let target_name = Description::get_reference_name(target, Some(performing_entity), world);

        if self.should_be_equipped {
            match EquippedItems::equip(performing_entity, target, world) {
                Ok(()) => (),
                Err(EquipError::CannotEquip) => {
                    return ActionResult::builder()
                        .with_error(performing_entity, "You can't equip things.".to_string())
                        .build_complete_no_tick(false)
                }
                Err(EquipError::CannotBeEquipped) => {
                    return ActionResult::builder()
                        .with_error(performing_entity, format!("You can't equip {target_name}."))
                        .build_complete_no_tick(false)
                }
                Err(EquipError::AlreadyEquipped) => {
                    return ActionResult::builder()
                        .with_error(
                            performing_entity,
                            format!("You already have {target_name} equipped."),
                        )
                        .build_complete_no_tick(false)
                }
                Err(EquipError::NotEnoughHands) => {
                    return ActionResult::builder()
                        .with_error(
                            performing_entity,
                            format!("You don't have enough free hands to equip {target_name}."),
                        )
                        .build_complete_no_tick(false);
                }
            }
        } else {
            match EquippedItems::unequip(performing_entity, target, world) {
                Ok(()) => (),
                Err(UnequipError::NotEquipped) => {
                    return ActionResult::builder()
                        .with_error(
                            performing_entity,
                            format!("You don't have {target_name} equipped."),
                        )
                        .build_complete_no_tick(false);
                }
            }
        }

        let (take_out_or_put_away, takes_out_or_puts_away) = if self.should_be_equipped {
            ("take out", "takes out")
        } else {
            ("put away", "puts away")
        };

        ActionResult::builder()
            .with_message(
                performing_entity,
                format!("You {take_out_or_put_away} {target_name}."),
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
                        "${performing_entity.Name} ${takes_out_or_puts_away} ${target.name}.",
                    )
                    .expect("message format should be valid"),
                    BasicTokens::new()
                        .with_entity("performing_entity".into(), performing_entity)
                        .with_string(
                            "takes_out_or_puts_away".into(),
                            takes_out_or_puts_away.to_string(),
                        )
                        .with_entity("target".into(), self.target),
                ),
                world,
            )
            .build_complete_should_tick(true)
    }

    fn interrupt(&self, performing_entity: Entity, _: &mut World) -> ActionInterruptResult {
        if self.should_be_equipped {
            ActionInterruptResult::message(
                performing_entity,
                "You stop equipping things.".to_string(),
                MessageCategory::Internal(InternalMessageCategory::Action),
                MessageDelay::None,
            )
        } else {
            ActionInterruptResult::message(
                performing_entity,
                "You stop unequipping things.".to_string(),
                MessageCategory::Internal(InternalMessageCategory::Action),
                MessageDelay::None,
            )
        }
    }

    fn may_require_tick(&self) -> bool {
        true
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

/// Verifies that the entity trying to equip an item contains it.
pub fn verify_has_item_to_equip(
    notification: &Notification<VerifyActionNotification, EquipAction>,
    world: &World,
) -> VerifyResult {
    let item = notification.contents.target;
    let performing_entity = notification.notification_type.performing_entity;

    if let Some(location) = world.get::<Location>(item) {
        if location.id == performing_entity {
            return VerifyResult::valid();
        }
    }

    let item_name = Description::get_reference_name(item, Some(performing_entity), world);

    VerifyResult::invalid(
        performing_entity,
        GameMessage::Error(format!("You don't have {item_name}.")),
    )
}

/// Verifies that the entity trying to equip an item is not wearing it.
pub fn verify_not_wearing_item_to_equip(
    notification: &Notification<VerifyActionNotification, EquipAction>,
    world: &World,
) -> VerifyResult {
    let item = notification.contents.target;
    let performing_entity = notification.notification_type.performing_entity;

    if let Some(wearing_entity) = find_wearing_entity(item, world) {
        if wearing_entity == performing_entity {
            let item_name = Description::get_reference_name(item, Some(performing_entity), world);
            return VerifyResult::invalid(
                performing_entity,
                GameMessage::Error(format!(
                    "You'll have to take off {item_name} before you can equip it."
                )),
            );
        }
    }

    VerifyResult::valid()
}

/// Attempts to unequip items to make room before equipping another item.
pub fn auto_unequip_on_equip(
    notification: &Notification<BeforeActionNotification, EquipAction>,
    world: &mut World,
) {
    let item = notification.contents.target;
    let performing_entity = notification.notification_type.performing_entity;

    // need to check wielding entity to make sure the item isn't already equipped, to avoid unequipping the entity just to equip it again
    if notification.contents.should_be_equipped && find_wielding_entity(item, world).is_none() {
        // about to equip something
        // NOTE: this verification only works because checking free hands is done as part of the action being performed rather than in a verify handler
        if notification
            .contents
            .send_verify_notification(VerifyActionNotification { performing_entity }, world)
            .is_valid
        {
            if let Some(num_hands_needed) = get_hands_to_equip(item, world) {
                let items_to_unequip = EquippedItems::get_items_to_unequip_to_free_hands(
                    performing_entity,
                    num_hands_needed.get(),
                    world,
                );

                // queue up unequip actions
                for item_to_unequip in items_to_unequip {
                    ActionQueue::queue_first(
                        world,
                        performing_entity,
                        Box::new(EquipAction {
                            target: item_to_unequip,
                            should_be_equipped: false,
                            notification_sender: ActionNotificationSender::new(),
                        }),
                    );
                }
            }
        }
    }
}
