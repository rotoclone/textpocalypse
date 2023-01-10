use bevy_ecs::prelude::*;

use crate::{
    color::Color,
    component::{
        Calories, Connection, Container, DescribeAttributes, Description, Edible, Fluid,
        FluidContainer, FluidType, GreetBehavior, KeyId, KeyedLock, OpenState, ParseCustomInput,
        Respawner, Room, SleepState, Vitals, Volume, WanderBehavior, Weight,
    },
    game_map::{Coordinates, GameMap, MapIcon},
    move_entity, ConstrainedValue, Direction, AFTERLIFE_ROOM_COORDINATES,
};

pub fn set_up_world(world: &mut World) -> Coordinates {
    //
    // rooms
    //
    let street_room_name = "Street";
    let horizontal_street_desc =
        "An old street running east-west. The pavement is cracked and the lines are faded.";
    let horizontal_street_icon =
        MapIcon::new_uniform(Color::Black, Color::DarkYellow, ['=', '=', '=']);
    let street_1_id = spawn_room(
        Room {
            name: street_room_name.to_string(),
            description: horizontal_street_desc.to_string(),
            map_icon: horizontal_street_icon.clone(),
        },
        Coordinates {
            x: 0,
            y: 0,
            z: 0,
            parent: None,
        },
        world,
    );

    let street_2_id = spawn_room(
        Room {
            name: street_room_name.to_string(),
            description: horizontal_street_desc.to_string(),
            map_icon: horizontal_street_icon.clone(),
        },
        Coordinates {
            x: 1,
            y: 0,
            z: 0,
            parent: None,
        },
        world,
    );
    connect_open(street_2_id, Direction::West, street_1_id, world);

    let street_3_id = spawn_room(
        Room {
            name: street_room_name.to_string(),
            description: horizontal_street_desc.to_string(),
            map_icon: horizontal_street_icon,
        },
        Coordinates {
            x: 2,
            y: 0,
            z: 0,
            parent: None,
        },
        world,
    );
    connect_open(street_3_id, Direction::West, street_2_id, world);

    let intersection_room_name = "Street Intersection";
    let intersection_desc =
        "A T-intersection. The street stretches away in all four cardinal directions.";
    let intersection_icon = MapIcon::new_uniform(Color::Black, Color::DarkYellow, ['#', ' ', '#']);
    let intersection_id = spawn_room(
        Room {
            name: intersection_room_name.to_string(),
            description: intersection_desc.to_string(),
            map_icon: intersection_icon,
        },
        Coordinates {
            x: 3,
            y: 0,
            z: 0,
            parent: None,
        },
        world,
    );
    connect_open(intersection_id, Direction::West, street_3_id, world);

    let vertical_street_desc =
        "An old street running north-south. The pavement is cracked and the lines are faded.";
    let vertical_street_icon =
        MapIcon::new_uniform(Color::Black, Color::DarkYellow, ['|', ' ', '|']);

    let street_4_id = spawn_room(
        Room {
            name: street_room_name.to_string(),
            description: vertical_street_desc.to_string(),
            map_icon: vertical_street_icon.clone(),
        },
        Coordinates {
            x: 3,
            y: 1,
            z: 0,
            parent: None,
        },
        world,
    );
    connect_open(street_4_id, Direction::South, intersection_id, world);

    let street_5_id = spawn_room(
        Room {
            name: street_room_name.to_string(),
            description: vertical_street_desc.to_string(),
            map_icon: vertical_street_icon.clone(),
        },
        Coordinates {
            x: 3,
            y: 2,
            z: 0,
            parent: None,
        },
        world,
    );
    connect_open(street_5_id, Direction::South, street_4_id, world);

    let street_6_id = spawn_room(
        Room {
            name: street_room_name.to_string(),
            description: vertical_street_desc.to_string(),
            map_icon: vertical_street_icon,
        },
        Coordinates {
            x: 3,
            y: 3,
            z: 0,
            parent: None,
        },
        world,
    );
    connect_open(street_6_id, Direction::South, street_5_id, world);

    let start_building_icon = MapIcon::new_uniform(Color::Black, Color::White, ['[', ':', ']']);
    let start_building_coords = Coordinates {
        x: 1,
        y: 1,
        z: 0,
        parent: None,
    };
    spawn_room(
        Room {
            // name and description aren't used because this is just so the map icon shows up
            name: "".to_string(),
            description: "".to_string(),
            map_icon: start_building_icon,
        },
        start_building_coords.clone(),
        world,
    );

    //
    // npcs
    //

    let npc_id = world
        .spawn((
            Description {
                name: "Some Guy".to_string(),
                room_name: "Some Guy".to_string(),
                plural_name: "Some Guys".to_string(),
                article: None,
                aliases: vec!["guy".to_string()],
                description:
                    "It's just some guy. He looks around, not focusing on anything in particular."
                        .to_string(),
                attribute_describers: vec![SleepState::get_attribute_describer()],
            },
            WanderBehavior {
                move_chance_per_tick: 0.25,
            },
            GreetBehavior {
                greeting: "Hey there!".to_string(),
            },
            Vitals {
                health: ConstrainedValue::new_max(0.0, 25.0),
                satiety: ConstrainedValue::new_max(0.0, 100.0),
                hydration: ConstrainedValue::new_max(0.0, 100.0),
                energy: ConstrainedValue::new_max(0.0, 100.0),
            },
        ))
        .id();
    move_entity(npc_id, street_2_id, world);

    spawn_start_building(world, start_building_coords, street_2_id)
}

