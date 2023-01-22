use comfy_table::{Cell, ContentArrangement, Table};
use crossterm::{style::style, style::Stylize};
use itertools::Itertools;
use std::{cmp::Ordering, hash::Hash};
use voca_rs::Voca;

use core_logic::*;

const INDENT: &str = "  ";
const FIRST_PM_HOUR: u8 = 12;

const BAR_LENGTH: usize = 20;
const BAR_START: &str = "[";
const BAR_END: &str = "]";
const BAR_FILLED: &str = "|";
const BAR_EMPTY: &str = " ";
const BAR_REDUCTION: &str = "-";
const BAR_ADDITION: &str = "+";

const MAX_WIDTH: usize = 80;

/// Transforms the provided message into a string for display.
pub fn message_to_string(message: GameMessage, time: Option<Time>) -> String {
    match message {
        GameMessage::Error(e) => e._capitalize(false),
        GameMessage::Message { content, .. } => content._capitalize(false),
        GameMessage::Help(h) => help_to_string(h),
        GameMessage::Room(room) => room_to_string(room, time),
        GameMessage::Entity(entity) => entity_to_string(entity),
        GameMessage::DetailedEntity(entity) => detailed_entity_to_string(entity),
        GameMessage::Container(container) => container_to_string(container),
        GameMessage::Vitals(vitals) => vitals_to_string(vitals),
        GameMessage::ValueChange(change, _) => value_change_to_string(change),
        GameMessage::Players(players) => players_to_string(players),
    }
}

/// Transforms the provided room description into a string for display.
fn room_to_string(room: RoomDescription, time: Option<Time>) -> String {
    let map = map_to_string(&room.map);
    let name = style(room.name).bold();
    let time = if let Some(time) = time {
        style(format!("({})", time_to_string(time)))
            .dark_grey()
            .to_string()
    } else {
        "".to_string()
    };
    let desc = room.description;
    let entities = if room.entities.is_empty() {
        "".to_string()
    } else {
        format!("\n\n{}", room_entities_to_string(&room.entities))
    };
    let exits = format!("Exits: {}", exits_to_string(room.exits));

    let mini_map_width = room.map.tiles.len() * CHARS_PER_TILE;
    let separator = " ";
    let max_desc_width = MAX_WIDTH - mini_map_width - separator.len();
    let wrapped_desc = desc._word_wrap(max_desc_width, "\n", "");

    let mini_map_and_desc =
        format_side_by_side(&map, &format!("{name} {time}\n\n{wrapped_desc}"), separator);

    format!("{mini_map_and_desc}{entities}\n\n{exits}")
}

/// Transforms the provided map into a string for display.
fn map_to_string<const S: usize>(map: &MapDescription<S>) -> String {
    let mut output = String::new();
    for (i, row) in map.tiles.iter().enumerate() {
        for icon in row {
            output.push_str(&map_icon_to_string(icon));
        }
        if i < map.tiles.len() - 1 {
            output.push('\n');
        }
    }

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

    let living_entities_counted = group_and_count(living_entity_descriptions, |d| d.name.clone());
    let object_entities_counted = group_and_count(object_entity_descriptions, |d| d.name.clone());

    let living_entities = living_entities_to_string(&living_entities_counted);
    let object_entities = object_entities_to_string(&object_entities_counted);
    let connection_entities = connection_entities_to_string(&connection_entity_descriptions);

    [living_entities, object_entities, connection_entities].join("\n\n")
}

/// Returns a list of de-duplicated items along with how many times that item appeared in the input list.
///
/// An item is considered to be the same as another item if the provided group function returns the same value for both items.
fn group_and_count<T, K, F>(items: Vec<T>, group_fn: F) -> Vec<(T, usize)>
where
    T: Ord,
    K: Hash + Eq,
    F: Fn(&T) -> K,
{
    let grouped = items.into_iter().into_group_map_by(group_fn);

    grouped
        .into_values()
        // unwrap is safe here because `into_group_map_by` will only create groups with at least 1 item
        .map(|mut group| {
            let len = group.len();
            (group.pop().unwrap(), len)
        })
        .sorted()
        .collect()
}

/// Transforms the provided living entity descriptions into a string for display as part of a room description.
///
/// Takes in a list of pairs of entity descriptions and how many of that entity are in the room.
fn living_entities_to_string(entities: &[(&RoomLivingEntityDescription, usize)]) -> Option<String> {
    if entities.is_empty() {
        return None;
    }

    let is_or_are = if entities.len() == 1 { "is" } else { "are" };

    let mut descriptions = Vec::new();
    for (i, (entity, count)) in entities.iter().enumerate() {
        let article;
        let entity_name;
        if *count == 1 {
            article = entity
                .article
                .as_ref()
                .map(|a| format!("{a} "))
                .unwrap_or_else(|| "".to_string());
            entity_name = &entity.name;
        } else {
            article = format!("{count} ");
            entity_name = &entity.plural_name;
        }

        let name = if i == 0 {
            entity_name._capitalize(false)
        } else {
            entity_name.clone()
        };

        let desc = format!("{article}{name}");
        descriptions.push(style(desc).bold().to_string());
    }

    Some(format!(
        "{} {} standing here.",
        format_list(&descriptions),
        is_or_are
    ))
}

