use bevy_ecs::prelude::*;

//TODO doc
pub struct FoundEntities<T> {
    pub exact_matches: Vec<Entity>,
    pub partial_matches: Vec<PartialMatchingEntity<T>>,
}

//TODO doc
pub struct PartialMatchingEntity<T> {
    pub entity: Entity,
    pub match_info: T,
}

impl<T> FoundEntities<T> {
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