pub fn spawn_start_building(
    world: &mut World,
    parent_coords: Coordinates,
    exit_room_id: Entity,
) -> Coordinates {
    //
    // rooms
    //

    let middle_room_desc = "A nondescript room. You feel uneasy here.";
    let middle_room_icon = MapIcon::new_uniform(Color::Black, Color::White, ['[', ' ', ']']);
    let middle_room_coords = Coordinates {
        x: 0,
        y: 0,
        z: 0,
        parent: Some(Box::new(parent_coords.clone())),
    };
    let middle_room_id = spawn_room(
        Room {
            name: "The middle room".to_string(),
            description: middle_room_desc.to_string(),
            map_icon: middle_room_icon,
        },
        middle_room_coords.clone(),
        world,
    );

    let north_room_desc =
        "The trim along the floor and ceiling looks to be made of real gold. Fancy.";
    let north_room_icon = MapIcon::new_uniform(Color::Black, Color::DarkYellow, ['[', ' ', ']']);
    let north_room_id = spawn_room(
        Room {
            name: "The north room".to_string(),
            description: north_room_desc.to_string(),
            map_icon: north_room_icon,
        },
        Coordinates {
            x: 0,
            y: 1,
            z: 0,
            parent: Some(Box::new(parent_coords.clone())),
        },
        world,
    );

    let east_room_desc =
        "This room is very small; you have to hunch over so your head doesn't hit the ceiling.";
    let east_room_icon = MapIcon::new_uniform(Color::Black, Color::White, ['[', ' ', ']']);
    let east_room_id = spawn_room(
        Room {
            name: "The east room".to_string(),
            description: east_room_desc.to_string(),
            map_icon: east_room_icon,
        },
        Coordinates {
            x: 1,
            y: 0,
            z: 0,
            parent: Some(Box::new(parent_coords)),
        },
        world,
    );

    let afterlife_room_desc = "There is nothing.";
    let afterlife_room_icon = MapIcon::new_uniform(Color::Black, Color::White, [' ', ' ', ' ']);
    let afterlife_room_id = spawn_room(
        Room {
            name: "Nowhere".to_string(),
            description: afterlife_room_desc.to_string(),
            map_icon: afterlife_room_icon,
        },
        AFTERLIFE_ROOM_COORDINATES.clone(),
        world,
    );

    let respawner_id = world.spawn(Respawner).id();
    Respawner::register_custom_input_parser(respawner_id, world);
    move_entity(respawner_id, afterlife_room_id, world);

    //
    // connections
    //

    let north_room_south_door_id = world.spawn(()).id();

    let middle_room_north_door_key_id = KeyId(0);

    let middle_room_north_door_id = world
        .spawn((
            Description {
                name: "fancy door to the north".to_string(),
                room_name: "fancy door".to_string(),
                plural_name: "fancy doors".to_string(),
                article: Some("a".to_string()),
                aliases: vec!["door".to_string(), "north".to_string(), "n".to_string()],
                description: "A fancy-looking door.".to_string(),
                attribute_describers: vec![
                    Connection::get_attribute_describer(),
                    OpenState::get_attribute_describer(),
                    KeyedLock::get_attribute_describer(),
                ],
            },
            Connection {
                direction: Direction::North,
                destination: north_room_id,
                other_side: Some(north_room_south_door_id),
            },
            OpenState { is_open: false },
            KeyedLock {
                is_locked: true,
                key_id: Some(middle_room_north_door_key_id.clone()),
            },
        ))
        .id();
    OpenState::register_custom_input_parser(middle_room_north_door_id, world);
    KeyedLock::register_custom_input_parser(middle_room_north_door_id, world);
    move_entity(middle_room_north_door_id, middle_room_id, world);

    world.entity_mut(north_room_south_door_id).insert((
        Description {
            name: "fancy door to the south".to_string(),
            room_name: "fancy door".to_string(),
            plural_name: "fancy doors".to_string(),
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
    ));
    OpenState::register_custom_input_parser(north_room_south_door_id, world);
    move_entity(north_room_south_door_id, north_room_id, world);

    connect_open(middle_room_id, Direction::East, east_room_id, world);
    connect_open(middle_room_id, Direction::South, exit_room_id, world);

    //
    // objects
    //

    let candy_bar_id = world
        .spawn((
            Description {
                name: "candy bar".to_string(),
                room_name: "candy bar".to_string(),
                plural_name: "candy bars".to_string(),
                article: Some("a".to_string()),
                aliases: vec!["candy".to_string(), "bar".to_string()],
                description: "A small candy bar. According to the packaging, it's bursting with chocolatey flavor.".to_string(),
                attribute_describers: vec![
                    Edible::get_attribute_describer(),
                    Calories::get_attribute_describer(),
                    Volume::get_attribute_describer(),
                    Weight::get_attribute_describer(),
                ],
            },
            Edible,
            Calories(300),
            Volume(0.1),
            Weight(0.1),
        ))
        .id();
    move_entity(candy_bar_id, middle_room_id, world);

    let large_thing_id = world
        .spawn((
            Description {
                name: "large thing".to_string(),
                room_name: "large thing".to_string(),
                plural_name: "large things".to_string(),
                article: Some("a".to_string()),
                aliases: vec!["thing".to_string()],
                description: "Some kind of largeish thing.".to_string(),
                attribute_describers: vec![
                    Volume::get_attribute_describer(),
                    Weight::get_attribute_describer(),
                ],
            },
            Volume(5.0),
            Weight(1.0),
        ))
        .id();
    move_entity(large_thing_id, middle_room_id, world);

    let fancy_door_key_id = world
        .spawn((
            Description {
                name: "fancy key".to_string(),
                room_name: "fancy key".to_string(),
                plural_name: "fancy keys".to_string(),
                article: Some("a".to_string()),
                aliases: vec!["key".to_string()],
                description: "A fancy-looking key.".to_string(),
                attribute_describers: vec![
                    Volume::get_attribute_describer(),
                    Weight::get_attribute_describer(),
                ],
            },
            Volume(0.1),
            Weight(0.1),
            middle_room_north_door_key_id,
        ))
        .id();
    move_entity(fancy_door_key_id, east_room_id, world);

    let duffel_bag_id = world
        .spawn((
            Description {
                name: "duffel bag".to_string(),
                room_name: "duffel bag".to_string(),
                plural_name: "duffel bags".to_string(),
                article: Some("a".to_string()),
                aliases: vec!["duffel".to_string(), "bag".to_string()],
                description: "A large duffel bag.".to_string(),
                attribute_describers: vec![
                    Container::get_attribute_describer(),
                    Volume::get_attribute_describer(),
                    Weight::get_attribute_describer(),
                ],
            },
            Volume(5.0),
            Weight(0.5),
            Container::new(Some(Volume(5.0)), None),
        ))
        .id();
    move_entity(duffel_bag_id, middle_room_id, world);

    let lead_weight_1_id = world
        .spawn((
            Description {
                name: "lead weight".to_string(),
                room_name: "lead weight".to_string(),
                plural_name: "lead weights".to_string(),
                article: Some("a".to_string()),
                aliases: vec!["weight".to_string()],
                description: "A very compact, yet very heavy chunk of lead.".to_string(),
                attribute_describers: vec![
                    Volume::get_attribute_describer(),
                    Weight::get_attribute_describer(),
                ],
            },
            Volume(0.5),
            Weight(15.0),
        ))
        .id();
    move_entity(lead_weight_1_id, middle_room_id, world);

    let lead_weight_2_id = world
        .spawn((
            Description {
                name: "lead weight".to_string(),
                room_name: "lead weight".to_string(),
                plural_name: "lead weights".to_string(),
                article: Some("a".to_string()),
                aliases: vec!["weight".to_string()],
                description: "A very compact, yet very heavy chunk of lead.".to_string(),
                attribute_describers: vec![
                    Volume::get_attribute_describer(),
                    Weight::get_attribute_describer(),
                ],
            },
            Volume(0.5),
            Weight(15.0),
        ))
        .id();
    move_entity(lead_weight_2_id, middle_room_id, world);

    let water_jug_id = world
        .spawn((
            Description {
                name: "water jug".to_string(),
                room_name: "water jug".to_string(),
                plural_name: "water jugs".to_string(),
                article: Some("a".to_string()),
                aliases: vec!["jug".to_string()],
                description: "A large jug made for holding water.".to_string(),
                attribute_describers: vec![
                    Volume::get_attribute_describer(),
                    Weight::get_attribute_describer(),
                    FluidContainer::get_attribute_describer(),
                ],
            },
            Volume(2.0),
            Weight(1.0),
            FluidContainer {
                contents: Fluid {
                    contents: [(FluidType::Water, Volume(1.0))].into(),
                },
                volume: Some(Volume(2.0)),
            },
        ))
        .id();
    move_entity(water_jug_id, middle_room_id, world);

    middle_room_coords
}

/// Spawns the provided room at the provided coordinates.
fn spawn_room(room: Room, coords: Coordinates, world: &mut World) -> Entity {
    let room_id = world
        .spawn((room, Container::new_infinite(), coords.clone()))
        .id();
    world
        .resource_mut::<GameMap>()
        .locations
        .insert(coords, room_id);

    room_id
}

/// Connects the provided entities with open connections.
fn connect_open(room_1: Entity, dir: Direction, room_2: Entity, world: &mut World) {
    let connection_1 = world
        .spawn(Connection {
            direction: dir,
            destination: room_2,
            other_side: None, // this is a lie but it's fine because this connection has no `OpenState`
        })
        .id();
    move_entity(connection_1, room_1, world);

    let connection_2 = world
        .spawn(Connection {
            direction: dir.opposite(),
            destination: room_1,
            other_side: None, // this is a lie but it's fine because this connection has no `OpenState`
        })
        .id();
    move_entity(connection_2, room_2, world);
}
