use bevy_ecs::prelude::*;
use itertools::Itertools;

/// Describes the entities found when searching for entities
#[derive(Default)]
pub struct FoundEntities<T: Ord> {
    /// Any entities that exactly matched the search
    pub exact_matches: Vec<Entity>,
    /// Any entities that partially matched the search
    pub partial_matches: Vec<PartialMatchingEntity<T>>,
    /// The input that was used to find the entities, without the name of the container to search in
    pub searched_name: Option<String>,
    /// The specific container that was searched, if any
    pub container: Option<Entity>,
}

impl<T: Ord> FoundEntities<T> {
    /// Gets the found entities, sorted by any exact matches first, then any partial matches with the best matches first.
    pub fn get_sorted_matches(&self) -> Vec<Entity> {
        self.exact_matches
            .iter()
            .copied()
            .chain(
                self.partial_matches
                    .iter()
                    .sorted()
                    .map(|partial_match| partial_match.entity),
            )
            .collect()
    }
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
    /// Creates a new `FoundEntities` with no matches and no specific container that was searched.
    pub fn new_without_container(searched_name: String) -> FoundEntities<T> {
        FoundEntities {
            exact_matches: Vec::new(),
            partial_matches: Vec::new(),
            searched_name: Some(searched_name),
            container: None,
        }
    }

    /// Creates a new `FoundEntities` with no matches and no specific input used or container that was searched.
    pub fn new_without_input_or_container() -> FoundEntities<T> {
        FoundEntities {
            exact_matches: Vec::new(),
            partial_matches: Vec::new(),
            searched_name: None,
            container: None,
        }
    }

    /// Creates a new `FoundEntities` with no matches and a specific container that was searched.
    pub fn new_with_container(searched_name: String, container: Entity) -> FoundEntities<T> {
        FoundEntities {
            exact_matches: Vec::new(),
            partial_matches: Vec::new(),
            searched_name: Some(searched_name),
            container: Some(container),
        }
    }

    /// Creates a `FoundEntities` with a single exact match and no specific input used or container that was searched.
    pub fn new_single_exact(entity: Entity) -> FoundEntities<T> {
        FoundEntities {
            exact_matches: vec![entity],
            partial_matches: Vec::new(),
            searched_name: None,
            container: None,
        }
    }

    /// Adds `other`'s exact matches to this exact matches, and its partial matches to this partial matches.
    pub fn extend(&mut self, other: FoundEntities<T>) {
        self.exact_matches.extend(other.exact_matches);
        self.partial_matches.extend(other.partial_matches);
    }
}
