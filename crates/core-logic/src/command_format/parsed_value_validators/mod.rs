use bevy_ecs::prelude::*;

use crate::component::Description;

/// Context included when checking whether a parsed part value is valid
#[derive(Debug)]
pub struct PartValidatorContext<T> {
    /// The parsed value
    pub parsed_value: T,
    /// The entity that entered the command being validated
    pub performing_entity: Entity,
}

/// The result of checking whether a parsed part value is valid
pub enum CommandPartValidateResult {
    /// The parsed value is valid
    Valid,
    /// The parsed value is invalid
    Invalid(CommandPartValidateError),
}

/// An error describing why a parsed part value is invalid
#[derive(PartialEq, Eq, Debug)]
pub struct CommandPartValidateError {
    /// Any details to include in the error message provided to the user
    pub details: Option<String>,
}

/// Validates that an entity has the provided component, and if not returns an error in the format `"You can't {verb_name} {parsed_value_reference_name}."`.
pub fn validate_parsed_value_has_component<T: Component>(
    context: &PartValidatorContext<Entity>,
    verb_name: &str,
    world: &World,
) -> CommandPartValidateResult {
    if world.get::<T>(context.parsed_value).is_some() {
        CommandPartValidateResult::Valid
    } else {
        build_invalid_result(context, verb_name, None, world)
    }
}

/// Validates that an entity has the provided component, and if not returns an error in the format `"You can't {verb_name} {parsed_value_reference_name}."`,
/// or `"You can't {verb_name} {parsed_value_reference_name} {suffix}."` if a suffix is provided.
pub fn validate_parsed_value_has_component_with_suffix<T: Component>(
    context: &PartValidatorContext<Entity>,
    verb_name: &str,
    suffix: &str,
    world: &World,
) -> CommandPartValidateResult {
    if world.get::<T>(context.parsed_value).is_some() {
        CommandPartValidateResult::Valid
    } else {
        build_invalid_result(context, verb_name, Some(suffix), world)
    }
}

/// Builds a `CommandPartValidateResult::Invalid` with error details in the format `"You can't {verb_name} {parsed_value_reference_name}."`.,
/// or `"You can't {verb_name} {parsed_value_reference_name} {suffix}."` if a suffix is provided.
pub fn build_invalid_result(
    context: &PartValidatorContext<Entity>,
    verb_name: &str,
    suffix: Option<&str>,
    world: &World,
) -> CommandPartValidateResult {
    let target_name = if context.parsed_value == context.performing_entity {
        "yourself".to_string()
    } else {
        Description::get_reference_name(
            context.parsed_value,
            Some(context.performing_entity),
            world,
        )
    };

    let suffix = suffix.map(|s| format!(" {s}")).unwrap_or_default();

    CommandPartValidateResult::Invalid(CommandPartValidateError {
        details: Some(format!("You can't {verb_name} {target_name}{suffix}.")),
    })
}
