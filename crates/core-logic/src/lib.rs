use bevy_ecs::prelude::*;
use flume::{Receiver, Sender};
use input_parser::InputParser;
use log::debug;
use std::{
    sync::{Arc, RwLock},
    thread,
};

mod action;
use action::*;

mod component;
pub use component::AttributeDescription;
pub use component::AttributeType;
use component::*;

mod time;
pub use time::Time;

mod input_parser;
use input_parser::*;

mod world_setup;
use world_setup::*;

mod game_message;
pub use game_message::*;

mod notification;
use notification::*;

mod direction;
pub use direction::Direction;

mod game_map;
pub use game_map::MapChar;
pub use game_map::MapIcon;
pub use game_map::CHARS_PER_TILE;
use game_map::*;

mod color;
pub use color::Color;

/// A notification sent before an action is performed.
#[derive(Debug)]
pub struct BeforeActionNotification {
    pub performing_entity: Entity,
}

impl NotificationType for BeforeActionNotification {}

/// A notification sent after an action is performed.
#[derive(Debug)]
pub struct AfterActionNotification {
    pub performing_entity: Entity,
}

impl NotificationType for AfterActionNotification {}

#[derive(Component)]
pub struct SpawnRoom;

pub struct Game {
    world: Arc<RwLock<World>>,
}

impl Default for Game {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Resource)]
struct StandardInputParsers {
    parsers: Vec<Box<dyn InputParser>>,
}

impl StandardInputParsers {
    pub fn new() -> StandardInputParsers {
        StandardInputParsers {
            parsers: vec![
                Box::new(MoveParser),
                Box::new(LookParser),
                Box::new(OpenParser),
                Box::new(WaitParser),
                Box::new(HelpParser),
            ],
        }
    }
}

impl Game {
    /// Creates a game with a new, empty world
    pub fn new() -> Game {
        let mut world = World::new();
        world.insert_resource(Time::new());
        world.insert_resource(GameMap::new());
        world.insert_resource(StandardInputParsers::new());
        set_up_world(&mut world);
        NotificationHandlers::add_handler(auto_open_doors, &mut world);
        Game {
            world: Arc::new(RwLock::new(world)),
        }
    }

    /// Adds a player to the game in the default spawn location.
    pub fn add_player(&self, name: String) -> (Sender<String>, Receiver<(GameMessage, Time)>) {
        // create channels for communication between the player and the world
        let (commands_sender, commands_receiver) = flume::unbounded::<String>();
        let (messages_sender, messages_receiver) = flume::unbounded::<(GameMessage, Time)>();

        // add the player to the world
        let mut world = self.world.write().unwrap();
        let desc = Description {
            name: name.clone(),
            room_name: name,
            article: None,
            aliases: Vec::new(),
            description: "A human-shaped person-type thing.".to_string(),
            attribute_describers: Vec::new(),
        };
        let message_channel = MessageChannel {
            sender: messages_sender,
        };
        let player_id = world.spawn((Player, desc, message_channel)).id();
        let spawn_room_id = find_spawn_room(&mut world);
        move_entity(player_id, spawn_room_id, &mut world);

        let player_thread_world = Arc::clone(&self.world);

        // set up thread for handling input from the player
        thread::Builder::new()
            .name(format!("command receiver for player {player_id:?}"))
            .spawn(move || loop {
                let input = match commands_receiver.recv() {
                    Ok(c) => c,
                    Err(_) => {
                        debug!("Command sender for player {player_id:?} has been dropped");
                        break;
                    }
                };
                debug!("Received input: {input:?}");
                handle_input(&player_thread_world, input, player_id);
            })
            .unwrap_or_else(|e| {
                panic!("failed to spawn thread to handle input for player {player_id:?}: {e}")
            });

        // send the player an initial message with their location
        let room = world
            .get::<Room>(spawn_room_id)
            .expect("Spawn room should be a room");
        send_message(
            &world,
            player_id,
            GameMessage::Room(RoomDescription::from_room(room, player_id, &world)),
        );

        (commands_sender, messages_receiver)
    }
}

