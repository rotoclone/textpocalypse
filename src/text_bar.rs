use std::fmt::Display;

use core_logic::*;
use crossterm::style::{style, Stylize};

const FULL_BAR_LENGTH: usize = 20;
const SHORT_BAR_LENGTH: usize = 5;
const BAR_START: &str = "[";
const BAR_END: &str = "]";
const BAR_FILLED: &str = "|";
const BAR_EMPTY: &str = " ";
const BAR_REDUCTION: &str = "-";
const BAR_MINOR_DECREASE: &str = "'";
const BAR_PARTIAL_DECREASE: &str = "*";
const BAR_FULL_DECREASE: &str = "#";
const BAR_ADDITION: &str = "+";

/// Describes a bar that can be rendered as text.
pub struct TextBar {
    /// The old value, if the value changed
    pub old_value: Option<ConstrainedValue<f32>>,
    /// The current value
    pub value: ConstrainedValue<f32>,
    /// Whether the value decreased or not
    pub decreased: bool,
    /// The color of the bar
    pub color: crossterm::style::Color,
    /// The style of the bar
    pub style: BarStyle,
}

/// The style of a visualization of a value within a range.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum BarStyle {
    /// A full-length bar with numbers
    Full,
    /// A short bar with no numbers
    Short,
}

impl Display for TextBar {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let bar_length = match self.style {
            BarStyle::Full => FULL_BAR_LENGTH,
            BarStyle::Short => SHORT_BAR_LENGTH,
        };

        let bar_contents = build_bar_contents(self, bar_length);
        let values = match self.style {
            BarStyle::Full => style(format!(
                " {:.0}/{:.0}",
                self.value.get(),
                self.value.get_max()
            ))
            .dark_grey()
            .to_string(),
            BarStyle::Short => "".to_string(),
        };

        format!("{BAR_START}{bar_contents}{BAR_END}{values}").fmt(f)
    }
}

/// Builds a string representing the middle part of the bar
fn build_bar_contents(text_bar: &TextBar, bar_length: usize) -> String {
    let mut change_params = ChangeParams::new(text_bar, bar_length);
    let original_num_filled = change_params.num_filled;
    let bar_change = if text_bar.decreased {
        match text_bar.style {
            BarStyle::Full => style(BAR_REDUCTION.repeat(change_params.num_changed)).red(),
            BarStyle::Short => {
                if change_params.num_fully_removed + change_params.num_partially_removed > 0 {
                    change_params.num_filled = change_params
                        .num_filled
                        .saturating_sub(change_params.num_partially_removed);
                    style(format!(
                        "{}{}",
                        BAR_FULL_DECREASE.repeat(change_params.num_fully_removed),
                        BAR_PARTIAL_DECREASE.repeat(change_params.num_partially_removed)
                    ))
                    .red()
                } else {
                    change_params.num_filled = change_params.num_filled.saturating_sub(1);
                    style(BAR_MINOR_DECREASE.to_string()).red()
                }
            }
        }
    } else {
        change_params.num_filled = change_params
            .num_filled
            .saturating_sub(change_params.num_changed);
        style(BAR_ADDITION.repeat(change_params.num_changed)).green()
    };

    let num_replaced = original_num_filled.saturating_sub(change_params.num_filled);

    //TODO sometimes num_empty can be too small:
    /*
    k guy
    [*   ] You punch Some Guy in the torso.

    Some Guy lurches forward as his fist sails harmlessly past you.

    k guy
    [#    ] You punch Some Guy in the head.
     */
    //TODO this is probably because sometimes a "partially removed" bar segment is removed enough to not be filled anymore, and sometimes it's still filled
    let num_empty = bar_length
        .saturating_sub(change_params.num_filled)
        .saturating_sub(change_params.num_changed)
        .saturating_sub(num_replaced);

    format!(
        "{}{}{}",
        style(BAR_FILLED.repeat(change_params.num_filled)).with(text_bar.color),
        bar_change,
        BAR_EMPTY.repeat(num_empty)
    )
}

/// Describes how a value change will be displayed.
struct ChangeParams {
    num_filled: usize,
    num_changed: usize,
    num_fully_removed: usize,
    num_partially_removed: usize,
}

impl ChangeParams {
    fn new(bar: &TextBar, bar_length: usize) -> ChangeParams {
        let filled_fraction = bar.value.get() / bar.value.get_max();
        let old_filled_fraction = if let Some(old_value) = bar.old_value {
            old_value.get() / old_value.get_max()
        } else {
            filled_fraction
        };

        let old_num_filled = (bar_length as f32 * old_filled_fraction).round() as usize;
        let num_filled = (bar_length as f32 * filled_fraction).round() as usize;
        let num_changed = old_num_filled.abs_diff(num_filled);

        let raw_change_per_bar_change = bar.value.get_max() / bar_length as f32;
        let raw_change = bar.value.get() - bar.old_value.map(|v| v.get()).unwrap_or(0.0);
        let num_fully_removed = (raw_change.abs() / raw_change_per_bar_change).floor() as usize;
        let num_partially_removed = (raw_change.abs() % raw_change_per_bar_change) as usize;

        ChangeParams {
            num_filled,
            num_changed,
            num_fully_removed,
            num_partially_removed,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn full_style_min_value_no_change() {
        let bar = TextBar {
            old_value: None,
            value: ConstrainedValue::new_min(0.0, 100.0),
            decreased: false,
            color: crossterm::style::Color::Cyan,
            style: BarStyle::Full,
        };

        let expected_bar_contents = format!(
            "{}{}{}",
            style("").with(crossterm::style::Color::Cyan),
            style("").green(),
            BAR_EMPTY.repeat(FULL_BAR_LENGTH)
        );
        let expected_values = style(" 0/100").dark_grey();
        let expected = format!("{BAR_START}{expected_bar_contents}{BAR_END}{expected_values}");

        assert_eq!(expected, bar.to_string());
    }

    #[test]
    fn full_style_max_value_no_change() {
        let bar = TextBar {
            old_value: None,
            value: ConstrainedValue::new_max(0.0, 100.0),
            decreased: false,
            color: crossterm::style::Color::Cyan,
            style: BarStyle::Full,
        };

        let expected_bar_contents = format!(
            "{}{}",
            style(BAR_FILLED.repeat(FULL_BAR_LENGTH)).with(crossterm::style::Color::Cyan),
            style("").green(),
        );
        let expected_values = style(" 100/100").dark_grey();
        let expected = format!("{BAR_START}{expected_bar_contents}{BAR_END}{expected_values}");

        assert_eq!(expected, bar.to_string());
    }

    #[test]
    fn full_style_min_value_decreased_from_max_value() {
        //TODO
    }

    #[test]
    fn full_style_max_value_increased_from_min_value() {
        //TODO
    }

    #[test]
    fn full_style_partial_decrease() {
        //TODO
    }

    #[test]
    fn full_style_partial_increase() {
        //TODO
    }

    //TODO tests for short style
}
