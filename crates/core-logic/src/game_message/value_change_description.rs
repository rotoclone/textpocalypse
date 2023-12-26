use crate::{ConstrainedValue, ValueType};

/// A description of a change of a single value.
#[derive(Debug, Clone)]
pub struct ValueChangeDescription {
    /// The message to include with the display of the new value.
    pub message: String,
    /// The type of value that changed.
    pub value_type: ValueType,
    /// The old value.
    pub old_value: ConstrainedValue<f32>,
    /// The new value.
    pub new_value: ConstrainedValue<f32>,
}
