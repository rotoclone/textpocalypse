use anyhow::Result;
use crossterm::{
    cursor,
    style::style,
    style::{Print, Stylize},
    terminal::{Clear, ClearType},
    QueueableCommand,
};
use cruet::Inflector;
use itertools::Itertools;
use log::debug;
use std::{
    cmp::Ordering,
    io::{stdin, stdout, Write},
    sync::{
        atomic::{self, AtomicBool},
        Arc,
    },
    thread,
    time::Duration,
};

use core_logic::{
    ActionDescription, AttributeDescription, AttributeType, ContainerDescription,
    ContainerEntityDescription, DetailedEntityDescription, Direction, EntityDescription,
    ExitDescription, Game, GameMessage, HelpMessage, MapChar, MapDescription, MapIcon,
    MessageDelay, RoomConnectionEntityDescription, RoomDescription, RoomEntityDescription,
    RoomLivingEntityDescription, RoomObjectDescription, Time, CHARS_PER_TILE,
};

const PROMPT: &str = "\n> ";
const INDENT: &str = "  ";
const FIRST_PM_HOUR: u8 = 12;
const SHORT_MESSAGE_DELAY: Duration = Duration::from_millis(333);
const LONG_MESSAGE_DELAY: Duration = Duration::from_millis(666);

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
            let delay = delay_for_message(&message);
            render_message(message, game_time).unwrap();
            thread::sleep(delay);
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

/// Determines the amount of time to wait after displaying the provided message.
fn delay_for_message(message: &GameMessage) -> Duration {
    match message {
        GameMessage::Message(_, delay) => match delay {
            MessageDelay::None => Duration::ZERO,
            MessageDelay::Short => SHORT_MESSAGE_DELAY,
            MessageDelay::Long => LONG_MESSAGE_DELAY,
        },
        _ => Duration::ZERO,
    }
}

/// Renders the provided `GameMessage` to the screen
fn render_message(message: GameMessage, time: Time) -> Result<()> {
    let output = match message {
        GameMessage::Error(e) => e,
        GameMessage::Message(m, _) => m,
        GameMessage::Help(h) => help_to_string(h),
        GameMessage::Room(room) => room_to_string(room, time),
        GameMessage::Entity(entity) => entity_to_string(entity),
        GameMessage::DetailedEntity(entity) => detailed_entity_to_string(entity),
        GameMessage::Container(container) => container_to_string(container),
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
    let map = map_to_string(&room.map);
    let name = style(room.name).bold();
    let time = style(format!("({})", time_to_string(time))).dark_grey();
    let desc = room.description;
    let entities = if room.entities.is_empty() {
        "".to_string()
    } else {
        format!("\n\n{}", room_entities_to_string(&room.entities))
    };
    let exits = format!("Exits: {}", exits_to_string(room.exits));

    let mini_map_and_desc = format_side_by_side(&map, &format!("{name} {time}\n\n{desc}"), " ");

    format!("{mini_map_and_desc}{entities}\n\n{exits}")
}

/// Transforms the provided map into a string for display.
fn map_to_string<const S: usize>(map: &MapDescription<S>) -> String {
    let width = S * CHARS_PER_TILE;
    let mut output = format!("+{}+\n", "-".repeat(width));
    for row in &map.tiles {
        output.push('|');
        for icon in row {
            output.push_str(&map_icon_to_string(icon));
        }
        output.push_str("|\n");
    }
    output.push_str(&format!("+{}+", "-".repeat(width)));

    output
}

/// Transforms the provided map icon into a string for display.
fn map_icon_to_string(icon: &MapIcon) -> String {
    let mut output = String::new();
    for map_char in icon.chars {
        output.push_str(&map_char_to_string(&map_char));
    }

    output
}

/// Transforms the provided map character into a string for display.
fn map_char_to_string(map_char: &MapChar) -> String {
    style(map_char.value)
        .on(convert_game_color_to_term_color(&map_char.bg_color))
        .with(convert_game_color_to_term_color(&map_char.fg_color))
        .to_string()
}

/// Converts the provided game color to its corresponding terminal color.
fn convert_game_color_to_term_color(color: &core_logic::Color) -> crossterm::style::Color {
    match color {
        core_logic::Color::Black => crossterm::style::Color::Black,
        core_logic::Color::DarkGray => crossterm::style::Color::DarkGrey,
        core_logic::Color::Red => crossterm::style::Color::Red,
        core_logic::Color::DarkRed => crossterm::style::Color::DarkRed,
        core_logic::Color::Green => crossterm::style::Color::Green,
        core_logic::Color::DarkGreen => crossterm::style::Color::DarkGreen,
        core_logic::Color::Yellow => crossterm::style::Color::Yellow,
        core_logic::Color::DarkYellow => crossterm::style::Color::DarkYellow,
        core_logic::Color::Blue => crossterm::style::Color::Blue,
        core_logic::Color::DarkBlue => crossterm::style::Color::DarkBlue,
        core_logic::Color::Magenta => crossterm::style::Color::Magenta,
        core_logic::Color::DarkMagenta => crossterm::style::Color::DarkMagenta,
        core_logic::Color::Cyan => crossterm::style::Color::Cyan,
        core_logic::Color::DarkCyan => crossterm::style::Color::DarkCyan,
        core_logic::Color::White => crossterm::style::Color::White,
        core_logic::Color::Gray => crossterm::style::Color::Grey,
    }
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

    [living_entities, object_entities, connection_entities].join("\n\n")
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
        let name = if i == 0 {
            // capitalize the article if there is one, otherwise capitalize the name
            if let Some(article) = &entity.article {
                format!(
                    "{} {}",
                    article.to_sentence_case(),
                    style(&entity.name).bold()
                )
            } else {
                style(&entity.name.to_sentence_case()).bold().to_string()
            }
        } else {
            format!(
                "{}{}",
                entity
                    .article
                    .as_ref()
                    .map(|a| format!("{a} "))
                    .unwrap_or_else(|| "".to_string()),
                style(&entity.name).bold()
            )
        };

        let desc = format!("{} leads {}", name, style(entity.direction).bold(),);
        descriptions.push(desc);
    }

    Some(format!("{}.", format_list(&descriptions)))
}

