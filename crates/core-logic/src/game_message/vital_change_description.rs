use crate::{vital_change::VitalType, ConstrainedValue};

/// A description of a change of a vital value.
#[derive(Debug, Clone)]
pub struct VitalChangeDescription {
    /// The type of vital that changed.
    pub vital_type: VitalType,
    /// The old value.
    pub old_value: ConstrainedValue<f32>,
    /// The new value.
    pub new_value: ConstrainedValue<f32>,
}
