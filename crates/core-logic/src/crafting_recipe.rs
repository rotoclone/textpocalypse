use std::collections::HashSet;

use bevy_ecs::prelude::*;

use crate::{Volume, Weight};

pub struct CraftingRecipe {
    components: Vec<CraftingRecipeComponent>,
}

pub struct CraftingRecipeComponent {
    required: bool,
    amount: CraftingComponentAmount,
    tags: Vec<TagBounds>,
}

pub enum TagBounds {
    OneOf(HashSet<TagDescriptor>),
    AllOf(HashSet<TagDescriptor>),
    NotAnyOf(HashSet<TagDescriptor>),
}

pub enum TagDescriptor {
    Category(CraftingTagCategory),
    TagId(CraftingTagId),
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
pub enum CraftingTagCategory {
    Size,
    Shape,
    Material,
    Sharpness,
    Concentration,
    Custom(String),
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub struct CraftingTagId(&'static str);

//TODO this should also go isomewhere else
pub trait CraftingTag: Component {
    /// Gets the ID of this tag
    fn get_id() -> CraftingTagId;

    /// Gets the category of this tag
    fn get_category() -> CraftingTagCategory;
}

static SIZE_SMALL_ID: CraftingTagId = CraftingTagId("size_small");

#[derive(Component)]
pub struct SizeSmall;

impl CraftingTag for SizeSmall {
    fn get_id() -> CraftingTagId {
        SIZE_SMALL_ID
    }

    fn get_category() -> CraftingTagCategory {
        CraftingTagCategory::Size
    }
}
