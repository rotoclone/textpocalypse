use std::{array, collections::HashMap};

use bevy_ecs::prelude::*;

use crate::color::Color;

/// The number of characters in the icon for a map tile.
pub const CHARS_PER_TILE: usize = 3;

/// Associates location entities to their coordinates.
#[derive(Resource)]
pub struct GameMap {
    /// The locations in the game world.
    pub locations: HashMap<Coordinates, Entity>,
}

impl GameMap {
    /// Creates an empty map.
    pub fn new() -> GameMap {
        GameMap {
            locations: HashMap::new(),
        }
    }
}

/// The coordinates of a location entity.
#[derive(Component, PartialEq, Eq, Hash, Clone)]
pub struct Coordinates {
    /// Location on the x-axis (east-west).
    /// Higher values are farther east.
    pub x: i64,
    /// Location on the y-axis (north-south).
    /// Higher values are farther north.
    pub y: i64,
    /// Location on the z-axis (up-down).
    /// Higher values are farther up.
    pub z: i64,
    /// The coordinates of the location these coordinates are "in".
    /// If this is `None`, the coordinates are at the top level, or "outside".
    pub parent: Option<Box<Coordinates>>,
}

/// An icon for a tile on the map.
#[derive(PartialEq, Eq, Debug, Clone)]
pub struct MapIcon {
    /// The characters that make up the icon.
    pub chars: [MapChar; CHARS_PER_TILE],
}

impl MapIcon {
    /// Creates an icon where all the characters have the same background and foreground colors.
    pub fn new_uniform(bg_color: Color, fg_color: Color, chars: [char; 3]) -> MapIcon {
        let chars = array::from_fn(|i| MapChar {
            bg_color,
            fg_color,
            value: chars[i],
        });

        MapIcon { chars }
    }

    /// Replaces the center character of this icon with the provided one.
    pub fn replace_center_char(&mut self, replacement: MapChar) {
        self.chars[CHARS_PER_TILE / 2] = replacement;
    }
}

/// A single character in a map icon.
#[derive(PartialEq, Eq, Debug, Clone, Copy)]
pub struct MapChar {
    /// The background color of the character.
    pub bg_color: Color,
    /// The foreground color of the character.
    pub fg_color: Color,
    /// The character itself.
    pub value: char,
}
