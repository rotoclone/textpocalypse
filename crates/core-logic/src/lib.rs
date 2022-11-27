use flume::{Receiver, Sender};
use log::debug;
use std::{
    collections::{HashMap, HashSet},
    sync::{Arc, RwLock},
    thread,
};

mod action;
use action::*;
pub use action::{Action, ActionResult};

mod command;
use command::*;

mod entity;
use entity::*;

const TICK_SECONDS: u8 = 15;
const SECONDS_PER_MINUTE: u8 = 60;
const MINUTES_PER_HOUR: u8 = 60;
const HOURS_PER_DAY: u8 = 24;

/// A message from the game, such as the description a location, a message describing the results of an action, etc.
#[derive(Debug)]
pub enum GameMessage {
    Location(LocationDescription),
    Message(String),
    Error(String),
}

#[derive(PartialEq, Eq, Hash, Clone, Copy, Debug)]
pub struct EntityId(pub usize);

pub struct World {
    entities: HashMap<EntityId, Box<dyn Entity + Send + Sync>>,
    message_senders: HashMap<EntityId, Sender<(GameMessage, Time)>>,
    locations: HashMap<LocationId, Location>,
    spawn_location_id: LocationId,
    time: Time,
}

#[derive(Copy, Clone, Debug)]
pub struct Time {
    pub day: usize,
    pub hour: u8,
    pub minute: u8,
    pub second: u8,
}

impl Time {
    fn new() -> Time {
        Time {
            day: 1,
            hour: 7,
            minute: 0,
            second: 0,
        }
    }

    fn tick(&mut self) {
        self.add_seconds(TICK_SECONDS);
    }

    fn add_seconds(&mut self, seconds_to_add: u8) {
        debug!("Adding {seconds_to_add} seconds to current time {self:?}");

        //TODO if self.second + seconds_to_add is > 2^8, will this overflow?
        let seconds_remaining_in_minute = SECONDS_PER_MINUTE - self.second;
        let new_second = (self.second + seconds_to_add) % SECONDS_PER_MINUTE;
        self.second = new_second;

        debug!("Seconds remaining in minute: {seconds_remaining_in_minute}");

        if seconds_to_add < seconds_remaining_in_minute {
            debug!("Did not roll over to new minute; new time: {self:?}");
            return;
        }

        let minutes_remaining_in_hour = MINUTES_PER_HOUR - self.minute;
        let minutes_to_add =
            1 + ((seconds_to_add - seconds_remaining_in_minute) / SECONDS_PER_MINUTE);
        let new_minute = (self.minute + minutes_to_add) % MINUTES_PER_HOUR;
        self.minute = new_minute;

        if minutes_to_add < minutes_remaining_in_hour {
            debug!("Did not roll over to new hour; new time: {self:?}");
            return;
        }

        let hours_remaining_in_day = HOURS_PER_DAY - self.hour;
        let hours_to_add = 1 + ((minutes_to_add - minutes_remaining_in_hour) / MINUTES_PER_HOUR);
        let new_hour = (self.hour + hours_to_add) % HOURS_PER_DAY;
        self.hour = new_hour;

        if hours_to_add < hours_remaining_in_day {
            debug!("Did not roll over to new day; new time: {self:?}");
            return;
        }

        let days_to_add = 1 + ((hours_to_add - hours_remaining_in_day) / HOURS_PER_DAY);
        let new_day = self.day + (days_to_add as usize);
        self.day = new_day;

        debug!("New time: {self:?}");
    }
}

struct Player {
    name: String,
    location_id: LocationId,
}

impl Entity for Player {
    fn get_name(&self) -> &str {
        &self.name
    }

    fn get_location_id(&self) -> LocationId {
        self.location_id
    }

    fn set_location_id(&mut self, location_id: LocationId) {
        self.location_id = location_id;
    }

    fn on_tick(&mut self) {
        //TODO increase hunger and stuff
    }

    fn on_command(
        &self,
        entity_id: EntityId,
        command: String,
        world: &World,
    ) -> Option<Box<dyn Action>> {
        None
    }
}

