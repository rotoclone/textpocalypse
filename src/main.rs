use std::io::stdin;

use core_logic::{Action, Direction, PlayerId, World};
use lazy_static::lazy_static;
use log::debug;
use regex::Regex;

lazy_static! {
    static ref MOVE_PATTERN: Regex = Regex::new("^go (.*)").unwrap();
}

fn main() -> anyhow::Result<()> {
    env_logger::init();

    let mut world = World::new();
    let player_id = PlayerId(0);
    world.add_player(player_id, "Player".to_string());

    let initial_state = world.get_state(player_id);
    println!("You appear in {}.", initial_state.location_desc.name);

    let mut input_buf = String::new();
    loop {
        print!("\n> ");
        stdin().read_line(&mut input_buf)?;
        debug!("Raw input: {input_buf:?}");
        let input = input_buf.trim();
        debug!("Trimmed input: {input:?}");

        if let Some(action) = parse_input(input) {
            let result = world.perform_action(player_id, action);
            println!("{}", result.description);
        } else {
            println!("I don't understand that.");
        }

        input_buf.clear();
    }
}

/// Parses the provided string to an `Action`. Returns `None` if the string doesn't map to any action.
fn parse_input(input: &str) -> Option<Action> {
    if let Some(captures) = MOVE_PATTERN.captures(input) {
        if let Some(dir_match) = captures.get(1) {
            if let Some(dir) = parse_direction(dir_match.as_str()) {
                return Some(Action::Move(dir));
            }
        }
    }

    if input == "look" {
        return Some(Action::Look);
    }

    None
}

/// Parses the provided string to a `Direction`. Returns `None` if the string doesn't map to any direction.
fn parse_direction(input: &str) -> Option<Direction> {
    match input {
        "n" | "north" => Some(Direction::North),
        "ne" | "northeast" => Some(Direction::NorthEast),
        "e" | "east" => Some(Direction::East),
        "se" | "southeast" => Some(Direction::SouthEast),
        "s" | "south" => Some(Direction::South),
        "sw" | "southwest" => Some(Direction::SouthWest),
        "w" | "west" => Some(Direction::West),
        "nw" | "northwest" => Some(Direction::NorthWest),
        _ => None,
    }
}
