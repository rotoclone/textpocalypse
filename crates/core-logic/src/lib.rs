use std::collections::HashMap;

#[derive(PartialEq, Eq, Hash, Clone, Copy, Debug)]
pub struct PlayerId(pub usize);

pub struct World {
    players: HashMap<PlayerId, Player>,
    locations: HashMap<LocationId, Location>,
    spawn_location_id: LocationId,
}

struct Player {
    name: String,
    location_id: LocationId,
}

pub struct Location {
    pub name: String,
    connections: HashMap<Direction, LocationId>,
}

#[derive(PartialEq, Eq, Hash, Clone, Copy, Debug)]
struct LocationId(usize);

pub struct PlayerState<'w> {
    pub location: &'w Location,
}

pub struct ActionResult {
    pub description: String,
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

pub enum Action {
    Move(Direction),
    Look,
}

impl Default for World {
    fn default() -> Self {
        Self::new()
    }
}

impl World {
    pub fn new() -> World {
        World {
            players: HashMap::new(),
            locations: generate_locations(),
            spawn_location_id: LocationId(0),
        }
    }

    pub fn add_player(&mut self, id: PlayerId, name: String) {
        self.players.entry(id).or_insert(Player {
            name,
            location_id: self.spawn_location_id,
        });
    }

    pub fn get_state(&self, player_id: PlayerId) -> PlayerState {
        let player = self.get_player(player_id);

        PlayerState {
            location: self.get_location(player.location_id),
        }
    }

    pub fn apply_action(&mut self, player_id: PlayerId, action: Action) -> ActionResult {
        let player = self.get_player(player_id);

        match action {
            Action::Move(dir) => {
                let current_location = self.get_location(player.location_id);

                if let Some(new_location_id) = current_location.connections.get(&dir) {
                    let new_location = self.get_location(*new_location_id);
                    let result = ActionResult {
                        description: format!("You move to {}.", new_location.name),
                    };

                    self.get_player_mut(player_id).location_id = *new_location_id;

                    result
                } else {
                    ActionResult {
                        description: "You can't move in that direction.".to_string(),
                    }
                }
            }
            Action::Look => ActionResult {
                description: format!("You are at {}.", self.get_state(player_id).location.name),
            },
        }
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
            connections: [(Direction::West, LocationId(0))].into(),
        },
    );

    locations
}
