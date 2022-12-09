use std::fmt::Display;

/// Directions in which different rooms can be connected.
#[derive(PartialEq, Eq, Hash, Clone, Copy, Debug)]
pub enum Direction {
    North,
    NorthEast,
    East,
    SouthEast,
    South,
    SouthWest,
    West,
    NorthWest,
    Up,
    Down,
}

impl Direction {
    /// Parses the provided string to a `Direction`. Returns `None` if the string doesn't map to any direction.
    pub fn parse(input: &str) -> Option<Direction> {
        match input {
            "n" | "north" => Some(Direction::North),
            "ne" | "northeast" => Some(Direction::NorthEast),
            "e" | "east" => Some(Direction::East),
            "se" | "southeast" => Some(Direction::SouthEast),
            "s" | "south" => Some(Direction::South),
            "sw" | "southwest" => Some(Direction::SouthWest),
            "w" | "west" => Some(Direction::West),
            "nw" | "northwest" => Some(Direction::NorthWest),
            "u" | "up" => Some(Direction::Up),
            "d" | "down" => Some(Direction::Down),
            _ => None,
        }
    }
}

impl Display for Direction {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let string = match self {
            Direction::North => "north",
            Direction::NorthEast => "northeast",
            Direction::East => "east",
            Direction::SouthEast => "southeast",
            Direction::South => "south",
            Direction::SouthWest => "southwest",
            Direction::West => "west",
            Direction::NorthWest => "northwest",
            Direction::Up => "up",
            Direction::Down => "down",
        };

        write!(f, "{string}")
    }
}
