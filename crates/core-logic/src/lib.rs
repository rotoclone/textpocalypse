use std::collections::{HashMap, VecDeque};

mod action;
use action::perform_action;
pub use action::{Action, ActionResult};

const TICK_SECONDS: u8 = 15;
const SECONDS_PER_MINUTE: u8 = 60;
const MINUTES_PER_HOUR: u8 = 60;
const HOURS_PER_DAY: u8 = 24;

trait Entity {
    /// Called when the game world ticks.
    fn on_tick();

    /// Called when this entity is added to the world.
    fn on_activate();

    /// Called when this entity is removed from the world.
    fn on_deactivate();
}

#[derive(PartialEq, Eq, Hash, Clone, Copy, Debug)]
pub struct PlayerId(pub usize);

pub struct World {
    players: HashMap<PlayerId, Player>,
    locations: HashMap<LocationId, Location>,
    spawn_location_id: LocationId,
    time: Time,
    action_queue: VecDeque<Action>,
}

#[derive(Copy, Clone)]
pub struct Time {
    day: usize,
    hour: u8,
    minute: u8,
    second: u8,
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
        //TODO if self.second + seconds_to_add is > 2^8, will this overflow?
        let new_second = (self.second + seconds_to_add) % SECONDS_PER_MINUTE;

        let seconds_remaining_in_minute = SECONDS_PER_MINUTE - self.second;
        let minutes_to_add = (seconds_to_add - seconds_remaining_in_minute) / SECONDS_PER_MINUTE;
        let new_minute = (self.minute + minutes_to_add) % MINUTES_PER_HOUR;

        let minutes_remaining_in_hour = MINUTES_PER_HOUR - self.minute;
        let hours_to_add = (minutes_to_add - minutes_remaining_in_hour) / MINUTES_PER_HOUR;
        let new_hour = (self.hour + hours_to_add) % HOURS_PER_DAY;

        let hours_remaining_in_day = HOURS_PER_DAY - self.hour;
        let days_to_add = (hours_to_add - hours_remaining_in_day) / HOURS_PER_DAY;
        let new_day = self.day + (days_to_add as usize);

        self.second = new_second;
        self.minute = new_minute;
        self.hour = new_hour;
        self.day = new_day;
    }
}

struct Player {
    name: String,
    location_id: LocationId,
}

struct Location {
    name: String,
    description: String,
    connections: HashMap<Direction, LocationId>,
}

#[derive(PartialEq, Eq, Hash, Clone, Copy, Debug)]
struct LocationId(usize);

pub struct LocationDescription {
    pub name: String,
    pub description: String,
    pub exits: Vec<ExitDescription>,
}

impl From<&Location> for LocationDescription {
    fn from(location: &Location) -> Self {
        LocationDescription {
            name: location.name.clone(),
            description: location.description.clone(),
            exits: vec![], //TODO
        }
    }
}

pub struct ExitDescription {
    pub direction: Direction,
    pub description: String,
}

pub struct PlayerState {
    pub location_desc: LocationDescription,
    pub time: Time,
}

#[derive(PartialEq, Eq, Hash, Clone, Copy)]
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

impl Default for World {
    fn default() -> Self {
        Self::new()
    }
}

impl World {
    /// Creates a new, empty world.
    pub fn new() -> World {
        World {
            players: HashMap::new(),
            locations: generate_locations(),
            spawn_location_id: LocationId(0),
            time: Time::new(),
            action_queue: VecDeque::new(),
        }
    }

    /// Adds a player to the world in the default spawn location.
    pub fn add_player(&mut self, id: PlayerId, name: String) {
        self.players.entry(id).or_insert(Player {
            name,
            location_id: self.spawn_location_id,
        });
    }

    /// Gets the state of the world from the provided player's point of view.
    pub fn get_state(&self, player_id: PlayerId) -> PlayerState {
        let player = self.get_player(player_id);

        PlayerState {
            location_desc: self.get_location(player.location_id).into(),
            time: self.time,
        }
    }

    /// Makes the provided player perform the provided action.
    pub fn perform_action(&mut self, player_id: PlayerId, action: Action) -> ActionResult {
        let result = perform_action(action, player_id, self);

        if result.should_tick {
            self.tick();
        }

        result
    }

    /// Performs one game tick.
    fn tick(&mut self) {
        self.time.tick();

        todo!() //TODO
    }

    /// Gets the player with the provided ID. Panics if the player isn't found.
    fn get_player(&self, player_id: PlayerId) -> &Player {
        self.players
            .get(&player_id)
            .unwrap_or_else(|| panic!("Invalid player ID {player_id:?}"))
    }

    /// Gets the player with the provided ID mutably. Panics if the player isn't found.
    fn get_player_mut(&mut self, player_id: PlayerId) -> &mut Player {
        self.players
            .get_mut(&player_id)
            .unwrap_or_else(|| panic!("Invalid player ID {player_id:?}"))
    }

    /// Gets the location with the provided ID. Panics if the location isn't found.
    fn get_location(&self, location_id: LocationId) -> &Location {
        self.locations
            .get(&location_id)
            .unwrap_or_else(|| panic!("Invalid location ID {location_id:?}"))
    }
}

fn generate_locations() -> HashMap<LocationId, Location> {
    let mut locations = HashMap::new();

    locations.insert(
        LocationId(0),
        Location {
            name: "The middle room".to_string(),
            description: "".to_string(),
            connections: [
                (Direction::North, LocationId(1)),
                (Direction::East, LocationId(2)),
            ]
            .into(),
        },
    );
    locations.insert(
        LocationId(1),
        Location {
            name: "The north room".to_string(),
            description: "".to_string(),
            connections: [
                (Direction::South, LocationId(0)),
                (Direction::SouthEast, LocationId(2)),
            ]
            .into(),
        },
    );
    locations.insert(
        LocationId(2),
        Location {
            name: "The east room".to_string(),
            description: "".to_string(),
            connections: [(Direction::West, LocationId(0))].into(),
        },
    );

    locations
}
