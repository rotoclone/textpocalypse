use crate::{vital_change::VitalType, ConstrainedValue, VitalChangeDescription};

/// A description of a change of a vital value, but shorter.
///
/// `R` is the resolution of the change. `old_value` and `new_value` will be constrained to ranges of `0..=R`.
/// For example, if `R` is 10, then a 10% change in the vital value would register as a difference of 1 between `old_value` and `new_value`,
/// and if `R` is 5, then a 20% change in the vital value would register as a difference of 1 between `old_value` and `new_value`.
#[derive(Debug, Clone, PartialEq)]
pub struct VitalChangeShortDescription<const R: u8> {
    /// The message to include with the display of the new value.
    pub message: String,
    /// The type of vital that changed.
    pub vital_type: VitalType,
    /// The old value.
    pub old_value: ConstrainedValue<u8>,
    /// The new value.
    pub new_value: ConstrainedValue<u8>,
}

impl<const R: u8> VitalChangeShortDescription<R> {
    pub fn from_vital_change_description(
        description: &VitalChangeDescription,
    ) -> VitalChangeShortDescription<R> {
        let f32_old_value = description.old_value.map_range(0.0..(R as f32));
        let f32_new_value = description.new_value.map_range(0.0..(R as f32));
        let old_value = ConstrainedValue::new(f32_old_value.get().round() as u8, 0, R);
        let new_value = ConstrainedValue::new(f32_new_value.get().round() as u8, 0, R);

        VitalChangeShortDescription {
            message: description.message.clone(),
            vital_type: description.vital_type,
            old_value,
            new_value,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn no_change() {
        let desc = VitalChangeDescription {
            message: "oh hello".to_string(),
            vital_type: VitalType::Health,
            old_value: ConstrainedValue::new(50.0, 0.0, 100.0),
            new_value: ConstrainedValue::new(50.0, 0.0, 100.0),
        };

        let short_desc_even_resolution =
            VitalChangeShortDescription::<10>::from_vital_change_description(&desc);
        assert_eq!(
            VitalChangeShortDescription {
                message: "oh hello".to_string(),
                vital_type: VitalType::Health,
                old_value: ConstrainedValue::new(5, 0, 10),
                new_value: ConstrainedValue::new(5, 0, 10)
            },
            short_desc_even_resolution
        );

        let short_desc_odd_resolution =
            VitalChangeShortDescription::<7>::from_vital_change_description(&desc);
        assert_eq!(
            VitalChangeShortDescription {
                message: "oh hello".to_string(),
                vital_type: VitalType::Health,
                old_value: ConstrainedValue::new(4, 0, 7),
                new_value: ConstrainedValue::new(4, 0, 7)
            },
            short_desc_odd_resolution
        );
    }

    #[test]
    fn change() {
        let desc = VitalChangeDescription {
            message: "oh hello".to_string(),
            vital_type: VitalType::Health,
            old_value: ConstrainedValue::new(50.0, 0.0, 100.0),
            new_value: ConstrainedValue::new(35.0, 0.0, 100.0),
        };

        let short_desc_even_resolution =
            VitalChangeShortDescription::<10>::from_vital_change_description(&desc);
        assert_eq!(
            VitalChangeShortDescription {
                message: "oh hello".to_string(),
                vital_type: VitalType::Health,
                old_value: ConstrainedValue::new(5, 0, 10),
                new_value: ConstrainedValue::new(4, 0, 10)
            },
            short_desc_even_resolution
        );

        let short_desc_odd_resolution =
            VitalChangeShortDescription::<7>::from_vital_change_description(&desc);
        assert_eq!(
            VitalChangeShortDescription {
                message: "oh hello".to_string(),
                vital_type: VitalType::Health,
                old_value: ConstrainedValue::new(4, 0, 7),
                new_value: ConstrainedValue::new(2, 0, 7)
            },
            short_desc_odd_resolution
        );
    }
}
