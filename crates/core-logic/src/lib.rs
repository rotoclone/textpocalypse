use flume::{Receiver, Sender};
use hecs::{Entity, World};
use log::debug;
use std::{
    collections::HashSet,
    sync::{Arc, RwLock},
    thread,
};

mod action;
use action::*;

mod command;
use command::*;

mod entity;
pub use entity::EntityDescription;

mod time;
pub use time::Time;

mod player;
use player::*;

mod room;
pub use room::Direction;
pub use room::ExitDescription;
pub use room::RoomDescription;
use room::*;

/// A message from the game, such as the description of a location, a message describing the results of an action, etc.
#[derive(Debug)]
pub enum GameMessage {
    Room(RoomDescription),
    Entity(EntityDescription),
    Message(String),
    Error(String),
}

pub struct Name(String);

pub struct Aliases(HashSet<String>);

pub struct Description {
    short: String,
    long: String,
}

pub struct Location {
    id: Entity,
}

pub struct SpawnRoom;

pub struct MessageChannel {
    sender: Sender<(GameMessage, Time)>,
}

pub struct Game {
    world: Arc<RwLock<World>>,
}

impl Default for Game {
    fn default() -> Self {
        Self::new()
    }
}

impl Game {
    /// Creates a game with a new, empty world
    pub fn new() -> Game {
        let mut world = World::new();
        world.spawn((Time::new(),));
        add_rooms(&mut world);
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
            short: "a person".to_string(),
            long: "A human-shaped person-type thing.".to_string(),
        };
        let message_channel = MessageChannel {
            sender: messages_sender,
        };
        let player_id = world.spawn((Player, Name(name), desc, message_channel));
        let spawn_room_id = find_spawn_room(&world);
        move_entity(&mut world, player_id, spawn_room_id);

        let player_thread_world = Arc::clone(&self.world);

        // set up thread for handling input from the player
        thread::Builder::new()
            .name(format!("command receiver for player {player_id:?}"))
            .spawn(move || loop {
                let command = match commands_receiver.recv() {
                    Ok(c) => c,
                    Err(_) => {
                        debug!("Command sender for player {player_id:?} has been dropped");
                        break;
                    }
                };
                debug!("Received command: {command:?}");
                handle_command(&player_thread_world, command, player_id);
            })
            .unwrap_or_else(|e| {
                panic!("failed to spawn thread to handle input for player {player_id:?}: {e}")
            });

        // send the player an initial message with their location
        let room = world.get::<&Room>(spawn_room_id).unwrap();
        send_message(
            &world,
            player_id,
            GameMessage::Room(RoomDescription::from_room(&room, &world)),
        );

        (commands_sender, messages_receiver)
    }
}

fn find_spawn_room(world: &World) -> Entity {
    world.query::<&SpawnRoom>().iter().next().unwrap().0
}

fn get_time(world: &World) -> Time {
    *world.query::<&Time>().iter().next().unwrap().1 //TODO store time better
}

/// Sends a message to the provided entity, if possible. Panics if the entity has no active message channel.
fn send_message(world: &World, entity_id: Entity, message: GameMessage) {
    world
        .get::<&MessageChannel>(entity_id)
        .unwrap()
        .sender
        .send((message, get_time(world)))
        .unwrap();
}

fn add_rooms(world: &mut World) {
    let middle_room_id = world.spawn((
        Room::new(
            "The middle room".to_string(),
            "A nondescript room. You feel uneasy here.".to_string(),
        ),
        SpawnRoom,
    ));

    let north_room_id = world.spawn((Room::new(
        "The north room".to_string(),
        "The trim along the floor and ceiling looks to be made of real gold. Fancy.".to_string(),
    ),));

    let east_room_id = world.spawn((Room::new(
        "The east room".to_string(),
        "This room is very small; you have to hunch over so your head doesn't hit the ceiling."
            .to_string(),
    ),));

    let mut middle_room = world.get::<&mut Room>(middle_room_id).unwrap();
    middle_room.connect(Direction::North, Connection::new_open(north_room_id));
    middle_room.connect(Direction::East, Connection::new_open(east_room_id));

    let mut north_room = world.get::<&mut Room>(north_room_id).unwrap();
    north_room.connect(Direction::South, Connection::new_open(middle_room_id));
    north_room.connect(Direction::SouthEast, Connection::new_open(east_room_id));
    drop(north_room); // required due to https://github.com/Ralith/hecs/issues/290

    let mut east_room = world.get::<&mut Room>(east_room_id).unwrap();
    east_room.connect(Direction::West, Connection::new_open(middle_room_id));
}

/// Makes the provided entity perform the provided action.
pub fn perform_action(world: &mut World, performing_entity_id: Entity, action: Box<dyn Action>) {
    debug!("Entity {performing_entity_id:?} is performing action {action:?}");
    let result = action.perform(performing_entity_id, world);

    if result.should_tick {
        tick(world);
    }

    for (entity_id, messages) in result.messages {
        for message in messages {
            send_message(world, entity_id, message);
        }
    }
}

/// Performs one game tick.
fn tick(world: &mut World) {
    world.query::<&mut Time>().iter().next().unwrap().1.tick();

    //TODO perform queued actions
}

/// Determines whether the provided entity has an active message channel for receiving messages.
pub fn can_receive_messages(world: &World, entity_id: Entity) -> bool {
    world
        .satisfies::<&MessageChannel>(entity_id)
        .unwrap_or(false)
}

/// Moves an entity to a room.
pub fn move_entity(world: &mut World, entity_id: Entity, destination_room_id: Entity) {
    //TODO handle moving between non-room entities
    // remove from source room, if necessary
    if let Ok(location) = world.get::<&mut Location>(entity_id) {
        let source_room_id = location.id;
        if let Ok(mut source_room) = world.get::<&mut Room>(source_room_id) {
            source_room.entities.remove(&entity_id);
        }
    }

    // add to destination room
    world
        .get::<&mut Room>(destination_room_id)
        .unwrap()
        .entities
        .insert(entity_id);

    // update location
    world
        .insert_one(
            entity_id,
            Location {
                id: destination_room_id,
            },
        )
        .unwrap();
}