struct Location {
    name: String,
    description: String,
    entities: HashSet<EntityId>,
    connections: HashMap<Direction, Connection>,
}

struct Connection {
    location_id: LocationId,
    via_entity_id: Option<EntityId>,
}

impl Connection {
    /// Creates a new open connection to the provided location
    fn new_open(location_id: LocationId) -> Connection {
        Connection {
            location_id,
            via_entity_id: None,
        }
    }

    /// Creates a new connection to the provided location that requires passing through the provided entity
    fn new_via_entity(location_id: LocationId, via_entity_id: EntityId) -> Connection {
        Connection {
            location_id,
            via_entity_id: Some(via_entity_id),
        }
    }
}

#[derive(PartialEq, Eq, Hash, Clone, Copy, Debug)]
pub struct LocationId(usize);

#[derive(Debug)]
pub struct LocationDescription {
    pub name: String,
    pub description: String,
    pub exits: Vec<ExitDescription>,
}

impl LocationDescription {
    /// Creates a `LocationDescription` for the provided location
    fn from_location(location: &Location, world: &World) -> LocationDescription {
        LocationDescription {
            name: location.name.clone(),
            description: location.description.clone(),
            exits: ExitDescription::from_location(location, world),
        }
    }
}

#[derive(Debug)]
pub struct ExitDescription {
    pub direction: Direction,
    pub description: String,
}

impl ExitDescription {
    /// Creates a list of exit descriptions for the provided location
    fn from_location(location: &Location, world: &World) -> Vec<ExitDescription> {
        location
            .connections
            .iter()
            .map(|(dir, connection)| {
                let location = world.get_location(connection.location_id);
                ExitDescription {
                    direction: *dir,
                    description: location.name.clone(),
                }
            })
            .collect()
    }
}

pub struct PlayerState {
    pub location_desc: LocationDescription,
    pub time: Time,
}

#[derive(PartialEq, Eq, Hash, Clone, Copy, Debug)]
pub enum Direction {
    North,
    NorthEast,
    East,
    SouthEast,
    South,
    SouthWest,
    West,
    NorthWest,
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
    pub fn add_player(
        &self,
        id: EntityId,
        name: String,
    ) -> (Sender<String>, Receiver<(GameMessage, Time)>) {
        // create channels for communication between the player and the world
        let (commands_sender, commands_receiver) = flume::unbounded::<String>();
        let (messages_sender, messages_receiver) = flume::unbounded::<(GameMessage, Time)>();

        // add the player to the world
        let mut world = self.world.write().unwrap();
        let spawn_location_id = world.spawn_location_id;
        world.entities.entry(id).or_insert_with(|| {
            Box::new(Player {
                name,
                location_id: spawn_location_id,
            })
        });
        world.message_senders.entry(id).or_insert(messages_sender);

        let player_thread_world = Arc::clone(&self.world);

        // set up thread for handling input from the player
        thread::Builder::new()
            .name(format!("command receiver for player {id:?}"))
            .spawn(move || loop {
                let command = match commands_receiver.recv() {
                    Ok(c) => c,
                    Err(_) => {
                        debug!("Command sender for player {id:?} has been dropped");
                        break;
                    }
                };
                debug!("Received command: {command:?}");
                handle_command(&player_thread_world, command, id);
            })
            .unwrap_or_else(|e| {
                panic!("failed to spawn thread to handle input for player {id:?}: {e}")
            });

        // send the player an initial message with their location
        let spawn_location = world.get_location(spawn_location_id);
        world.send_message(
            id,
            GameMessage::Location(LocationDescription::from_location(spawn_location, &world)),
        );

        (commands_sender, messages_receiver)
    }
}

impl Default for World {
    fn default() -> Self {
        Self::new()
    }
}

impl World {
    /// Creates a new, empty world.
    fn new() -> World {
        World {
            entities: HashMap::new(),
            message_senders: HashMap::new(),
            locations: generate_locations(),
            spawn_location_id: LocationId(0),
            time: Time::new(),
        }
    }

