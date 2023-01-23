use bevy_ecs::prelude::*;
use lazy_static::lazy_static;
use regex::Regex;

use crate::{
    action::{
        Action, ActionInterruptResult, ActionNotificationSender, ActionResult, OpenAction,
        ThirdPersonMessage, ThirdPersonMessageLocation,
    },
    get_reference_name,
    input_parser::{
        input_formats_if_has_component, CommandParseError, CommandTarget, InputParseError,
        InputParser,
    },
    notification::{Notification, VerifyResult},
    AttributeDescription, GameMessage, InternalMessageCategory, MessageCategory, MessageDelay,
    SurroundingsMessageCategory,
};

use super::{
    queue_action_first, ActionEndNotification, AfterActionPerformNotification, AttributeDescriber,
    AttributeDetailLevel, BeforeActionNotification, Connection, Container, DescribeAttributes,
    Description, Location, ParseCustomInput, VerifyActionNotification,
};

const UNLOCK_VERB_NAME: &str = "unlock";
const LOCK_VERB_NAME: &str = "lock";
const UNLOCK_FORMAT: &str = "unlock <>";
const LOCK_FORMAT: &str = "lock <>";
const NAME_CAPTURE: &str = "name";

lazy_static! {
    static ref UNLOCK_PATTERN: Regex = Regex::new("^unlock (the )?(?P<name>.*)").unwrap();
    static ref LOCK_PATTERN: Regex = Regex::new("^lock (the )?(?P<name>.*)").unwrap();
}

pub struct LockParser;

impl InputParser for LockParser {
    fn parse(
        &self,
        input: &str,
        source_entity: Entity,
        world: &World,
    ) -> Result<Box<dyn Action>, InputParseError> {
        let (captures, verb_name, should_be_locked) =
            if let Some(captures) = UNLOCK_PATTERN.captures(input) {
                (captures, UNLOCK_VERB_NAME, false)
            } else if let Some(captures) = LOCK_PATTERN.captures(input) {
                (captures, LOCK_VERB_NAME, true)
            } else {
                return Err(InputParseError::UnknownCommand);
            };

        if let Some(target_match) = captures.name(NAME_CAPTURE) {
            let command_target = CommandTarget::parse(target_match.as_str());
            if let Some(target) = command_target.find_target_entity(source_entity, world) {
                Ok(Box::new(LockAction {
                    target,
                    should_be_locked,
                    notification_sender: ActionNotificationSender::new(),
                }))
            } else {
                Err(InputParseError::CommandParseError {
                    verb: verb_name.to_string(),
                    error: CommandParseError::TargetNotFound(command_target),
                })
            }
        } else {
            Err(InputParseError::CommandParseError {
                verb: verb_name.to_string(),
                error: CommandParseError::MissingTarget,
            })
        }
    }

    fn get_input_formats(&self) -> Vec<String> {
        vec![UNLOCK_FORMAT.to_string(), LOCK_FORMAT.to_string()]
    }

    fn get_input_formats_for(&self, entity: Entity, world: &World) -> Option<Vec<String>> {
        input_formats_if_has_component::<KeyedLock>(entity, world, &[UNLOCK_FORMAT, LOCK_FORMAT])
    }
}

#[derive(Debug)]
pub struct LockAction {
    pub target: Entity,
    pub should_be_locked: bool,
    pub notification_sender: ActionNotificationSender<Self>,
}

impl Action for LockAction {
    fn perform(&mut self, performing_entity: Entity, world: &mut World) -> ActionResult {
        let lock = match world.get::<KeyedLock>(self.target) {
            Some(s) => s,
            None => {
                if self.should_be_locked {
                    return ActionResult::error(
                        performing_entity,
                        "You can't lock that.".to_string(),
                    );
                } else {
                    return ActionResult::error(
                        performing_entity,
                        "You can't unlock that.".to_string(),
                    );
                }
            }
        };

        if lock.is_locked == self.should_be_locked {
            if lock.is_locked {
                return ActionResult::message(
                    performing_entity,
                    "It's already locked.".to_string(),
                    MessageCategory::Internal(InternalMessageCategory::Misc),
                    MessageDelay::Short,
                    false,
                );
            } else {
                return ActionResult::message(
                    performing_entity,
                    "It's already unlocked.".to_string(),
                    MessageCategory::Internal(InternalMessageCategory::Misc),
                    MessageDelay::Short,
                    false,
                );
            }
        }

        let name = get_reference_name(self.target, Some(performing_entity), world);

        // make sure the performing entity has the key to this lock, if needed
        let mut key = None;
        if let Some(key_id) = &lock.key_id {
            if let Some(inventory) = world.get::<Container>(performing_entity) {
                let mut matching_keys = inventory
                    .find_recursive(|entity| world.get::<KeyId>(entity) == Some(key_id), world);

                key = matching_keys.pop();
            }
        }

        if lock.key_id.is_some() && key.is_none() {
            return ActionResult::error(
                performing_entity,
                format!("You don't have the key to {name}."),
            );
        }

        KeyedLock::set_locked(self.target, self.should_be_locked, world);

        let (lock_or_unlock, locks_or_unlocks) = if self.should_be_locked {
            ("lock", "locks")
        } else {
            ("unlock", "unlocks")
        };

        let first_person_key_message = if let Some(key) = key {
            let key_name = get_reference_name(key, Some(performing_entity), world);
            format!("use {key_name} to ")
        } else {
            "".to_string()
        };

        let mut third_person_message = ThirdPersonMessage::new(
            MessageCategory::Surroundings(SurroundingsMessageCategory::Action),
            MessageDelay::Short,
        )
        .add_entity_name(performing_entity);

        if let Some(key) = key {
            third_person_message = third_person_message
                .add_string(" uses ")
                .add_entity_name(key)
                .add_string(format!(" to {lock_or_unlock} "));
        } else {
            third_person_message = third_person_message.add_string(format!(" {locks_or_unlocks} "));
        }

        third_person_message = third_person_message
            .add_entity_name(self.target)
            .add_string(".");

        ActionResult::builder()
            .with_message(
                performing_entity,
                format!("You {first_person_key_message}{lock_or_unlock} {name}."),
                MessageCategory::Internal(InternalMessageCategory::Action),
                MessageDelay::Short,
            )
            .with_third_person_message(
                Some(performing_entity),
                ThirdPersonMessageLocation::SourceEntity,
                third_person_message,
                world,
            )
            .build_complete_should_tick(true)
    }

