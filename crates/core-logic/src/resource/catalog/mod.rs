use bevy_ecs::prelude::*;
use strum::IntoEnumIterator;

mod ammo_caliber_name_catalog;
pub use ammo_caliber_name_catalog::*;

/// Trait for resources that represent a catalog of things to associates values (such as names).
// TODO convert existing catalogs to use this
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

    /// Gets the value of the provided thing.
    fn get_value(thing: &K, world: &World) -> Self::V;

    /// Sets the value of the provided thing.
    fn set(&mut self, thing: &K, value: Self::V);

    /// Determines the value for the provided thing.
    fn get(&self, thing: &K) -> Self::V;
}
