use std::{
    any::Any,
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

pub struct CraftingTags(HashSet<Box<dyn CraftingTag>>);

#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub struct CraftingTagId(&'static str);

//TODO this should also go isomewhere else
pub trait CraftingTag {
    /// Determines whether this tag is equal to the provided one, by type
    fn equals(&self, other: &dyn CraftingTag) -> bool;

    /// Gets the ID of this tag
    fn id(&self) -> CraftingTagId;

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
        TypeEq::type_eq(self, other.as_any())
    }
}
impl Eq for dyn CraftingTag {}

trait AsAny {
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

trait TypeEq {
    fn type_eq(&self, other: &dyn Any) -> bool;
}
impl<T: Any> TypeEq for T {
    fn type_eq(&self, other: &dyn Any) -> bool {
        other.downcast_ref::<Self>().is_some()
    }
}

//TODO these tag structs should go somewhere else
static SIZE_SMALL_ID: CraftingTagId = CraftingTagId("size_small");
static SIZE_MEDIUM_ID: CraftingTagId = CraftingTagId("size_medium");
static SIZE_LARGE_ID: CraftingTagId = CraftingTagId("size_large");

#[derive(Component)]
pub struct SizeSmall;

//TODO make a proc macro to auto-derive this
impl CraftingTag for SizeSmall {
    fn id() -> CraftingTagId {
        SIZE_SMALL_ID
    }

    fn category() -> Option<TagCategory> {
        Some(TagCategory::Size)
    }
}

#[derive(Component)]
pub struct SizeMedium;

impl CraftingTag for SizeMedium {
    fn id() -> CraftingTagId {
        SIZE_MEDIUM_ID
    }

    fn category() -> Option<TagCategory> {
        Some(TagCategory::Size)
    }
}

#[derive(Component)]
pub struct SizeLarge;

impl CraftingTag for SizeLarge {
    fn id() -> CraftingTagId {
        SIZE_LARGE_ID
    }

    fn category() -> Option<TagCategory> {
        Some(TagCategory::Size)
    }
}

static SHAPE_ROD_ID: CraftingTagId = CraftingTagId("shape_rod");
static SHAPE_ROPE_ID: CraftingTagId = CraftingTagId("shape_rope");

#[derive(Component)]
pub struct ShapeRod;

impl CraftingTag for ShapeRod {
    fn id() -> CraftingTagId {
        SHAPE_ROD_ID
    }

    fn category() -> Option<TagCategory> {
        Some(TagCategory::Shape)
    }
}

#[derive(Component)]
pub struct ShapeRope;

impl CraftingTag for ShapeRope {
    fn id() -> CraftingTagId {
        SHAPE_ROPE_ID
    }

    fn category() -> Option<TagCategory> {
        Some(TagCategory::Shape)
    }
}

//TODO remove
fn build_test_recipe() -> CraftingRecipe {
    CraftingRecipe {
        name: "Axe".to_string(),
        ingredients: vec![
            CraftingRecipeIngredient {
                name: "handle".to_string(),
                required: true,
                amount: CraftingIngredientAmount::Items(1),
                tags: vec![
                    TagBounds::Just(TagDescriptor::Tag(SizeMedium::id())),
                    TagBounds::Just(TagDescriptor::Tag(ShapeRod::id())),
                ],
            },
            CraftingRecipeIngredient {
                name: "head".to_string(),
                required: true,
                amount: CraftingIngredientAmount::Items(1),
                tags: vec![
                    TagBounds::Just(TagDescriptor::Tag(SizeMedium::id())),
                    TagBounds::Just(TagDescriptor::Category(TagCategory::Sharpness)),
                ],
            },
            CraftingRecipeIngredient {
                name: "rope".to_string(),
                required: true,
                amount: CraftingIngredientAmount::Items(1),
                tags: vec![
                    TagBounds::Just(TagDescriptor::Tag(SizeSmall::id())),
                    TagBounds::Just(TagDescriptor::Tag(ShapeRope::id())),
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
