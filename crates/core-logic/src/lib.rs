use bevy_ecs::prelude::*;
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

mod component;
pub use component::Direction;
pub use component::EntityDescription;
pub use component::ExitDescription;
pub use component::RoomConnectionEntityDescription;
pub use component::RoomDescription;
pub use component::RoomEntityDescription;
pub use component::RoomLivingEntityDescription;
pub use component::RoomObjectDescription;
use component::*;

mod time;
pub use time::Time;

/// A message from the game, such as the description of a location, a message describing the results of an action, etc.
#[derive(Debug)]
pub enum GameMessage {
    Room(RoomDescription),
    Entity(EntityDescription),
    Message(String),
    Error(String),
}

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

impl Game {
    /// Creates a game with a new, empty world
    pub fn new() -> Game {
        let mut world = World::new();
        world.insert_resource(Time::new());
        set_up_world(&mut world);
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
            aliases: HashSet::new(),
            description: "A human-shaped person-type thing.".to_string(),
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

fn find_spawn_room(world: &mut World) -> Entity {
    world
        .query::<(Entity, With<SpawnRoom>)>()
        .get_single(world)
        .expect("A spawn room should exist")
        .0
}

/// Sends a message to the provided entity, if possible. Panics if the entity has no active message channel.
fn send_message(world: &World, entity_id: Entity, message: GameMessage) {
    world
        .get::<MessageChannel>(entity_id)
        .expect("Entity being sent a message should have a message channel")
        .sender
        .send((message, *world.resource::<Time>()))
        .expect("Message receiver should exist");
}

fn set_up_world(world: &mut World) {
    //
    // rooms
    //
    let middle_room_id = world
        .spawn((
            Room::new(
                "The middle room".to_string(),
                "A nondescript room. You feel uneasy here.".to_string(),
            ),
            SpawnRoom,
        ))
        .id();

    let north_room_id = world
        .spawn((Room::new(
            "The north room".to_string(),
            "The trim along the floor and ceiling looks to be made of real gold. Fancy."
                .to_string(),
        ),))
        .id();

    let east_room_id = world
        .spawn((Room::new(
            "The east room".to_string(),
            "This room is very small; you have to hunch over so your head doesn't hit the ceiling."
                .to_string(),
        ),))
        .id();

    let north_room_south_door_id = world.spawn(()).id();

    let middle_room_north_door_id = world
        .spawn((
            Description {
                name: "fancy door to the north".to_string(),
                room_name: "fancy door".to_string(),
                article: Some("a".to_string()),
                aliases: ["door".to_string(), "north".to_string(), "n".to_string()].into(),
                description: "A fancy-looking door.".to_string(),
            },
            Connection {
                direction: Direction::North,
                destination: north_room_id,
                other_side: Some(north_room_south_door_id),
            },
            OpenState { is_open: false },
            OpenState::new_command_parser(),
        ))
        .id();
    move_entity(middle_room_north_door_id, middle_room_id, world);

    world.entity_mut(north_room_south_door_id).insert((
        Description {
            name: "fancy door to the south".to_string(),
            room_name: "fancy door".to_string(),
            article: Some("a".to_string()),
            aliases: ["door".to_string(), "south".to_string(), "s".to_string()].into(),
            description: "A fancy-looking door.".to_string(),
        },
        Connection {
            direction: Direction::South,
            destination: middle_room_id,
            other_side: Some(middle_room_north_door_id),
        },
        OpenState { is_open: false },
        OpenState::new_command_parser(),
    ));
    move_entity(north_room_south_door_id, north_room_id, world);

    let middle_room_east_connection_id = world
        .spawn(Connection {
            direction: Direction::East,
            destination: east_room_id,
            other_side: None, //TODO this is a lie but it's fine because this connection has no OpenState
        })
        .id();
    move_entity(middle_room_east_connection_id, middle_room_id, world);

    let east_room_west_connection_id = world
        .spawn(Connection {
            direction: Direction::West,
            destination: middle_room_id,
            other_side: None, //TODO this is a lie but it's fine because this connection has no OpenState
        })
        .id();
    move_entity(east_room_west_connection_id, east_room_id, world);

    //
    // objects
    //

    let small_thing_id = world
        .spawn(Description {
            name: "small thing".to_string(),
            room_name: "small thing".to_string(),
            article: Some("a".to_string()),
            aliases: ["thing".to_string()].into(),
            description: "Some kind of smallish thing.".to_string(),
        })
        .id();
    move_entity(small_thing_id, middle_room_id, world);

    let large_thing_id = world
        .spawn(Description {
            name: "large thing".to_string(),
            room_name: "large thing".to_string(),
            article: Some("a".to_string()),
            aliases: ["thing".to_string()].into(),
            description: "Some kind of largeish thing.".to_string(),
        })
        .id();
    move_entity(large_thing_id, middle_room_id, world);
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
    world.resource_mut::<Time>().tick();

    //TODO perform queued actions
}

/// Determines whether the provided entity has an active message channel for receiving messages.
pub fn can_receive_messages(world: &World, entity_id: Entity) -> bool {
    world.entity(entity_id).contains::<MessageChannel>()
}

/// Moves an entity to a room.
pub fn move_entity(entity_id: Entity, destination_room_id: Entity, world: &mut World) {
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
