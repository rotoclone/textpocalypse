use bevy_ecs::prelude::*;
use strum::IntoEnumIterator;

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

/* TODO
/// Creates a new catalog with the provided values for standard things.
    fn new() -> Self {
        Self {
            standard: Self::build_standard_values(),
            custom: HashMap::new(),
        }
    }

    /// Builds the default values for standard things.
    fn build_standard_names() -> HashMap<K, Self::V> {
        K::iter()
            .filter_map(|thing| Self::get_default_value(&thing).map(|value| (thing, value)))
            .collect()
    }

    /// Gets the value of the provided thing.
    fn get_value(thing: &K, world: &World) -> Self::V {
        world.resource::<Self>().get(thing)
    }

    /// Sets the value of the provided thing.
    fn set(&mut self, thing: &K, value: Self::V) {
        match thing {
            K::Custom(id) => self.custom.insert(id.clone(), value),
            _ => self.standard.insert(thing.clone(), value),
        };
    }

    /// Determines the value for the provided thing.
    fn get(&self, thing: &K) -> Self::V {
        match thing {
            K::Custom(id) => self.custom.get(id),
            _ => self.standard.get(thing),
        }
        .cloned()
        .unwrap_or_else(|| Self::get_not_found_value())
    }
    */