/// Transforms the provided object entity descriptions into a string for display as part of a room description.
///
/// Takes in a list of pairs of entity descriptions and how many of that entity are in the room.
fn object_entities_to_string(entities: &[(&RoomObjectDescription, usize)]) -> Option<String> {
    if entities.is_empty() {
        return None;
    }

    let descriptions = entities
        .iter()
        .map(|(entity, count)| {
            let article;
            let entity_name;
            if *count == 1 {
                article = entity
                    .article
                    .as_ref()
                    .map(|a| format!("{a} "))
                    .unwrap_or_else(|| "".to_string());
                entity_name = &entity.name;
            } else {
                article = format!("{count} ");
                entity_name = &entity.plural_name;
            }
            style(format!("{article}{entity_name}")).bold().to_string()
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
                format!("{} {}", article._capitalize(false), entity.name)
            } else {
                entity.name._capitalize(false)
            }
        } else {
            format!(
                "{}{}",
                entity
                    .article
                    .as_ref()
                    .map(|a| format!("{a} "))
                    .unwrap_or_else(|| "".to_string()),
                entity.name
            )
        };

        let desc = format!(
            "{} leads {}",
            style(name).bold(),
            style(entity.direction).bold(),
        );
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
    let attributes = entity_attributes_to_string(entity.attributes, entity.pronouns)
        .map_or_else(|| "".to_string(), |s| format!("\n\n{s}"));

    format!("{name}{aliases}\n{desc}{attributes}")
}

/// Transforms the provided entity attribute descriptions into a string for display on the entity with the provided pronouns.
fn entity_attributes_to_string(
    attributes: Vec<AttributeDescription>,
    pronouns: Pronouns,
) -> Option<String> {
    if attributes.is_empty() {
        return None;
    }

    let mut is_descriptions = Vec::new();
    let mut does_descriptions = Vec::new();
    let mut has_descriptions = Vec::new();
    let mut messages = Vec::new();
    for attribute in attributes {
        match attribute {
            AttributeDescription::Basic(basic_attribute) => {
                let description = basic_attribute.description.clone();
                match basic_attribute.attribute_type {
                    AttributeType::Is => is_descriptions.push(description),
                    AttributeType::Does => does_descriptions.push(description),
                    AttributeType::Has => has_descriptions.push(description),
                }
            }
            AttributeDescription::Message(m) => messages.push(m),
        }
    }

    let capitalized_personal_subj_pronoun = pronouns.personal_subject._capitalize(false);

    let is_description = if is_descriptions.is_empty() {
        None
    } else {
        let is_or_are = if pronouns.plural { "are" } else { "is" };
        Some(format!(
            "{} {} {}.",
            capitalized_personal_subj_pronoun,
            is_or_are,
            format_list(&is_descriptions)
        ))
    };

    let does_description = if does_descriptions.is_empty() {
        None
    } else {
        Some(format!(
            "{} {}.",
            capitalized_personal_subj_pronoun,
            format_list(&does_descriptions)
        ))
    };

    let has_description = if has_descriptions.is_empty() {
        None
    } else {
        let has_or_have = if pronouns.plural { "have" } else { "has" };
        Some(format!(
            "{} {} {}.",
            capitalized_personal_subj_pronoun,
            has_or_have,
            format_list(&has_descriptions)
        ))
    };

    let messages_description = if messages.is_empty() {
        None
    } else {
        Some(
            messages
                .into_iter()
                .map(|message| message_to_string(message, None))
                .collect::<Vec<String>>()
                .join("\n\n"),
        )
    };

    Some(
        [
            is_description,
            does_description,
            has_description,
            messages_description,
        ]
        .join("\n\n"),
    )
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
        "{:.2}/{:.2}L  {:.2}/{:.2}kg",
        container.used_volume, max_volume, container.used_weight, max_weight
    );

    format!("Contents:\n{contents}\n\nTotal: {usage}")
}

/// Transforms the provided container entity description into a string for display.
fn container_entity_to_string(entity: &ContainerEntityDescription) -> String {
    let volume_and_weight = format!("[{:.2}L] [{:.2}kg]", entity.volume, entity.weight);

    format!(
        "{} {}",
        style(entity.name.clone()).bold(),
        style(volume_and_weight).dark_grey(),
    )
}

