use bevy_ecs::prelude::*;
use flume::{Receiver, Sender};
use input_parser::InputParser;
use log::debug;
use std::{
    collections::{HashMap, HashSet},
    sync::{Arc, RwLock},
    thread::{self},
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

mod constrained_value;
pub use constrained_value::ConstrainedValue;

mod value_change;
pub use value_change::ValueType;

pub const AFTERLIFE_ROOM_COORDINATES: Coordinates = Coordinates {
    x: 0,
    y: 0,
    z: i64::MAX,
    parent: None,
};

/// Coordinates of the spawn room.
#[derive(Resource)]
pub struct SpawnRoom(Coordinates);

/// Coordinates of the afterlife room.
#[derive(Resource)]
pub struct AfterlifeRoom(Coordinates);

/// Mapping of player IDs to entities.
#[derive(Resource)]
pub struct PlayerIdMapping(HashMap<PlayerId, Entity>);

#[derive(StageLabel)]
pub struct TickStage;

/// Entities that should have in-progress actions interrupted.
#[derive(Resource)]
pub struct InterruptedEntities(pub HashSet<Entity>);

pub struct Game {
    /// The game world.
    world: Arc<RwLock<World>>,
    /// The schedule to run on each tick.
    /// TODO remove?
    tick_schedule: Arc<RwLock<Schedule>>,
    /// The ID to assign to the next added player.
    next_player_id: PlayerId,
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
                Box::new(PutParser),
                Box::new(VitalsParser),
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
        world.insert_resource(InterruptedEntities(HashSet::new()));
        world.insert_resource(SpawnRoom(Coordinates {
            x: 0,
            y: 0,
            z: 0,
            parent: None,
        }));
        world.insert_resource(AfterlifeRoom(AFTERLIFE_ROOM_COORDINATES));
        world.insert_resource(PlayerIdMapping(HashMap::new()));
        set_up_world(&mut world);
        register_component_handlers(&mut world);

        let tick_schedule = Schedule::default().with_stage(TickStage, SystemStage::parallel());

        NotificationHandlers::add_handler(look_after_move, &mut world);
        Game {
            world: Arc::new(RwLock::new(world)),
            tick_schedule: Arc::new(RwLock::new(tick_schedule)),
            next_player_id: PlayerId(0),
        }
    }

    /// Adds a player to the game in the default spawn location.
    pub fn add_player(&mut self, name: String) -> (Sender<String>, Receiver<(GameMessage, Time)>) {
        // create channels for communication between the player and the world
        let (commands_sender, commands_receiver) = flume::unbounded::<String>();
        let (messages_sender, messages_receiver) = flume::unbounded::<(GameMessage, Time)>();

        let player_id = self.get_player_id();

        // add the player to the world
        let mut world = self.world.write().unwrap();
        let spawn_room_id = find_spawn_room(&world);
        let player = Player {
            id: player_id,
            sender: messages_sender,
        };
        let player_entity = spawn_player(name, player, spawn_room_id, &mut world);
        self.spawn_command_thread(player_id, commands_receiver);

        // add stuff to the player's inventory
        let medium_thing_id = world
            .spawn((
                Description {
                    name: "medium thing".to_string(),
                    room_name: "medium thing".to_string(),
                    plural_name: "medium things".to_string(),
                    article: Some("a".to_string()),
                    aliases: vec!["thing".to_string()],
                    description: "Some kind of medium-sized thing.".to_string(),
                    attribute_describers: vec![
                        Volume::get_attribute_describer(),
                        Weight::get_attribute_describer(),
                    ],
                },
                Volume(0.25),
                Weight(0.5),
            ))
            .id();
        move_entity(medium_thing_id, player_entity, &mut world);

        let heavy_thing_id = world
            .spawn((
                Description {
                    name: "heavy thing".to_string(),
                    room_name: "heavy thing".to_string(),
                    plural_name: "heavy things".to_string(),
                    article: Some("a".to_string()),
                    aliases: vec!["thing".to_string()],
                    description: "Some kind of heavy thing.".to_string(),
                    attribute_describers: vec![
                        Volume::get_attribute_describer(),
                        Weight::get_attribute_describer(),
                    ],
                },
                Volume(0.5),
                Weight(15.0),
            ))
            .id();
        move_entity(heavy_thing_id, player_entity, &mut world);

        // send the player an initial message with their location
        send_current_location_message(player_entity, &world);

        (commands_sender, messages_receiver)
    }

    /// Gets the next player ID to use.
    fn get_player_id(&mut self) -> PlayerId {
        let id = self.next_player_id;
        self.next_player_id = self.next_player_id.increment();

        id
    }

    /// Sets up a thread for handling input from a player.
    fn spawn_command_thread(&self, player_id: PlayerId, commands_receiver: Receiver<String>) {
        let player_thread_world = Arc::clone(&self.world);
        let player_thread_tick_schedule = Arc::clone(&self.tick_schedule);

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
                let read_world = player_thread_world.read().unwrap();
                if let Some(player_entity) = find_entity_for_player(player_id, &read_world) {
                    drop(read_world);
                    handle_input(
                        &player_thread_tick_schedule,
                        &player_thread_world,
                        input,
                        player_entity,
                    );
                } else {
                    debug!("Player with ID {player_id:?} has no corresponding entity");
                    break;
                }
            })
            .unwrap_or_else(|e| {
                panic!("failed to spawn thread to handle input for player {player_id:?}: {e}")
            });
    }
}

