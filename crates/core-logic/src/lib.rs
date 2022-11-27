use flume::{Receiver, Sender};
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
use entity::*;

mod time;
pub use time::Time;

mod world;
use world::*;

mod player;
use player::*;

mod location;
pub use location::Direction;
pub use location::ExitDescription;
pub use location::LocationDescription;
use location::*;

/// A message from the game, such as the description of a location, a message describing the results of an action, etc.
#[derive(Debug)]
pub enum GameMessage {
    Location(LocationDescription),
    Message(String),
    Error(String),
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
        Game {
            world: Arc::new(RwLock::new(World::new())),
        }
    }

    /// Adds a player to the game in the default spawn location.
    pub fn add_player(&self, name: String) -> (Sender<String>, Receiver<(GameMessage, Time)>) {
        // create channels for communication between the player and the world
        let (commands_sender, commands_receiver) = flume::unbounded::<String>();
        let (messages_sender, messages_receiver) = flume::unbounded::<(GameMessage, Time)>();

        // add the player to the world
        let mut world = self.world.write().unwrap();
        let spawn_location_id = world.spawn_location_id;
        let player_id = world.add_entity(Box::new(Player {
            name,
            aliases: HashSet::new(),
            location_id: spawn_location_id,
        }));
        world.register_message_sender(player_id, messages_sender);

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
        let spawn_location = world.get_location(spawn_location_id);
        world.send_message(
            player_id,
            GameMessage::Location(LocationDescription::from_location(spawn_location, &world)),
        );

        (commands_sender, messages_receiver)
    }
}
