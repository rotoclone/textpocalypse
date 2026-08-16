use std::{
    any::{Any, TypeId},
    collections::{HashMap, HashSet},
    hash::{Hash, Hasher},
};

use bevy_ecs::prelude::*;

use crate::{Volume, Weight};

pub struct CraftingRecipe {
    name: String,
    ingredients: Vec<CraftingRecipeIngredient>,
}

pub struct CraftingRecipeIngredient {
    name: String,
    //TODO allow grouping ingredients, i.e. "you have to provide at least one of these but not all of them"
    required: bool,
    amount: CraftingIngredientAmount,
    tags: Vec<TagBounds>,
}

pub enum TagBounds {
    Just(TagDescriptor),
    OneOf(HashSet<TagDescriptor>),
    AllOf(HashSet<TagDescriptor>),
    NotAnyOf(HashSet<TagDescriptor>),
}

impl TagBounds {
    /// Determines whether the provided entity is within these bounds.
    fn matches(&self, entity: Entity, world: &World) -> bool {
        todo!() //TODO
    }
}

#[derive(Hash)]
pub enum TagDescriptor {
    Category(TagCategory),
    Tag(Box<dyn CraftingTag>),
}

impl TagDescriptor {
    /// Determines whether the provided entity has a tag matching this descriptor
    fn matches(&self, entity: Entity, world: &World) -> bool {
        match self {
            TagDescriptor::Category(category) => {
                CraftingTags::has_tag_with_category(entity, category, world)
            }
            TagDescriptor::Tag(tag) => CraftingTags::has_tag(entity, tag.as_ref(), world),
        }
    }
}

/// The amount of an ingredient needed for a crafting recipe.
pub enum CraftingIngredientAmount {
    /// A number of items (e.g. 3 screws)
    Items(usize),
    /// A mass of material (e.g. 3 kg of metal)
    Weight(Weight),
    /// A volume of fluid (e.g. 3 L of water)
    Fluid(Volume),
}

// TODO this should go somewhere else probably
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

#[derive(Component)]
pub struct CraftingTags(HashSet<Box<dyn CraftingTag>>);

impl CraftingTags {
    /// Adds a tag to an entity.
    pub fn add_to<T: CraftingTag>(entity: Entity, tag: T, world: &mut World) {
        todo!() //TODO
    }

    /// Adds multiple tags to an entity.
    pub fn add_all_to(entity: Entity, tags: &[Box<dyn CraftingTag>], world: &mut World) {
        todo!() //TODO
    }

    /// Removes a tag from an entity.
    pub fn remove_from<T: CraftingTag>(entity: Entity, tag: T, world: &mut World) {
        todo!() //TODO
    }

    /// Removes multiple tags from an entity.
    pub fn remove_all_from(entity: Entity, tags: &[Box<dyn CraftingTag>], world: &mut World) {
        todo!() //TODO
    }

    /// Adds a tag.
    pub fn add<T: CraftingTag>(&mut self, tag: T) {
        self.0.insert(Box::new(tag));
    }

    /// Determines whether the provided entity has the provided tag.
    pub fn has_tag(entity: Entity, tag: &dyn CraftingTag, world: &World) -> bool {
        world
            .get::<CraftingTags>(entity)
            .is_some_and(|tags| tags.0.contains(tag))
    }

    /// Determines whether the provided entity has any tag with the provided category.
    pub fn has_tag_with_category(entity: Entity, category: &TagCategory, world: &World) -> bool {
        world.get::<CraftingTags>(entity).is_some_and(|tags| {
            tags.0
                .iter()
                .any(|tag| tag.category().as_ref() == Some(category))
        })
    }

    /// Removes a tag.
    pub fn remove(&mut self, tag: &dyn CraftingTag) {
        self.0.remove(tag);
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub struct CraftingTagId(&'static str);

//TODO this should also go isomewhere else
/// Trait for crafting tags.
/// Equality is based solely on types, so crafting tag structs should not have any fields.
pub trait CraftingTag: AsAny + Send + Sync + 'static {
    /// Gets the category of this tag, if it has one
    fn category(&self) -> Option<TagCategory>;
}

impl Hash for dyn CraftingTag {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.dyn_hash(state);
    }
}
impl PartialEq for dyn CraftingTag {
    fn eq(&self, other: &dyn CraftingTag) -> bool {
        self.as_any().type_id() == other.as_any().type_id()
    }
}
impl Eq for dyn CraftingTag {}

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

//TODO these tag structs should go somewhere else

pub struct SizeSmall;

//TODO make a proc macro to auto-derive this
impl CraftingTag for SizeSmall {
    fn category(&self) -> Option<TagCategory> {
        Some(TagCategory::Size)
    }
}

pub struct SizeMedium;

impl CraftingTag for SizeMedium {
    fn category(&self) -> Option<TagCategory> {
        Some(TagCategory::Size)
    }
}

pub struct SizeLarge;

impl CraftingTag for SizeLarge {
    fn category(&self) -> Option<TagCategory> {
        Some(TagCategory::Size)
    }
}

pub struct ShapeRod;

impl CraftingTag for ShapeRod {
    fn category(&self) -> Option<TagCategory> {
        Some(TagCategory::Shape)
    }
}

pub struct ShapeRope;

impl CraftingTag for ShapeRope {
    fn category(&self) -> Option<TagCategory> {
        Some(TagCategory::Shape)
    }
}

//TODO remove
#[allow(unused)]
fn build_test_recipe() -> CraftingRecipe {
    CraftingRecipe {
        name: "Axe".to_string(),
        ingredients: vec![
            CraftingRecipeIngredient {
                name: "handle".to_string(),
                required: true,
                amount: CraftingIngredientAmount::Items(1),
                tags: vec![
                    TagBounds::Just(TagDescriptor::Tag(Box::new(SizeMedium))),
                    TagBounds::Just(TagDescriptor::Tag(Box::new(ShapeRod))),
                ],
            },
            CraftingRecipeIngredient {
                name: "head".to_string(),
                required: true,
                amount: CraftingIngredientAmount::Items(1),
                tags: vec![
                    TagBounds::Just(TagDescriptor::Tag(Box::new(SizeMedium))),
                    TagBounds::Just(TagDescriptor::Category(TagCategory::Sharpness)),
                ],
            },
            CraftingRecipeIngredient {
                name: "rope".to_string(),
                required: true,
                amount: CraftingIngredientAmount::Items(1),
                tags: vec![
                    TagBounds::Just(TagDescriptor::Tag(Box::new(SizeSmall))),
                    TagBounds::Just(TagDescriptor::Tag(Box::new(ShapeRope))),
                ],
            },
        ],
    }
}

fn entity_valid_for_ingredient(
    entity: Entity,
    ingredient: CraftingRecipeIngredient,
    world: &World,
) -> bool {
    ingredient
        .tags
        .iter()
        .all(|tag_bounds| tag_bounds.matches(entity, world))
}