/// Transforms the provided vitals description into a string for display.
fn vitals_to_string(vitals: VitalsDescription) -> String {
    let health = format!(
        "Health:    {}",
        constrained_float_to_string(None, vitals.health, value_type_to_color(&ValueType::Health))
    );
    let satiety = format!(
        "Satiety:   {}",
        constrained_float_to_string(
            None,
            vitals.satiety,
            value_type_to_color(&ValueType::Satiety)
        )
    );
    let hydration = format!(
        "Hydration: {}",
        constrained_float_to_string(
            None,
            vitals.hydration,
            value_type_to_color(&ValueType::Hydration)
        )
    );
    let energy = format!(
        "Energy:    {}",
        constrained_float_to_string(None, vitals.energy, value_type_to_color(&ValueType::Energy))
    );

    [health, satiety, hydration, energy].join("\n")
}

/// Transforms the provided value change description into a string for display.
fn value_change_to_string(change: ValueChangeDescription) -> String {
    let bar_title = format!("{}: ", value_type_to_bar_title(&change.value_type));
    let color = value_type_to_color(&change.value_type);
    format!(
        "{}\n{}{}",
        change.message,
        bar_title,
        constrained_float_to_string(Some(change.old_value), change.new_value, color)
    )
}

/// Determines the bar title to use for a value of the provided type.
fn value_type_to_bar_title(value_type: &ValueType) -> String {
    match value_type {
        ValueType::Health => "Health",
        ValueType::Satiety => "Satiety",
        ValueType::Hydration => "Hydration",
        ValueType::Energy => "Energy",
    }
    .to_string()
}

/// Determines the bar color to use for a value of the provided type.
fn value_type_to_color(value_type: &ValueType) -> crossterm::style::Color {
    match value_type {
        ValueType::Health => crossterm::style::Color::Red,
        ValueType::Satiety => crossterm::style::Color::Yellow,
        ValueType::Hydration => crossterm::style::Color::Blue,
        ValueType::Energy => crossterm::style::Color::Green,
    }
}

/// Transforms the provided constrained value into a string that looks like a bar for display.
fn constrained_float_to_string(
    old_value: Option<ConstrainedValue<f32>>,
    value: ConstrainedValue<f32>,
    color: crossterm::style::Color,
) -> String {
    let filled_fraction = value.get() / value.get_max();
    let old_filled_fraction = if let Some(old_value) = old_value {
        old_value.get() / old_value.get_max()
    } else {
        filled_fraction
    };

    let old_num_filled = (BAR_LENGTH as f32 * old_filled_fraction).round() as usize;
    let mut num_filled = (BAR_LENGTH as f32 * filled_fraction).round() as usize;
    let num_changed = old_num_filled.abs_diff(num_filled);

    let bar_change = if filled_fraction < old_filled_fraction {
        style(BAR_REDUCTION.repeat(num_changed)).red()
    } else {
        num_filled = num_filled.saturating_sub(num_changed);
        style(BAR_ADDITION.repeat(num_changed)).green()
    };

    let num_empty = BAR_LENGTH
        .saturating_sub(num_filled)
        .saturating_sub(num_changed);

    let bar = format!(
        "{}{}{}",
        style(BAR_FILLED.repeat(num_filled)).with(color),
        bar_change,
        BAR_EMPTY.repeat(num_empty)
    );
    let values = style(format!("{:.0}/{:.0}", value.get(), value.get_max())).dark_grey();

    format!("{BAR_START}{bar}{BAR_END} {values}")
}

/// Transforms the provided players description into a string for display.
fn players_to_string(players: PlayersMessage) -> String {
    let mut table = Table::new();
    table
        .set_content_arrangement(ContentArrangement::Dynamic)
        .set_width(MAX_WIDTH.try_into().expect("max size should fit in u16"))
        .set_header(vec![
            Cell::new("Name"),
            Cell::new("Queued action?"),
            Cell::new("AFK?"),
        ]);

    for player in players.players {
        let mut name = Cell::new(player.name);
        if player.is_self {
            name = name.add_attribute(comfy_table::Attribute::Bold);
        } else if player.is_afk {
            name = name.fg(comfy_table::Color::DarkGrey);
        }

        let mut has_queued_action = if player.has_queued_action {
            Cell::new("Y").fg(comfy_table::Color::Green)
        } else {
            Cell::new("N").fg(comfy_table::Color::Red)
        };

        if player.is_afk {
            has_queued_action = has_queued_action.fg(comfy_table::Color::Grey)
        }

        let is_afk = if player.is_afk {
            Cell::new("Y").fg(comfy_table::Color::Green)
        } else {
            Cell::new("N").fg(comfy_table::Color::Red)
        };

        table.add_row(vec![name, has_queued_action, is_afk]);
    }

    table.to_string()
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
