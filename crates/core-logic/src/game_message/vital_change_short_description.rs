use crate::{vital_change::VitalType, ConstrainedValue, VitalChangeDescription};

/// A description of a change of a vital value, but shorter.
///
/// `R` is the resolution of the change. `old_value` and `new_value` will be constrained to ranges of `0..=R`.
/// For example, if `R` is 10, then a 10% change in the vital value would register as a difference of 1 between `old_value` and `new_value`,
/// and if `R` is 5, then a 20% change in the vital value would register as a difference of 1 between `old_value` and `new_value`.
#[derive(Debug, Clone, PartialEq)]
pub struct VitalChangeShortDescription<const R: u8> {
    /// The type of vital that changed.
    pub vital_type: VitalType,
    /// The old value.
    pub old_value: ConstrainedValue<u8>,
    /// The new value.
    pub new_value: ConstrainedValue<u8>,
    /// Whether the change is a decrease or not (might not be visible due to low resolution of R)
    pub decreased: bool,
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
            vital_type: description.vital_type,
            old_value,
            new_value,
            decreased: description.new_value < description.old_value,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn no_change() {
        let desc = VitalChangeDescription {
            vital_type: VitalType::Health,
            old_value: ConstrainedValue::new(50.0, 0.0, 100.0),
            new_value: ConstrainedValue::new(50.0, 0.0, 100.0),
        };

        let short_desc_even_resolution =
            VitalChangeShortDescription::<10>::from_vital_change_description(&desc);
        assert_eq!(
            VitalChangeShortDescription {
                vital_type: VitalType::Health,
                old_value: ConstrainedValue::new(5, 0, 10),
                new_value: ConstrainedValue::new(5, 0, 10),
                decreased: false
            },
            short_desc_even_resolution
        );

        let short_desc_odd_resolution =
            VitalChangeShortDescription::<7>::from_vital_change_description(&desc);
        assert_eq!(
            VitalChangeShortDescription {
                vital_type: VitalType::Health,
                old_value: ConstrainedValue::new(4, 0, 7),
                new_value: ConstrainedValue::new(4, 0, 7),
                decreased: false
            },
            short_desc_odd_resolution
        );
    }

    #[test]
    fn decrease() {
        let desc = VitalChangeDescription {
            vital_type: VitalType::Health,
            old_value: ConstrainedValue::new(50.0, 0.0, 100.0),
            new_value: ConstrainedValue::new(35.0, 0.0, 100.0),
        };

        let short_desc_even_resolution =
            VitalChangeShortDescription::<10>::from_vital_change_description(&desc);
        assert_eq!(
            VitalChangeShortDescription {
                vital_type: VitalType::Health,
                old_value: ConstrainedValue::new(5, 0, 10),
                new_value: ConstrainedValue::new(4, 0, 10),
                decreased: true
            },
            short_desc_even_resolution
        );

        let short_desc_odd_resolution =
            VitalChangeShortDescription::<7>::from_vital_change_description(&desc);
        assert_eq!(
            VitalChangeShortDescription {
                vital_type: VitalType::Health,
                old_value: ConstrainedValue::new(4, 0, 7),
                new_value: ConstrainedValue::new(2, 0, 7),
                decreased: true
            },
            short_desc_odd_resolution
        );
    }

    #[test]
    fn increase() {
        let desc = VitalChangeDescription {
            vital_type: VitalType::Health,
            old_value: ConstrainedValue::new(35.0, 0.0, 100.0),
            new_value: ConstrainedValue::new(50.0, 0.0, 100.0),
        };

        let short_desc_even_resolution =
            VitalChangeShortDescription::<10>::from_vital_change_description(&desc);
        assert_eq!(
            VitalChangeShortDescription {
                vital_type: VitalType::Health,
                old_value: ConstrainedValue::new(4, 0, 10),
                new_value: ConstrainedValue::new(5, 0, 10),
                decreased: false
            },
            short_desc_even_resolution
        );

        let short_desc_odd_resolution =
            VitalChangeShortDescription::<7>::from_vital_change_description(&desc);
        assert_eq!(
            VitalChangeShortDescription {
                vital_type: VitalType::Health,
                old_value: ConstrainedValue::new(2, 0, 7),
                new_value: ConstrainedValue::new(4, 0, 7),
                decreased: false
            },
            short_desc_odd_resolution
        );
    }
}
