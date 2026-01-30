use bevy_ecs::prelude::*;

use crate::component::Description;

/* TODO remove
pub trait ValidateParsedValue<T>: ValidateParsedValueUntyped + ValidateParsedValueClone<T> {
    fn validate(
        &self,
        context: PartValidatorContext<T>,
        world: &World,
    ) -> CommandPartValidateResult;

    fn as_untyped(&self) -> Box<dyn ValidateParsedValueUntyped>;
}

pub trait ValidateParsedValueUntyped:
    std::fmt::Debug + Send + Sync + ValidateParsedValueUntypedClone
{
    fn validate(
        &self,
        context: PartValidatorContext<ParsedValue>,
        world: &World,
    ) -> CommandPartValidateResult;
}

/// This trait exists because adding regular `Clone` to a trait makes it not object-safe, but doing this silly thing works apparently.
/// https://stackoverflow.com/a/30353928
pub trait ValidateParsedValueUntypedClone {
    fn clone_box(&self) -> Box<dyn ValidateParsedValueUntyped>;
}

impl<T: 'static + ValidateParsedValueUntyped + Clone> ValidateParsedValueUntypedClone for T {
    fn clone_box(&self) -> Box<dyn ValidateParsedValueUntyped> {
        Box::new(self.clone())
    }
}

/// This trait exists because adding regular `Clone` to a trait makes it not object-safe, but doing this silly thing works apparently.
/// https://stackoverflow.com/a/30353928
pub trait ValidateParsedValueClone<T> {
    fn clone_box(&self) -> Box<dyn ValidateParsedValue<T>>;
}

impl<T: 'static + ValidateParsedValue<P> + Clone, P> ValidateParsedValueClone<P> for T {
    fn clone_box(&self) -> Box<dyn ValidateParsedValue<P>> {
        Box::new(self.clone())
    }
}
    */

/// TODO doc
#[derive(Debug)]
pub struct PartValidatorContext<T> {
    pub parsed_value: T,
    pub performing_entity: Entity,
}

/// TODO doc
pub enum CommandPartValidateResult {
    Valid,
    Invalid(CommandPartValidateError),
}

/// TODO doc
#[derive(PartialEq, Eq, Debug)]
pub struct CommandPartValidateError {
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