/// Sends a message to an entity with their current location.
fn send_current_location_message(entity: Entity, world: &World) {
    let room_id = world
        .get::<Location>(entity)
        .expect("Entity should have a location")
        .id;
    let room = world
        .get::<Room>(room_id)
        .expect("Entity's location should be a room");
    let container = world
        .get::<Container>(room_id)
        .expect("Entity's location should be a container");
    let coords = world
        .get::<Coordinates>(room_id)
        .expect("Entity's location should have coordinates");
    send_message(
        world,
        entity,
        GameMessage::Room(RoomDescription::from_room(
            room, container, coords, entity, world,
        )),
    );
}

/// Finds the entity corresponding to the provided player ID, if one exists.
fn find_entity_for_player(player_id: PlayerId, world: &World) -> Option<Entity> {
    world
        .resource::<PlayerIdMapping>()
        .0
        .get(&player_id)
        .copied()
}

/// Spawns a new player.
fn spawn_player(name: String, player: Player, spawn_room: Entity, world: &mut World) -> Entity {
    let player_id = player.id;
    let desc = Description {
        name: name.clone(),
        room_name: name,
        plural_name: "people".to_string(),
        article: None,
        aliases: Vec::new(),
        description: "A human-shaped person-type thing.".to_string(),
        attribute_describers: Vec::new(),
    };
    let vitals = Vitals::new();
    let player_entity = world
        .spawn((
            player,
            Container::new(Some(Volume(10.0)), Some(Weight(25.0))),
            desc,
            vitals,
        ))
        .id();
    move_entity(player_entity, spawn_room, world);

    world
        .resource_mut::<PlayerIdMapping>()
        .0
        .insert(player_id, player_entity);

    player_entity
}

/// Finds the ID of the spawn room in the provided world.
fn find_spawn_room(world: &World) -> Entity {
    let spawn_room_coords = &world.resource::<SpawnRoom>().0;
    *world
        .resource::<GameMap>()
        .locations
        .get(spawn_room_coords)
        .expect("The spawn room should exist")
}

/// Finds the ID of the afterlife room in the provided world.
fn find_afterlife_room(world: &World) -> Entity {
    let spawn_room_coords = &world.resource::<AfterlifeRoom>().0;
    *world
        .resource::<GameMap>()
        .locations
        .get(spawn_room_coords)
        .expect("The afterlife room should exist")
}

/// Determines whether the provided entity has an active message channel for receiving messages.
fn can_receive_messages(world: &World, entity_id: Entity) -> bool {
    world.entity(entity_id).contains::<Player>()
}

