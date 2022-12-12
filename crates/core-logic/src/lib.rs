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
        let player_id = world.spawn((desc, message_channel)).id();
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
        Ok(action) => {
            debug!("Parsed input into action: {action:?}");
            drop(read_world);
            perform_action(&mut world.write().unwrap(), entity, action);
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
fn perform_action(world: &mut World, performing_entity: Entity, mut action: Box<dyn Action>) {
    debug!("Entity {performing_entity:?} is performing action {action:?}");
    action.send_before_notification(BeforeActionNotification { performing_entity }, world);
    loop {
        let result = action.perform(performing_entity, world);
        for (entity_id, messages) in result.messages {
            for message in messages {
                send_message(world, entity_id, message);
            }
        }

        if result.should_tick {
            tick(world);
        }

        if result.is_complete {
            break;
        }

        //TODO interrupt action if something that would interrupt it has happened, like a hostile entity entering the performing entity's room
    }
}

/// Performs one game tick.
fn tick(world: &mut World) {
    world.resource_mut::<Time>().tick();

    //TODO perform queued actions
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
                        //TODO queue up the action instead so it's done all proper-like
                        perform_action(
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
