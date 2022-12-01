use anyhow::Result;
use crossterm::{
    cursor,
    style::style,
    style::{Print, Stylize},
    terminal::{Clear, ClearType},
    QueueableCommand,
};
use cruet::Inflector;
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
    Direction, EntityDescription, ExitDescription, Game, GameMessage,
    RoomConnectionEntityDescription, RoomDescription, RoomEntityDescription,
    RoomLivingEntityDescription, RoomObjectDescription, Time,
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

        commands_sender
            .send(input.to_string())
            .expect("Command receiver should exist");

        input_buf.clear();
    }
}

/// Renders the provided `GameMessage` to the screen
fn render_message(message: GameMessage, time: Time) -> Result<()> {
    let output = match message {
        GameMessage::Error(e) => e,
        GameMessage::Message(m) => m,
        GameMessage::Room(room) => room_to_string(room, time),
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

/// Transforms the provided room description into a string for display.
fn room_to_string(room: RoomDescription, time: Time) -> String {
    let name = style(room.name).bold();
    let time = style(format!("({})", time_to_string(time))).dark_grey();
    let desc = room.description;
    let entities = if room.entities.is_empty() {
        "".to_string()
    } else {
        format!("\n\n{}", room_entities_to_string(&room.entities))
    };
    let exits = format!("Exits: {}", exits_to_string(room.exits));

    format!("{name} {time}\n\n{desc}{entities}\n\n{exits}")
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
    let dir = style(direction_to_short_string(exit.direction)).bold();
    let desc = style(format!("({})", exit.description)).dark_grey();

    format!("{dir} {desc}")
}

/// Transforms the provided direction into a short string for display.
fn direction_to_short_string(dir: Direction) -> String {
    match dir {
        Direction::North => "N",
        Direction::NorthEast => "NE",
        Direction::East => "E",
        Direction::SouthEast => "SE",
        Direction::South => "S",
        Direction::SouthWest => "SW",
        Direction::West => "W",
        Direction::NorthWest => "NW",
        Direction::Up => "U",
        Direction::Down => "D",
    }
    .to_string()
}

/// Transforms the provided direction into a long string for display.
fn direction_to_long_string(dir: Direction) -> String {
    match dir {
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
    }
    .to_string()
}

/// Transforms the provided entity descriptions into a string for display as part of a room description.
fn room_entities_to_string(entities: &[RoomEntityDescription]) -> String {
    if entities.is_empty() {
        return "".to_string();
    }

    let mut living_entity_descriptions = Vec::new();
    let mut object_entity_descriptions = Vec::new();
    let mut connection_entity_descriptions = Vec::new();

    for desc in entities {
        match desc {
            RoomEntityDescription::Living(d) => living_entity_descriptions.push(d),
            RoomEntityDescription::Object(d) => object_entity_descriptions.push(d),
            RoomEntityDescription::Connection(d) => connection_entity_descriptions.push(d),
        }
    }

    let living_entities = living_entities_to_string(&living_entity_descriptions);
    let object_entities = object_entities_to_string(&object_entity_descriptions);
    let connection_entities = connection_entities_to_string(&connection_entity_descriptions);

    [living_entities, object_entities, connection_entities]
        .into_iter()
        .flatten()
        .collect::<Vec<String>>()
        .join("\n\n")
}

/// Transforms the provided living entity descriptions into a string for display as part of a room description.
fn living_entities_to_string(entities: &[&RoomLivingEntityDescription]) -> Option<String> {
    if entities.is_empty() {
        return None;
    }

    let is_or_are = if entities.len() == 1 { "is" } else { "are" };

    let mut descriptions = Vec::new();
    for (i, entity) in entities.iter().enumerate() {
        let name = if i == 0 {
            entity.name.to_sentence_case()
        } else {
            entity.name.clone()
        };

        let desc = format!(
            "{}{}",
            entity
                .article
                .as_ref()
                .map(|a| format!("{a} "))
                .unwrap_or_else(|| "".to_string()),
            style(&name).bold(),
        );
        descriptions.push(desc);
    }

    Some(format!(
        "{} {} here.",
        format_list(&descriptions),
        is_or_are
    ))
}

/// Transforms the provided object entity descriptions into a string for display as part of a room description.
fn object_entities_to_string(entities: &[&RoomObjectDescription]) -> Option<String> {
    if entities.is_empty() {
        return None;
    }

    let descriptions = entities
        .iter()
        .map(|entity| {
            format!(
                "{}{}",
                entity
                    .article
                    .as_ref()
                    .map(|a| format!("{a} "))
                    .unwrap_or_else(|| "".to_string()),
                style(&entity.name).bold()
            )
        })
        .collect::<Vec<String>>();

    Some(format!("You see {} here.", format_list(&descriptions)))
}

/// Transforms the provided connection entity descriptions into a string for display as part of a room description.
fn connection_entities_to_string(entities: &[&RoomConnectionEntityDescription]) -> Option<String> {
    if entities.is_empty() {
        return None;
    }

    let mut descriptions = Vec::new();
    for (i, entity) in entities.iter().enumerate() {
        //TODO capitalize the article if there is one, otherwise capitalize the name
        let name = if i == 0 {
            entity.name.to_sentence_case()
        } else {
            entity.name.clone()
        };

        let desc = format!(
            "{}{} leads {}",
            entity
                .article
                .as_ref()
                .map(|a| format!("{a} "))
                .unwrap_or_else(|| "".to_string()),
            style(&name).bold(),
            style(direction_to_long_string(entity.direction)).bold(),
        );
        descriptions.push(desc);
    }

    Some(format!("{}.", format_list(&descriptions)))
}

/// Transforms the provided entity description into a string for display.
fn entity_to_string(entity: EntityDescription) -> String {
    let name = style(entity.name).bold();
    let desc = entity.description;

    format!("{name}\n{desc}")
}

/// Formats a list of items into a single string.
fn format_list(items: &[String]) -> String {
    if items.is_empty() {
        return "".to_string();
    }

    let num_items = items.len();
    let mut string = String::new();
    for (i, item) in items.iter().enumerate() {
        if i == 0 {
            // first item
            string.push_str(item);
        } else if i == num_items - 1 {
            // last item
            if num_items > 2 {
                string.push(',');
            }
            string.push_str(&format!(" and {item}"));
        } else {
            // middle item
            string.push_str(&format!(", {item}"));
        }
    }

    string
}