    fn interrupt(&self, performing_entity: Entity, _: &mut World) -> ActionInterruptResult {
        let locking_or_unlocking = if self.should_be_locked {
            "locking"
        } else {
            "unlocking"
        };
        ActionInterruptResult::message(
            performing_entity,
            format!("You stop {locking_or_unlocking}."),
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

/// The ID of an entity that can be used to lock or unlock things.
#[derive(Component, Clone, PartialEq, Eq)]
pub struct KeyId(pub u32);

/// Describes whether an entity is locked or unlocked using a key.
/// TODO make locks entities that are installed on things, rather than components
#[derive(Component)]
pub struct KeyedLock {
    /// Whether the entity is locked.
    pub is_locked: bool,
    /// The id of the key used to lock or unlock the entity, if a key is needed.
    pub key_id: Option<KeyId>,
}

impl KeyedLock {
    /// Sets the locked state of the provided entity.
    pub fn set_locked(entity: Entity, should_be_locked: bool, world: &mut World) {
        // this side
        if let Some(mut lock) = world.get_mut::<KeyedLock>(entity) {
            lock.is_locked = should_be_locked;
        }

        // other side
        if let Some(other_side_id) = world.get::<Connection>(entity).and_then(|c| c.other_side) {
            if let Some(mut other_side_lock) = world.get_mut::<KeyedLock>(other_side_id) {
                other_side_lock.is_locked = should_be_locked;

                // send messages to entities on the other side
                if let Some(location) = world.get::<Location>(other_side_id) {
                    let open_or_closed = if should_be_locked { "closed" } else { "open" };
                    ThirdPersonMessage::new(
                        MessageCategory::Surroundings(SurroundingsMessageCategory::Action),
                        MessageDelay::Short,
                    )
                    .add_string("The lock on ")
                    .add_entity_name(other_side_id)
                    .add_string(format!(" clicks {open_or_closed}."))
                    .send(
                        None,
                        ThirdPersonMessageLocation::Location(location.id),
                        world,
                    );
                }
            }
        }
    }
}

impl ParseCustomInput for KeyedLock {
    fn get_parser() -> Box<dyn InputParser> {
        Box::new(LockParser)
    }
}

/// Describes whether the entity is locked or not.
#[derive(Debug)]
struct KeyedLockAttributeDescriber;

impl AttributeDescriber for KeyedLockAttributeDescriber {
    fn describe(
        &self,
        _: Entity,
        entity: Entity,
        _: AttributeDetailLevel,
        world: &World,
    ) -> Vec<AttributeDescription> {
        if let Some(locked_state) = world.get::<KeyedLock>(entity) {
            let description = if locked_state.is_locked {
                "locked"
            } else {
                "unlocked"
            };

            return vec![AttributeDescription::is(description.to_string())];
        }

        Vec::new()
    }
}

impl DescribeAttributes for KeyedLock {
    fn get_attribute_describer() -> Box<dyn super::AttributeDescriber> {
        Box::new(KeyedLockAttributeDescriber)
    }
}

/// Attempts to unlock keyed locks automatically before an attempt is made to open a locked one.
pub fn auto_unlock_keyed_locks(
    notification: &Notification<BeforeActionNotification, OpenAction>,
    world: &mut World,
) {
    if notification.contents.should_be_open {
        if let Some(keyed_lock) = world.get::<KeyedLock>(notification.contents.target) {
            if keyed_lock.is_locked {
                queue_action_first(
                    world,
                    notification.notification_type.performing_entity,
                    Box::new(LockAction {
                        target: notification.contents.target,
                        should_be_locked: false,
                        notification_sender: ActionNotificationSender::new(),
                    }),
                );
            }
        }
    }
}

/// Notification handler for preventing entities from opening entities locked with a keyed lock.
pub fn prevent_opening_locked_keyed_locks(
    notification: &Notification<VerifyActionNotification, OpenAction>,
    world: &World,
) -> VerifyResult {
    if notification.contents.should_be_open {
        if let Some(keyed_lock) = world.get::<KeyedLock>(notification.contents.target) {
            if keyed_lock.is_locked {
                let message = world
                    .get::<Description>(notification.contents.target)
                    .map_or("It's locked.".to_string(), |desc| {
                        format!("The {} is locked.", desc.name)
                    });
                return VerifyResult::invalid(
                    notification.notification_type.performing_entity,
                    GameMessage::Message {
                        content: message,
                        category: MessageCategory::Internal(InternalMessageCategory::Misc),
                        delay: MessageDelay::Short,
                    },
                );
            }
        }
    }

    VerifyResult::valid()
}