    /// Makes the provided entity perform the provided action.
    pub fn perform_action(&mut self, entity_id: EntityId, action: Box<dyn Action>) {
        debug!("Entity {entity_id:?} is performing an action");
        let result = action.perform(entity_id, self);

        if result.should_tick {
            self.tick();
        }

        for (player_id, messages) in result.messages {
            for message in messages {
                self.send_message(entity_id, message);
            }
        }
    }

    /// Performs one game tick.
    fn tick(&mut self) {
        self.time.tick();

        //TODO perform queued actions
    }

    /// Gets the entity with the provided ID. Panics if the entity isn't found.
    fn get_entity(&self, entity_id: EntityId) -> &(dyn Entity + Send + Sync) {
        self.entities
            .get(&entity_id)
            .unwrap_or_else(|| panic!("Invalid entity ID {entity_id:?}"))
            .as_ref()
    }

    /// Gets the entity with the provided ID mutably. Panics if the entity isn't found.
    fn get_entity_mut(&mut self, entity_id: EntityId) -> &mut (dyn Entity + Send + Sync) {
        self.entities
            .get_mut(&entity_id)
            .unwrap_or_else(|| panic!("Invalid entity ID {entity_id:?}"))
            .as_mut()
    }

    /// Moves an entity from the source location to the destination location.
    fn move_entity(
        &mut self,
        entity_id: EntityId,
        source_location_id: LocationId,
        destination_location_id: LocationId,
    ) {
        let entity = self.get_entity_mut(entity_id);
        entity.set_location_id(destination_location_id);

        let source_location = self.get_location_mut(source_location_id);
        source_location.entities.remove(&entity_id);

        let destination_location = self.get_location_mut(destination_location_id);
        destination_location.entities.insert(entity_id);
    }

    /// Gets the location with the provided ID. Panics if the location isn't found.
    fn get_location(&self, location_id: LocationId) -> &Location {
        self.locations
            .get(&location_id)
            .unwrap_or_else(|| panic!("Invalid location ID {location_id:?}"))
    }

    /// Gets the location with the provided ID mutably. Panics if the location isn't found.
    fn get_location_mut(&mut self, location_id: LocationId) -> &mut Location {
        self.locations
            .get_mut(&location_id)
            .unwrap_or_else(|| panic!("Invalid location ID {location_id:?}"))
    }

    /// Determines whether the entity with the provided ID can receive messages or not.
    fn can_receive_messages(&self, entity_id: EntityId) -> bool {
        self.message_senders.contains_key(&entity_id)
    }

    /// Sends a message to an entity. Panics if the entity's message receiver is closed.
    fn send_message(&self, entity_id: EntityId, message: GameMessage) {
        if let Some(sender) = self.message_senders.get(&entity_id) {
            sender.send((message, self.time)).unwrap();
        }
    }
}

fn generate_locations() -> HashMap<LocationId, Location> {
    let mut locations = HashMap::new();

    locations.insert(
        LocationId(0),
        Location {
            name: "The middle room".to_string(),
            description: "A nondescript room. You feel uneasy here.".to_string(),
            entities: HashSet::new(),
            connections: [
                (Direction::North, Connection::new_open(LocationId(1))),
                (Direction::East, Connection::new_open(LocationId(2))),
            ]
            .into(),
        },
    );
    locations.insert(
        LocationId(1),
        Location {
            name: "The north room".to_string(),
            description:
                "The trim along the floor and ceiling looks to be made of real gold. Fancy."
                    .to_string(),
            entities: HashSet::new(),
            connections: [
                (Direction::South, Connection::new_open(LocationId(0))),
                (Direction::SouthEast, Connection::new_open(LocationId(2))),
            ]
            .into(),
        },
    );
    locations.insert(
        LocationId(2),
        Location {
            name: "The east room".to_string(),
            description: "This room is very small; you have to hunch over so your head doesn't hit the ceiling.".to_string(),
            entities: HashSet::new(),
            connections: [(Direction::West, Connection::new_open(LocationId(0)))].into(),
        },
    );

    locations
}
