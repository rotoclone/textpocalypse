use std::{collections::HashSet, sync::LazyLock};

use bevy_ecs::prelude::*;
use regex::Regex;

use crate::{
    action::{Action, ActionInterruptResult, ActionNotificationSender, ActionResult, OpenAction},
    input_parser::{
        input_formats_if_has_component, CommandParseError, CommandTarget, InputParseError,
        InputParser,
    },
    notification::{Notification, VerifyResult},
    ActionTag, AttributeDescription, BasicTokens, DynamicMessage, DynamicMessageLocation,
    GameMessage, InternalMessageCategory, MessageCategory, MessageDelay, MessageFormat,
    SurroundingsMessageCategory,
};

use super::{
    ActionEndNotification, ActionQueue, AfterActionPerformNotification, AttributeDescriber,
    AttributeDetailLevel, BeforeActionNotification, Connection, Container, DescribeAttributes,
    Description, Location, ParseCustomInput, VerifyActionNotification,
};

const UNLOCK_VERB_NAME: &str = "unlock";
const LOCK_VERB_NAME: &str = "lock";
const UNLOCK_FORMAT: &str = "unlock <>";
const LOCK_FORMAT: &str = "lock <>";
const NAME_CAPTURE: &str = "name";

static UNLOCK_PATTERN: LazyLock<Regex> =
    LazyLock::new(|| Regex::new("^unlock (the )?(?P<name>.*)").unwrap());
static LOCK_PATTERN: LazyLock<Regex> =
    LazyLock::new(|| Regex::new("^lock (the )?(?P<name>.*)").unwrap());

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

    fn get_input_formats_for(
        &self,
        entity: Entity,
        _: Entity,
        world: &World,
    ) -> Option<Vec<String>> {
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

        let name = Description::get_reference_name(self.target, Some(performing_entity), world);

        // make sure the performing entity has the key to this lock, if needed
        let mut key = None;
        if let Some(key_id) = &lock.key_id {
            if let Some(inventory) = world.get::<Container>(performing_entity) {
                let mut matching_keys = inventory.find_recursive(
                    |entity| world.get::<KeyId>(entity) == Some(key_id),
                    performing_entity,
                    world,
                );

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
            let key_name = Description::get_reference_name(key, Some(performing_entity), world);
            format!("use {key_name} to ")
        } else {
            "".to_string()
        };

        let (message_format, message_tokens) = if let Some(key) = key {
            (MessageFormat::new(
                "${performing_entity.Name} uses ${key.Name} to ${lock_or_unlock} ${target.name}.",
            )
            .expect("message format should be valid"),
            BasicTokens::new().with_entity("performing_entity".into(), performing_entity).with_entity("key".into(), key).with_string("lock_or_unlock".into(), lock_or_unlock.to_string()).with_entity("target".into(), self.target)
        )
        } else {
            (
                MessageFormat::new("${performing_entity.Name} ${locks_or_unlocks} ${target.name}.")
                    .expect("message format should be valid"),
                BasicTokens::new()
                    .with_entity("performing_entity".into(), performing_entity)
                    .with_string("locks_or_unlocks".into(), locks_or_unlocks.to_string())
                    .with_entity("target".into(), self.target),
            )
        };

        ActionResult::builder()
            .with_message(
                performing_entity,
                format!("You {first_person_key_message}{lock_or_unlock} {name}."),
                MessageCategory::Internal(InternalMessageCategory::Action),
                MessageDelay::Short,
            )
            .with_dynamic_message(
                Some(performing_entity),
                DynamicMessageLocation::SourceEntity,
                DynamicMessage::new_third_person(
                    MessageCategory::Surroundings(SurroundingsMessageCategory::Action),
                    MessageDelay::Short,
                    message_format,
                    message_tokens,
                ),
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
                    DynamicMessage::new_third_person(
                        MessageCategory::Surroundings(SurroundingsMessageCategory::Action),
                        MessageDelay::Short,
                        MessageFormat::new(
                            "The lock on ${other_side.name} clicks ${open_or_closed}.",
                        )
                        .expect("message format should be valid"),
                        BasicTokens::new()
                            .with_entity("other_side".into(), other_side_id)
                            .with_string("open_or_closed".into(), open_or_closed.to_string()),
                    )
                    .send(
                        None,
                        DynamicMessageLocation::Location(location.id),
                        world,
                    );
                }
            }
        }
    }
}

impl ParseCustomInput for KeyedLock {
    fn get_parsers() -> Vec<Box<dyn InputParser>> {
        vec![Box::new(LockParser)]
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
                ActionQueue::queue_first(
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
                        decorations: Vec::new(),
                    },
                );
            }
        }
    }

    VerifyResult::valid()
}
