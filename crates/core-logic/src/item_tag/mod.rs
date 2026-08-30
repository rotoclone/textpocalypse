use std::any::Any;
use std::hash::Hash;
use std::hash::Hasher;

mod uncategorized;
pub use uncategorized::*;

pub mod shape;
pub mod size;

/// Trait for item tags.
/// Equality is based solely on types, so item tag structs should not have any fields.
pub trait ItemTag: AsAny + Send + Sync + 'static {
    /// Gets the category of this tag, if it has one
    fn category(&self) -> Option<TagCategory>;
}

// TODO should this be a trait instead?
#[derive(PartialEq, Eq, Hash)]
pub enum TagCategory {
    Size,
    Shape,
    Material,
    Sharpness,
    Concentration,
    Custom(String),
}

impl Hash for dyn ItemTag {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.dyn_hash(state);
    }
}
impl PartialEq for dyn ItemTag {
    fn eq(&self, other: &dyn ItemTag) -> bool {
        self.as_any().type_id() == other.as_any().type_id()
    }
}
impl Eq for dyn ItemTag {}

pub trait AsAny {
    fn as_any(&self) -> &dyn Any;
}
impl<T: Any> AsAny for T {
    fn as_any(&self) -> &dyn Any {
        self
    }
}

trait DynHash {
    fn dyn_hash(&self, state: &mut dyn Hasher);
}
impl<H: Hash + ?Sized> DynHash for H {
    fn dyn_hash(&self, mut state: &mut dyn Hasher) {
        self.hash(&mut state);
    }
}
