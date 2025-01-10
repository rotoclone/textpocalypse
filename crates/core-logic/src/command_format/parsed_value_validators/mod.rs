use std::any::Any;

use bevy_ecs::prelude::*;

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
        context: PartValidatorContext<Box<dyn Any>>,
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

pub struct PartValidatorContext<T> {
    parsed_value: T,
    performing_entity: Entity,
}

pub enum CommandPartValidateResult {
    Valid,
    Invalid(CommandPartValidateError),
}

#[derive(PartialEq, Eq, Debug)]
pub enum CommandPartValidateError {
    //TODO
}
