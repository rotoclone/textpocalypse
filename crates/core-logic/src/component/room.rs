use bevy_ecs::prelude::*;

use crate::game_map::MapIcon;

/// A room in the world.
#[derive(PartialEq, Eq, Debug, Component)]
pub struct Room {
    /// The name of the room.
    pub name: String,
    /// The base description of the room.
    pub description: String,
    /// The icon to display on the map for this room.
    pub map_icon: MapIcon,
}
