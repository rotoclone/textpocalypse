use bevy_ecs::prelude::*;

use crate::{
    color::Color,
    component::{
        Connection, Container, DescribeAttributes, Description, OpenState, ParseCustomInput, Room,
        Volume, Weight,
    },
    game_map::{Coordinates, GameMap, MapIcon},
    move_entity, Direction, SpawnRoom,
};

pub fn set_up_world(world: &mut World) {
    //
    // rooms
    //
    let middle_room_desc = "A nondescript room. You feel uneasy here.";
    let middle_room_icon = MapIcon::new_uniform(Color::Black, Color::White, ['[', ' ', ']']);
    let middle_room_coords = Coordinates {
        x: 0,
        y: 0,
        z: 0,
        parent: None,
    };
    let middle_room_id = world
        .spawn((
            Room {
                name: "The middle room".to_string(),
                description: middle_room_desc.to_string(),
                map_icon: middle_room_icon,
            },
            Container::new_infinite(),
            middle_room_coords.clone(),
            SpawnRoom,
        ))
        .id();
    world
        .resource_mut::<GameMap>()
        .locations
        .insert(middle_room_coords, middle_room_id);

    let north_room_desc =
        "The trim along the floor and ceiling looks to be made of real gold. Fancy.";
    let north_room_icon = MapIcon::new_uniform(Color::Black, Color::DarkYellow, ['[', ' ', ']']);
    let north_room_coords = Coordinates {
        x: 0,
        y: 1,
        z: 0,
        parent: None,
    };
    let north_room_id = world
        .spawn((
            Room {
                name: "The north room".to_string(),
                description: north_room_desc.to_string(),
                map_icon: north_room_icon,
            },
            Container::new_infinite(),
            north_room_coords.clone(),
        ))
        .id();
    world
        .resource_mut::<GameMap>()
        .locations
        .insert(north_room_coords, north_room_id);

    let east_room_desc =
        "This room is very small; you have to hunch over so your head doesn't hit the ceiling.";
    let east_room_icon = MapIcon::new_uniform(Color::Black, Color::White, ['[', ' ', ']']);
    let east_room_coords = Coordinates {
        x: 1,
        y: 0,
        z: 0,
        parent: None,
    };
    let east_room_id = world
        .spawn((
            Room {
                name: "The east room".to_string(),
                description: east_room_desc.to_string(),
                map_icon: east_room_icon,
            },
            Container::new_infinite(),
            east_room_coords.clone(),
        ))
        .id();
    world
        .resource_mut::<GameMap>()
        .locations
        .insert(east_room_coords, east_room_id);

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
