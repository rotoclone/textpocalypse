use std::{array, sync::LazyLock};

use bevy_ecs::prelude::*;

use crate::{
    component::{Location, Room},
    game_map::{Coordinates, GameMap},
    Color, MapIcon,
};

static BLANK_ICON: LazyLock<MapIcon> =
    LazyLock::new(|| MapIcon::new_uniform(Color::Black, Color::DarkGray, ['.', '.']));
static PLAYER_MAP_ICON: LazyLock<MapIcon> =
    LazyLock::new(|| MapIcon::new_uniform(Color::Black, Color::Cyan, ['(', ')']));

/// A collection of tiles around an entity.
/// `S` is the length and width of the map, in tiles.
#[derive(Debug, Clone)]
pub struct MapDescription<const S: usize> {
    /// The tiles in the map. Formatted as an array of rows.
    pub tiles: [[MapIcon; S]; S],
}

impl<const S: usize> MapDescription<S> {
    /// Creates a map centered on the location of the provided entity.
    pub fn for_entity(
        pov_entity: Entity,
        center_coords: &Coordinates,
        world: &World,
    ) -> MapDescription<S> {
        let pov_coords = find_coordinates_of_entity(pov_entity, world);
        let center_index = S / 2;

        let tiles = array::from_fn(|row_index| {
            array::from_fn(|col_index| {
                let x = center_coords.x + (col_index as i64 - center_index as i64);
                let y = center_coords.y - (row_index as i64 - center_index as i64);
                let z = center_coords.z;
                let parent = center_coords.parent.clone();

                let current_coords = Coordinates { x, y, z, parent };

                if current_coords == *pov_coords {
                    PLAYER_MAP_ICON.clone()
                } else {
                    icon_for_coords(&current_coords, world)
                }
            })
        });

        MapDescription { tiles }
    }
}

/// Finds the coordinates of the location the provided entity is in.
///
/// Panics if the entity does not have a location with coordinates.
fn find_coordinates_of_entity(entity: Entity, world: &World) -> &Coordinates {
    let location = world
        .get::<Location>(entity)
        .expect("entity should have a location");

    world
        .get::<Coordinates>(location.id)
        .expect("entity should be located in an entity with coordinates")
}

/// Finds the icon associated with the room at the provided location.
///
/// Panics if the provided coordinates map to an entity that isn't a room.
fn icon_for_coords(coords: &Coordinates, world: &World) -> MapIcon {
    if let Some(entity) = world.resource::<GameMap>().locations.get(coords) {
        return world
            .get::<Room>(*entity)
            .expect("coordinates should map to a room")
            .map_icon
            .clone();
    }

    BLANK_ICON.clone()
}