/// Finds the ID of the single spawn room in the provided world.
fn find_spawn_room(world: &mut World) -> Entity {
    world
        .query::<(Entity, With<SpawnRoom>)>()
        .get_single(world)
        .expect("A spawn room should exist")
        .0
}

/// Determines whether the provided entity has an active message channel for receiving messages.
fn can_receive_messages(world: &World, entity_id: Entity) -> bool {
    world.entity(entity_id).contains::<MessageChannel>()
}

/// Sends a message to the provided entity, if possible. Panics if the entity's message receiver has been dropped.
fn send_message(world: &World, entity_id: Entity, message: GameMessage) {
    if let Some(channel) = world.get::<MessageChannel>(entity_id) {
        channel
            .sender
            .send((message, world.resource::<Time>().clone()))
            .expect("Message receiver should exist");
    }
}

/// Handles input from an entity.
fn handle_input(world: &Arc<RwLock<World>>, input: String, entity: Entity) {
    let read_world = world.read().unwrap();
    match parse_input(&input, entity, &read_world) {
        Ok(mut action) => {
            debug!("Parsed input into action: {action:?}");
            drop(read_world);
            let mut write_world = world.write().unwrap();
            if action.may_require_tick() {
                queue_action(&mut write_world, entity, action);
                try_perform_queued_actions(&mut write_world);
            } else {
                perform_action(&mut write_world, entity, &mut action);
            }
        }
        Err(e) => handle_input_error(entity, e, &read_world),
    }
}

/// Sends a message to an entity based on the provided input parsing error.
fn handle_input_error(entity: Entity, error: InputParseError, world: &World) {
    let message = match error {
        InputParseError::UnknownCommand => "I don't understand that.".to_string(),
        InputParseError::CommandParseError { verb, error } => match error {
            CommandParseError::MissingTarget => format!("'{verb}' requires more targets."),
            CommandParseError::TargetNotFound(t) => {
                format!("There is no '{t}' here.")
            }
            CommandParseError::Other(e) => e,
        },
    };

    send_message(world, entity, GameMessage::Error(message));
}

/// Makes the provided entity perform the provided action.
fn perform_action(
    world: &mut World,
    performing_entity: Entity,
    action: &mut Box<dyn Action>,
) -> ActionResult {
    debug!("Entity {performing_entity:?} is performing action {action:?}");
    action.send_before_notification(BeforeActionNotification { performing_entity }, world);
    //TODO sending the notification might have queued an action before this one, so...deal with that
    let result = action.perform(performing_entity, world);
    //TODO these messages are sent before any relevant tick happens, so before the time is updated, which means the displayed time is wrong
    for (entity_id, messages) in &result.messages {
        for message in messages {
            send_message(world, *entity_id, message.clone());
        }
    }

    result
}

/// Queues an action for the provided entity
fn queue_action(world: &mut World, performing_entity: Entity, action: Box<dyn Action>) {
    if let Some(mut action_queue) = world.get_mut::<ActionQueue>(performing_entity) {
        action_queue.actions.push_back(action);
    } else {
        world.entity_mut(performing_entity).insert(ActionQueue {
            actions: [action].into(),
        });
    }
}

/// Queues an action for the provided entity to perform before its other queued actions.
fn queue_action_first(world: &mut World, performing_entity: Entity, action: Box<dyn Action>) {
    if let Some(mut action_queue) = world.get_mut::<ActionQueue>(performing_entity) {
        action_queue.actions.push_front(action);
    } else {
        world.entity_mut(performing_entity).insert(ActionQueue {
            actions: [action].into(),
        });
    }
}

