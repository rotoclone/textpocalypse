use bevy_ecs::prelude::*;

use crate::{
    component::{Connection, DescribeAttributes, Description, OpenState, ParseCustomInput, Room},
    move_entity, Direction, SpawnRoom,
};

pub fn set_up_world(world: &mut World) {
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
                aliases: vec!["door".to_string(), "north".to_string(), "n".to_string()],
                description: "A fancy-looking door.".to_string(),
                attribute_describers: vec![
                    Connection::get_attribute_describer(),
                    OpenState::get_attribute_describer(),
                ],
            },
            Connection {
                direction: Direction::North,
                destination: north_room_id,
                other_side: Some(north_room_south_door_id),
            },
            OpenState { is_open: false },
            OpenState::new_custom_input_parser(),
        ))
        .id();
    move_entity(middle_room_north_door_id, middle_room_id, world);

    world.entity_mut(north_room_south_door_id).insert((
        Description {
            name: "fancy door to the south".to_string(),
            room_name: "fancy door".to_string(),
            article: Some("a".to_string()),
            aliases: vec!["door".to_string(), "south".to_string(), "s".to_string()],
            description: "A fancy-looking door.".to_string(),
            attribute_describers: vec![
                Connection::get_attribute_describer(),
                OpenState::get_attribute_describer(),
            ],
        },
        Connection {
            direction: Direction::South,
            destination: middle_room_id,
            other_side: Some(middle_room_north_door_id),
        },
        OpenState { is_open: false },
        OpenState::new_custom_input_parser(),
    ));
    move_entity(north_room_south_door_id, north_room_id, world);

    let middle_room_east_connection_id = world
        .spawn(Connection {
            direction: Direction::East,
            destination: east_room_id,
            other_side: None, // this is a lie but it's fine because this connection has no `OpenState`
        })
        .id();
    move_entity(middle_room_east_connection_id, middle_room_id, world);

    let east_room_west_connection_id = world
        .spawn(Connection {
            direction: Direction::West,
            destination: middle_room_id,
            other_side: None, // this is a lie but it's fine because this connection has no `OpenState`
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
            aliases: vec!["thing".to_string()],
            description: "Some kind of smallish thing.".to_string(),
            attribute_describers: Vec::new(),
        })
        .id();
    move_entity(small_thing_id, middle_room_id, world);

    let large_thing_id = world
        .spawn(Description {
            name: "large thing".to_string(),
            room_name: "large thing".to_string(),
            article: Some("a".to_string()),
            aliases: vec!["thing".to_string()],
            description: "Some kind of largeish thing.".to_string(),
            attribute_describers: Vec::new(),
        })
        .id();
    move_entity(large_thing_id, middle_room_id, world);
}
