use anyhow::Result;
use crossterm::{
    cursor,
    style::style,
    style::{Print, Stylize},
    terminal::{Clear, ClearType},
    QueueableCommand,
};
use log::debug;
use std::{
    cmp::Ordering,
    io::{stdin, stdout, Write},
    sync::{
        atomic::{self, AtomicBool},
        Arc,
    },
    thread,
};

use core_logic::{
    Direction, EntityDescription, ExitDescription, Game, GameMessage, LocationDescription, Time,
};

const PROMPT: &str = "\n> ";
const FIRST_PM_HOUR: u8 = 12;

fn main() -> Result<()> {
    env_logger::init();

    let game = Game::new();
    let (commands_sender, messages_receiver) = game.add_player("Player".to_string());

    let quitting = Arc::new(AtomicBool::new(false));
    let quitting_for_thread = Arc::clone(&quitting);

    thread::Builder::new()
        .name("message receiver".to_string())
        .spawn(move || loop {
            let (message, game_time) = match messages_receiver.recv() {
                Ok(x) => x,
                Err(_) => {
                    debug!("Message sender has been dropped");
                    if quitting_for_thread.load(atomic::Ordering::Relaxed) {
                        break;
                    }
                    panic!("Disconnected from game")
                }
            };
            debug!("Got message: {message:?}");
            render_message(message, game_time).unwrap();
        })?;

    let mut input_buf = String::new();
    loop {
        print!("{PROMPT}");
        stdin().read_line(&mut input_buf)?;
        debug!("Raw input: {input_buf:?}");
        let input = input_buf.trim();
        debug!("Trimmed input: {input:?}");

        if input == "quit" {
            quitting.store(true, atomic::Ordering::Relaxed);
            println!("ok bye");
            return Ok(());
        }

        commands_sender.send(input.to_string()).unwrap();

        input_buf.clear();
    }
}

/// Renders the provided `GameMessage` to the screen
fn render_message(message: GameMessage, time: Time) -> Result<()> {
    let output = match message {
        GameMessage::Error(e) => e,
        GameMessage::Message(m) => m,
        GameMessage::Location(loc) => location_to_string(loc, time),
        GameMessage::Entity(entity) => entity_to_string(entity),
    };
    stdout()
        .queue(Clear(ClearType::CurrentLine))?
        .queue(cursor::MoveToColumn(0))?
        .queue(Print(output))?
        .queue(Print("\n"))?
        .queue(Print(PROMPT))?
        .flush()?;

    Ok(())
}

/// Transforms the provided location description into a string for display.
fn location_to_string(location: LocationDescription, time: Time) -> String {
    let name = style(location.name).bold();
    let time = style(format!("({})", time_to_string(time))).dark_grey();
    let desc = location.description;
    let exits = format!("Exits: {}", exits_to_string(location.exits));

    format!("{name} {time}\n\n{desc}\n\n{exits}")
}

/// Transforms the provided time into a string for display.
fn time_to_string(time: Time) -> String {
    let (hour, am_pm) = match time.hour.cmp(&FIRST_PM_HOUR) {
        Ordering::Less => (time.hour, "AM"),
        Ordering::Equal => (time.hour, "PM"),
        Ordering::Greater => (time.hour - FIRST_PM_HOUR, "PM"),
    };
    let min = time.minute;
    let day = time.day;

    format!("{hour}:{min:02} {am_pm}, Day {day}")
}

/// Transforms the provided exit descriptions into a string for display.
fn exits_to_string(exits: Vec<ExitDescription>) -> String {
    if exits.is_empty() {
        return "None".to_string();
    }

    exits
        .iter()
        .map(exit_to_string)
        .collect::<Vec<String>>()
        .join(", ")
}

/// Transforms the provided exit description into a string for display.
fn exit_to_string(exit: &ExitDescription) -> String {
    let dir = style(direction_to_string(exit.direction)).bold();
    let desc = style(format!("({})", exit.description)).dark_grey();

    format!("{dir} {desc}")
}

/// Transforms the provided direction into a string for display.
fn direction_to_string(dir: Direction) -> String {
    match dir {
        Direction::North => "N",
        Direction::NorthEast => "NE",
        Direction::East => "E",
        Direction::SouthEast => "SE",
        Direction::South => "S",
        Direction::SouthWest => "SW",
        Direction::West => "W",
        Direction::NorthWest => "NW",
    }
    .to_string()
}

/// Transforms the provided entity description into a string for display.
fn entity_to_string(entity: EntityDescription) -> String {
    let name = style(entity.name).bold();
    let desc = entity.description;

    format!("{name}\n{desc}")
}
