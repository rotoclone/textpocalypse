use bevy_ecs::prelude::*;

/// Describes the entities found when searching for entities
#[derive(Default)]
pub struct FoundEntities<T: Ord> {
    /// Any entities that exactly matched the search
    pub exact_matches: Vec<Entity>,
    /// Any entities that partially matched the search
    pub partial_matches: Vec<PartialMatchingEntity<T>>,
}

/// An entity that partially matched a search
#[derive(PartialEq, Eq)]
pub struct PartialMatchingEntity<T: Ord> {
    /// The entity
    pub entity: Entity,
    /// Something describing how well the entity matched
    pub match_info: T,
}

impl<T: Ord> PartialOrd for PartialMatchingEntity<T> {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl<T: Ord> Ord for PartialMatchingEntity<T> {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        // reverse because when sorting matches, better matches should come first
        self.match_info.cmp(&other.match_info).reverse()
    }
}

impl<T: Ord> FoundEntities<T> {
    /// Creates a new `FoundEntities` with no matches.
    pub fn new() -> FoundEntities<T> {
        FoundEntities {
            exact_matches: Vec::new(),
            partial_matches: Vec::new(),
        }
    }

    /// Creates a `FoundEntities` with a single exact match.
    pub fn new_single_exact(entity: Entity) -> FoundEntities<T> {
        FoundEntities {
            exact_matches: vec![entity],
            partial_matches: Vec::new(),
        }
    }

    /// Adds `other`'s exact matches to this exact matches, and its partial matches to this partial matches.
    pub fn extend(&mut self, other: FoundEntities<T>) {
        self.exact_matches.extend(other.exact_matches);
        self.partial_matches.extend(other.partial_matches);
    }
}

/// Described the result of searching for entities in a container
pub struct FoundEntitiesInContainer<T: Ord> {
    /// The found entities
    pub found_entities: FoundEntities<T>,
    /// The container that was searched
    pub searched_container: Option<Entity>,
}