/// Performs queued actions if all players have one queued.
fn try_perform_queued_actions(world: &mut World) {
    loop {
        debug!("Performing queued actions...");
        let mut entities_with_actions = Vec::new();
        for (entity, action_queue, _) in world
            .query::<(Entity, &ActionQueue, With<Player>)>()
            .iter_mut(world)
        {
            if action_queue.actions.is_empty() {
                // somebody doesn't have any action queued yet, so don't perform any
                debug!("{entity:?} has no queued actions, not performing any");
                return;
            }

            debug!("{entity:?} has a queued action");
            entities_with_actions.push(entity);
        }

        if entities_with_actions.is_empty() {
            return;
        }

        let mut should_tick = false;
        for entity in entities_with_actions {
            // unwrap is safe here because the only entities that can be in `entities_with_actions` are ones with an `ActionQueue` component
            let mut action_queue = world.get_mut::<ActionQueue>(entity).unwrap();

            // `action_queue.actions` is guaranteed to have at least one element in it because if it didn't the entity wouldn't have been added to `entities_with_actions`
            let mut action = action_queue.actions.get_mut(0).unwrap();

            action.send_before_notification(
                BeforeActionNotification {
                    performing_entity: entity,
                },
                world,
            );

            if let Some(action) = action_queue.actions.get_mut(0) {
                let result = perform_action(world, entity, &mut action);

                if result.should_tick {
                    should_tick = true;
                }

                if result.is_complete {
                    //TODO action_queue.actions.remove
                }
            }

            let result = perform_action(world, entity, &mut action);

            if result.should_tick {
                should_tick = true;
            }

            if !result.is_complete {
                //TODO interrupt action if something that would interrupt it has happened, like a hostile entity entering the performing entity's room
                //TODO queue_action_first(world, entity, action);
            }
        }

        if should_tick {
            tick(world);
        }

        /* TODO remove
        let results = actions
            .into_iter()
            .map()
            .map(|(entity, mut action)| {
                let result = perform_action(world, entity, &mut action);
                (entity, action, result)
            })
            .collect::<Vec<(Entity, Box<dyn Action>, ActionResult)>>();

        if results.iter().any(|(_, _, result)| result.should_tick) {
            tick(world);
        }

        for (entity, action, result) in results.into_iter() {
            if !result.is_complete {
                //TODO interrupt action if something that would interrupt it has happened, like a hostile entity entering the performing entity's room
                queue_action_first(world, entity, action);
            }
        }
        */
    }
}

/// Performs one game tick.
fn tick(world: &mut World) {
    //TODO perform queued actions on non-player entities
    world.resource_mut::<Time>().tick();
}

/// Moves an entity to a room.
fn move_entity(entity_id: Entity, destination_room_id: Entity, world: &mut World) {
    //TODO handle moving between non-room entities

    // remove from source room, if necessary
    if let Some(location) = world.get_mut::<Location>(entity_id) {
        let source_room_id = location.id;
        if let Some(mut source_room) = world.get_mut::<Room>(source_room_id) {
            source_room.entities.remove(&entity_id);
        }
    }

    // add to destination room
    world
        .get_mut::<Room>(destination_room_id)
        .expect("Destination entity should be a room")
        .entities
        .insert(entity_id);

    // update location
    world.entity_mut(entity_id).insert(Location {
        id: destination_room_id,
    });
}

/// Builds a string to use to refer to the provided entity.
///
/// For example, if the entity is named "book", this will return "the book".
fn get_reference_name(entity: Entity, world: &World) -> String {
    //TODO handle proper names, like names of people
    world
        .get::<Description>(entity)
        .map_or("it".to_string(), |n| format!("the {}", n.name))
}

/// Attempts to open doors automatically before an attempt is made to move through a closed one.
fn auto_open_doors(
    notification: &Notification<BeforeActionNotification, MoveAction>,
    world: &mut World,
) {
    if let Some(current_location) =
        world.get::<Location>(notification.notification_type.performing_entity)
    {
        if let Some(room) = world.get::<Room>(current_location.id) {
            if let Some((connecting_entity, _)) =
                room.get_connection_in_direction(&notification.contents.direction, world)
            {
                if let Some(open_state) = world.get::<OpenState>(connecting_entity) {
                    if !open_state.is_open {
                        queue_action_first(
                            world,
                            notification.notification_type.performing_entity,
                            Box::new(OpenAction {
                                target: connecting_entity,
                                should_be_open: true,
                            }),
                        );
                    }
                }
            }
        }
    }
}