/// Transforms the provided entity description into a string for display.
fn entity_to_string(entity: EntityDescription) -> String {
    let name = style(entity.name).bold();
    let aliases = if entity.aliases.is_empty() {
        "".to_string()
    } else {
        style(format!(" (aka {})", entity.aliases.join(", ")))
            .dark_grey()
            .to_string()
    };
    let desc = entity.description;
    let attributes = entity_attributes_to_string(&entity.attributes)
        .map_or_else(|| "".to_string(), |s| format!("\n\n{s}"));

    format!("{name}{aliases}\n{desc}{attributes}")
}

/// Transforms the provided entity attribute descriptions into a string for display.
fn entity_attributes_to_string(attributes: &[AttributeDescription]) -> Option<String> {
    if attributes.is_empty() {
        return None;
    }

    let mut is_descriptions = Vec::new();
    let mut does_descriptions = Vec::new();
    let mut has_descriptions = Vec::new();
    for attribute in attributes {
        let description = attribute.description.clone();
        match attribute.attribute_type {
            AttributeType::Is => is_descriptions.push(description),
            AttributeType::Does => does_descriptions.push(description),
            AttributeType::Has => has_descriptions.push(description),
        }
    }

    let is_description = if is_descriptions.is_empty() {
        None
    } else {
        Some(format!("It's {}.", format_list(&is_descriptions)))
    };

    let does_description = if does_descriptions.is_empty() {
        None
    } else {
        Some(format!("It {}.", format_list(&does_descriptions)))
    };

    let has_description = if has_descriptions.is_empty() {
        None
    } else {
        Some(format!("It has {}.", format_list(&has_descriptions)))
    };

    Some([is_description, does_description, has_description].join("\n\n"))
}

/// Transforms the provided detailed entity description into a string for display.
fn detailed_entity_to_string(entity: DetailedEntityDescription) -> String {
    let basic_desc = Some(entity_to_string(entity.basic_desc));
    let actions = action_descriptions_to_string("Actions:", &entity.actions);

    [basic_desc, actions].join("\n\n")
}

/// Transforms the provided help message into a string for display.
fn help_to_string(help: HelpMessage) -> String {
    action_descriptions_to_string("Available actions:", &help.actions).unwrap_or_default()
}

/// Transforms the provided list of action descriptions into a string for display.
fn action_descriptions_to_string(
    header: &str,
    action_descriptions: &[ActionDescription],
) -> Option<String> {
    if action_descriptions.is_empty() {
        None
    } else {
        Some(format!(
            "{}\n{}",
            header,
            action_descriptions
                .iter()
                .map(|a| format!("{INDENT}{}", a.format))
                .collect::<Vec<String>>()
                .join("\n")
        ))
    }
}

/// Transforms the provided container description into a string for display.
fn container_to_string(container: ContainerDescription) -> String {
    let contents = container
        .items
        .iter()
        .map(|item| format!("{INDENT}{}", container_entity_to_string(item)))
        .join("\n");

    let max_volume = container
        .max_volume
        .map(|x| x.to_string())
        .unwrap_or_else(|| "-".to_string());
    let max_weight = container
        .max_weight
        .map(|x| x.to_string())
        .unwrap_or_else(|| "-".to_string());
    let usage = format!(
        "{}/{}L  {}/{}kg",
        container.used_volume, max_volume, container.used_weight, max_weight
    );

    format!("Contents:\n\n{contents}\n{usage}")
}

/// Transforms the provided container entity description into a string for display.
fn container_entity_to_string(entity: &ContainerEntityDescription) -> String {
    format!(
        "{} [{}L] [{}kg]",
        style(entity.name.clone()).bold(),
        entity.volume,
        entity.weight
    )
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

trait Join<T> {
    fn join(self, between: &str) -> T;
}

impl<const N: usize> Join<String> for [Option<String>; N] {
    fn join(self, between: &str) -> String {
        self.into_iter()
            .flatten()
            .collect::<Vec<String>>()
            .join(between)
    }
}

/// Combines the provided strings into a new string with the contents of the strings next to each other, separated by the provided separator.
/// For example, with inputs of `"a1\na2"` and `"b1\nb2"` and a separator of `"|"`, the resulting output would be `"a1|b1\na2|b2"`
fn format_side_by_side(str1: &str, str2: &str, separator: &str) -> String {
    str1.lines()
        .zip_longest(str2.lines())
        .map(|pair| {
            let (a, b) = pair.or_default();
            format!("{a}{separator}{b}")
        })
        .join("\n")
}
