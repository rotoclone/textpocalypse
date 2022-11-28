use std::collections::{HashMap, HashSet};

use flume::Sender;
use log::debug;

use crate::{Action, Connection, Direction, Entity, GameMessage, Location, Time};

#[derive(PartialEq, Eq, Hash, Clone, Copy, Debug)]
pub struct EntityId(usize);

#[derive(PartialEq, Eq, Hash, Clone, Copy, Debug)]
pub struct LocationId(usize);

pub struct World {
    entities: HashMap<EntityId, Box<dyn Entity>>,
    next_entity_id: usize,
    message_senders: HashMap<EntityId, Sender<(GameMessage, Time)>>,
    locations: HashMap<LocationId, Location>,
    next_location_id: usize,
    pub spawn_location_id: LocationId,
    time: Time,
}

impl Default for World {
    fn default() -> Self {
        Self::new()
    }
}

impl World {
    /// Creates a new, empty world.
    pub fn new() -> World {
        let mut world = World {
            entities: HashMap::new(),
            next_entity_id: 0,
            message_senders: HashMap::new(),
            locations: HashMap::new(),
            next_location_id: 0,
            spawn_location_id: LocationId(0),
            time: Time::new(),
        };

        add_locations(&mut world);

        world
    }

    /// Makes the provided entity perform the provided action.
    pub fn perform_action(&mut self, performing_entity_id: EntityId, action: Box<dyn Action>) {
        debug!("Entity {performing_entity_id:?} is performing action {action:?}");
        let result = action.perform(performing_entity_id, self);

        if result.should_tick {
            self.tick();
        }

        for (entity_id, messages) in result.messages {
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

    /// Adds an entity to the world. Returns the ID of the entity.
    pub fn add_entity(&mut self, entity: Box<dyn Entity>) -> EntityId {
        let id = EntityId(self.next_entity_id);
        self.next_entity_id += 1;

        self.get_location_mut(entity.get_location_id())
            .entities
            .insert(id);
        self.entities.insert(id, entity);

        id
    }

    /// Gets the entity with the provided ID. Panics if the entity isn't found.
    pub fn get_entity(&self, entity_id: EntityId) -> &dyn Entity {
        self.entities
            .get(&entity_id)
            .unwrap_or_else(|| panic!("Invalid entity ID {entity_id:?}"))
            .as_ref()
    }

    /// Gets the entity with the provided ID mutably. Panics if the entity isn't found.
    pub fn get_entity_mut(&mut self, entity_id: EntityId) -> &mut dyn Entity {
        self.entities
            .get_mut(&entity_id)
            .unwrap_or_else(|| panic!("Invalid entity ID {entity_id:?}"))
            .as_mut()
    }

    /// Finds the entity with the provided name, if it exists. The search is limited to entities in the location with the provided ID.
    pub fn find_entity_by_name(&self, name: &str, location_id: LocationId) -> Option<EntityId> {
        for entity_id in &self.get_location(location_id).entities {
            let entity = self.get_entity(*entity_id);
            if entity.get_name().eq_ignore_ascii_case(name) || entity.get_aliases().contains(name) {
                return Some(*entity_id);
            }
        }

        None
    }

    /// Moves an entity from the source location to the destination location.
    pub fn move_entity(
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

    /// Adds a location to the world. Returns the ID of the location.
    pub fn add_location(&mut self, location: Location) -> LocationId {
        let id = LocationId(self.next_location_id);
        self.next_location_id += 1;

        self.locations.insert(id, location);

        id
    }

    /// Gets the location with the provided ID. Panics if the location isn't found.
    pub fn get_location(&self, location_id: LocationId) -> &Location {
        self.locations
            .get(&location_id)
            .unwrap_or_else(|| panic!("Invalid location ID {location_id:?}"))
    }

    /// Gets the location with the provided ID mutably. Panics if the location isn't found.
    pub fn get_location_mut(&mut self, location_id: LocationId) -> &mut Location {
        self.locations
            .get_mut(&location_id)
            .unwrap_or_else(|| panic!("Invalid location ID {location_id:?}"))
    }

    /// Registers a message sender for an entity, replacing any existing one for that entity.
    pub fn register_message_sender(
        &mut self,
        entity_id: EntityId,
        message_sender: Sender<(GameMessage, Time)>,
    ) {
        self.message_senders.insert(entity_id, message_sender);
    }

    /// Determines whether the entity with the provided ID can receive messages or not.
    pub fn can_receive_messages(&self, entity_id: EntityId) -> bool {
        self.message_senders.contains_key(&entity_id)
    }

    /// Sends a message to an entity. Panics if the entity's message receiver is closed.
    pub fn send_message(&self, entity_id: EntityId, message: GameMessage) {
        if let Some(sender) = self.message_senders.get(&entity_id) {
            sender.send((message, self.time)).unwrap();
        }
    }
}

fn add_locations(world: &mut World) {
    let middle_room_id = world.add_location(Location::new(
        "The middle room".to_string(),
        "A nondescript room. You feel uneasy here.".to_string(),
    ));

    let north_room_id = world.add_location(Location::new(
        "The north room".to_string(),
        "The trim along the floor and ceiling looks to be made of real gold. Fancy.".to_string(),
    ));

    let east_room_id = world.add_location(Location::new(
        "The east room".to_string(),
        "This room is very small; you have to hunch over so your head doesn't hit the ceiling."
            .to_string(),
    ));

    let middle_room = world.get_location_mut(middle_room_id);
    middle_room.connect(Direction::North, Connection::new_open(north_room_id));
    middle_room.connect(Direction::East, Connection::new_open(east_room_id));

    let north_room = world.get_location_mut(north_room_id);
    north_room.connect(Direction::South, Connection::new_open(middle_room_id));
    north_room.connect(Direction::SouthEast, Connection::new_open(east_room_id));

    let east_room = world.get_location_mut(east_room_id);
    east_room.connect(Direction::West, Connection::new_open(middle_room_id));
}
