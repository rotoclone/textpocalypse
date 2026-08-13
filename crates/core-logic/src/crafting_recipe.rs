use std::collections::HashSet;

use bevy_ecs::prelude::*;

use crate::{Volume, Weight};

pub struct CraftingRecipe {
    name: String,
    components: Vec<CraftingRecipeComponent>,
}

pub struct CraftingRecipeComponent {
    name: String,
    //TODO allow grouping components, i.e. "you have to provide at least one of these but not all of them"
    required: bool,
    amount: CraftingComponentAmount,
    tags: Vec<TagBounds>,
}

pub enum TagBounds {
    Just(TagDescriptor),
    OneOf(HashSet<TagDescriptor>),
    AllOf(HashSet<TagDescriptor>),
    NotAnyOf(HashSet<TagDescriptor>),
}

#[derive(PartialEq, Eq, Hash)]
pub enum TagDescriptor {
    Category(TagCategory),
    Tag(CraftingTagId),
}

/// The amount of a component needed for a crafting recipe.
pub enum CraftingComponentAmount {
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

#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub struct CraftingTagId(&'static str);

//TODO this should also go isomewhere else
pub trait CraftingTag: Component {
    /// Gets the ID of this tag
    fn id() -> CraftingTagId;

    /// Gets the category of this tag
    /// TODO should this return an `Option` to support category-less tags, like tagging something as an adhesive?
    fn category() -> TagCategory;
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

    fn category() -> TagCategory {
        TagCategory::Size
    }
}

#[derive(Component)]
pub struct SizeMedium;

impl CraftingTag for SizeMedium {
    fn id() -> CraftingTagId {
        SIZE_MEDIUM_ID
    }

    fn category() -> TagCategory {
        TagCategory::Size
    }
}

#[derive(Component)]
pub struct SizeLarge;

impl CraftingTag for SizeLarge {
    fn id() -> CraftingTagId {
        SIZE_LARGE_ID
    }

    fn category() -> TagCategory {
        TagCategory::Size
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

    fn category() -> TagCategory {
        TagCategory::Shape
    }
}

#[derive(Component)]
pub struct ShapeRope;

impl CraftingTag for ShapeRope {
    fn id() -> CraftingTagId {
        SHAPE_ROPE_ID
    }

    fn category() -> TagCategory {
        TagCategory::Shape
    }
}

//TODO remove
fn build_test_recipe() -> CraftingRecipe {
    CraftingRecipe {
        name: "Axe".to_string(),
        components: vec![
            CraftingRecipeComponent {
                name: "handle".to_string(),
                required: true,
                amount: CraftingComponentAmount::Items(1),
                tags: vec![
                    TagBounds::Just(TagDescriptor::Tag(SizeMedium::id())),
                    TagBounds::Just(TagDescriptor::Tag(ShapeRod::id())),
                ],
            },
            CraftingRecipeComponent {
                name: "head".to_string(),
                required: true,
                amount: CraftingComponentAmount::Items(1),
                tags: vec![
                    TagBounds::Just(TagDescriptor::Tag(SizeMedium::id())),
                    TagBounds::Just(TagDescriptor::Category(TagCategory::Sharpness)),
                ],
            },
            CraftingRecipeComponent {
                name: "rope".to_string(),
                required: true,
                amount: CraftingComponentAmount::Items(1),
                tags: vec![
                    TagBounds::Just(TagDescriptor::Tag(SizeSmall::id())),
                    TagBounds::Just(TagDescriptor::Tag(ShapeRope::id())),
                ],
            },
        ],
    }
}
