use bevy_ecs::prelude::*;

//TODO doc
#[derive(Default)]
pub struct FoundEntities<T: Ord> {
    pub exact_matches: Vec<Entity>,
    pub partial_matches: Vec<PartialMatchingEntity<T>>,
}

//TODO doc
#[derive(PartialEq, Eq)]
pub struct PartialMatchingEntity<T: Ord> {
    pub entity: Entity,
    pub match_info: T,
}

impl<T: Ord> PartialOrd for PartialMatchingEntity<T> {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl<T: Ord> Ord for PartialMatchingEntity<T> {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.match_info.cmp(&other.match_info)
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

//TODO doc
pub struct FoundEntitiesInContainer<T: Ord> {
    pub found_entities: FoundEntities<T>,
    pub searched_container: Option<Entity>,
}
