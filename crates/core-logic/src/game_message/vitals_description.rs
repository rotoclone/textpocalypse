use crate::{component::Vitals, ConstrainedValue};

/// The description of an entity's vitals.
#[derive(Debug, Clone)]
pub struct VitalsDescription {
    /// The health of the entity.
    pub health: ConstrainedValue<f32>,
    /// The non-hunger of the entity.
    pub satiety: ConstrainedValue<f32>,
    /// The non-thirst of the entity.
    pub hydration: ConstrainedValue<f32>,
    /// The non-tiredness of the entity.
    pub energy: ConstrainedValue<f32>,
}

impl VitalsDescription {
    /// Creates a vitals description for the provided vitals.
    pub fn from_vitals(vitals: &Vitals) -> VitalsDescription {
        VitalsDescription {
            health: vitals.health,
            satiety: vitals.satiety,
            hydration: vitals.hydration,
            energy: vitals.energy,
        }
    }
}