/// Sends a message to the provided entity, if possible. Panics if the entity's message receiver has been dropped.
fn send_message(world: &World, entity_id: Entity, message: GameMessage) {
    if let Some(player) = world.get::<Player>(entity_id) {
        let time = world.resource::<Time>().clone();
        send_message_to_player(player, message, time);
    }
}

/// Sends a message to the provided player. Panics if the channel's message receiver has been dropped.
fn send_message_to_player(player: &Player, message: GameMessage, time: Time) {
    player
        .sender
        .send((message, time))
        .expect("Message receiver should exist");
}

/// Handles input from an entity.
fn handle_input(
    tick_schedule: &Arc<RwLock<Schedule>>,
    world: &Arc<RwLock<World>>,
    input: String,
    entity: Entity,
) {
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
            try_perform_queued_actions(&mut tick_schedule.write().unwrap(), &mut write_world);
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
                let location_name = match &t {
                    CommandTarget::Named(target_name) => {
                        if target_name.location_chain.is_empty() {
                            "here".to_string()
                        } else {
                            format!("in '{}'", target_name.location_chain.join(" in "))
                        }
                    }
                    _ => "here".to_string(),
                };
                format!("There is no '{t}' {location_name}.")
            }
            CommandParseError::Other(e) => e,
        },
    };

    send_message(world, entity, GameMessage::Error(message));
}

/// A notification that a tick is occurring.
#[derive(Debug)]
pub struct TickNotification;

impl NotificationType for TickNotification {}

/// Performs one game tick.
fn tick(tick_schedule: &mut Schedule, world: &mut World) {
    world.resource_mut::<Time>().tick();

    tick_schedule.run(world);

    Notification {
        notification_type: TickNotification,
        contents: &(),
    }
    .send(world);
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

/// Sets an entity's actions to be interrupted.
fn interrupt_entity(entity: Entity, world: &mut World) {
    world.resource_mut::<InterruptedEntities>().0.insert(entity);
}

/// Kills an entity.
fn kill_entity(entity: Entity, world: &mut World) {
    //TODO don't kill it if it's already dead
    send_message(
        world,
        entity,
        GameMessage::Message(
            "You crumple to the ground and gasp your last breath.".to_string(),
            MessageDelay::Long,
        ),
    );

    let mut entity_ref = world.entity_mut(entity);
    if let Some(desc) = entity_ref.remove::<Description>() {
        let mut aliases = desc.aliases;
        aliases.push("dead body".to_string());
        aliases.push("body".to_string());

        let mut attribute_describers = desc.attribute_describers;
        attribute_describers.push(Container::get_attribute_describer());

        let new_desc = Description {
            name: format!("dead body of {}", desc.name),
            room_name: format!("dead body of {}", desc.room_name),
            plural_name: format!("dead bodies of {}", desc.room_name),
            article: Some("the".to_string()),
            aliases,
            description: desc.description,
            attribute_describers,
        };

        entity_ref.insert(new_desc);
    }

    if let Some(player) = world.entity_mut(entity).remove::<Player>() {
        let name = world
            .get::<Description>(entity)
            .map(|d| d.name.clone())
            .unwrap_or_else(|| "".to_string());
        let new_entity = spawn_player(name, player, find_afterlife_room(world), world);

        // players shouldn't have vitals until they actually respawn
        world.entity_mut(new_entity).remove::<Vitals>();

        send_current_location_message(new_entity, world);
    }
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

/// Determines the total weight of an entity.
fn get_weight(entity: Entity, world: &World) -> Weight {
    get_weight_recursive(entity, world, &mut vec![entity])
}

fn get_weight_recursive(
    entity: Entity,
    world: &World,
    contained_entities: &mut Vec<Entity>,
) -> Weight {
    let mut weight = world.get::<Weight>(entity).cloned().unwrap_or(Weight(0.0));

    if let Some(container) = world.get::<Container>(entity) {
        let contained_weight = container
            .entities
            .iter()
            .map(|e| {
                if contained_entities.contains(e) {
                    panic!("{entity:?} contains itself")
                }
                contained_entities.push(*e);
                get_weight_recursive(*e, world, contained_entities)
            })
            .sum::<Weight>();

        weight += contained_weight;
    }

    weight
}
