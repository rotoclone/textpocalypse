use bevy_ecs::prelude::*;
use strum::IntoEnumIterator;

mod ammo_caliber_name_catalog;
pub use ammo_caliber_name_catalog::*;

mod attribute_name_catalog;
pub use attribute_name_catalog::*;

mod body_part_type_name_catalog;
pub use body_part_type_name_catalog::*;

mod fluid_density_catalog;
pub use fluid_density_catalog::*;

mod fluid_hydration_factor_catalog;
pub use fluid_hydration_factor_catalog::*;

mod fluid_name_catalog;
pub use fluid_name_catalog::*;

mod skill_base_attribute_catalog;
pub use skill_base_attribute_catalog::*;

mod skill_name_catalog;
pub use skill_name_catalog::*;

mod weapon_type_name_catalog;
pub use weapon_type_name_catalog::*;

mod weapon_type_stat_catalog;
pub use weapon_type_stat_catalog::*;

/// Trait for resources that represent a catalog of things to associates values (such as names).
pub trait Catalog<K>: Resource {
    type V;

    /// Gets the default value of a thing.
    fn get_default_value(thing: &K) -> Option<Self::V>;

    /// Gets the value to use if one isn't found in the catalog for a thing.
    fn get_not_found_value() -> Self::V;
}

/// Trait for catalogs to implement that can be auto-derived.
pub trait CatalogBoilerplate<K: Clone + IntoEnumIterator>: Catalog<K> {
    /// Creates a new catalog with the provided values for standard things.
    fn new() -> Self;

    /// Gets the value of the provided thing from the catalog.
    fn get_value(thing: &K, world: &World) -> Self::V;

    /// Sets the value of the provided thing in the catalog.
    #[expect(unused)]
    fn set(&mut self, thing: &K, value: Self::V);

    /// Determines the value for the provided thing in this catalog.
    fn get(&self, thing: &K) -> Self::V;
}
