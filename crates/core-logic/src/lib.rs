use bevy_ecs::prelude::*;
use flume::{Receiver, Sender};
use input_parser::InputParser;
use log::debug;
use std::{
    collections::HashMap,
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
                Box::new(InventoryParser),
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
        NotificationHandlers::add_handler(auto_open_connections, &mut world);
        NotificationHandlers::add_handler(look_after_move, &mut world);
        VerifyNotificationHandlers::add_handler(
            prevent_moving_through_closed_connections,
            &mut world,
        );
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
        let player_id = world
            .spawn((
                Player,
                Container::new(Some(Volume(10.0)), Some(Weight(100.0))),
                desc,
                message_channel,
            ))
            .id();
        let spawn_room_id = find_spawn_room(&mut world);
        move_entity(player_id, spawn_room_id, &mut world);

        // add stuff to the player's inventory
        let medium_thing_id = world
            .spawn((
                Description {
                    name: "medium thing".to_string(),
                    room_name: "medium thing".to_string(),
                    article: Some("a".to_string()),
                    aliases: vec!["thing".to_string()],
                    description: "Some kind of medium-sized thing.".to_string(),
                    attribute_describers: Vec::new(),
                },
                Volume(0.1),
                Weight(0.5),
            ))
            .id();
        move_entity(medium_thing_id, player_id, &mut world);

        let heavy_thing_id = world
            .spawn((
                Description {
                    name: "heavy thing".to_string(),
                    room_name: "heavy thing".to_string(),
                    article: Some("a".to_string()),
                    aliases: vec!["thing".to_string()],
                    description: "Some kind of heavy thing.".to_string(),
                    attribute_describers: Vec::new(),
                },
                Volume(0.5),
                Weight(10.0),
            ))
            .id();
        move_entity(heavy_thing_id, player_id, &mut world);

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
        let container = world
            .get::<Container>(spawn_room_id)
            .expect("Spawn room should be a container");
        send_message(
            &world,
            player_id,
            GameMessage::Room(RoomDescription::from_room(
                room, container, player_id, &world,
            )),
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
            let mut write_world = world.write().unwrap();
            if action.may_require_tick() {
                queue_action(&mut write_world, entity, action);
            } else {
                queue_action_first(&mut write_world, entity, action);
            }
            try_perform_queued_actions(&mut write_world);
        }
        Err(e) => handle_input_error(entity, e, &read_world),
    }
}

/// Sends multiple messages.
fn send_messages(messages_map: &HashMap<Entity, Vec<GameMessage>>, world: &World) {
    for (entity_id, messages) in messages_map {
        for message in messages {
            send_message(world, *entity_id, message.clone());
        }
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

/// Performs one game tick.
fn tick(world: &mut World) {
    //TODO perform queued actions on non-player entities
    world.resource_mut::<Time>().tick();
}

/// Moves an entity to a container.
fn move_entity(moving_entity: Entity, destination_entity: Entity, world: &mut World) {
    // remove from source container, if necessary
    if let Some(location) = world.get_mut::<Location>(moving_entity) {
        let source_location_id = location.id;
        if let Some(mut source_location) = world.get_mut::<Container>(source_location_id) {
            source_location.entities.remove(&moving_entity);
        }
    }

    // add to destination container
    world
        .get_mut::<Container>(destination_entity)
        .expect("Destination entity should be a container")
        .entities
        .insert(moving_entity);

    // update location
    world.entity_mut(moving_entity).insert(Location {
        id: destination_entity,
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
