use std::collections::HashSet;

use bevy_ecs::prelude::*;

use crate::{Volume, Weight};

pub struct CraftingRecipe {
    components: Vec<CraftingRecipeComponent>,
}

pub struct CraftingRecipeComponent {
    required: bool,
    amount: CraftingComponentAmount,
    required_tags: HashSet<dyn CraftingTag>,
    //TODO instead of just disallowing tags, should this be either a set of allowed or a set of disallowed tags? for example, if you want to allow just small and medium items, it's annoying to have to disallow every other size specifically
    disallowed_tags: HashSet<CraftingTag>,
    required_tag_categories: HashSet<CraftingTagCategory>,
    disallowed_tag_categories: HashSet<CraftingTagCategory>,
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
}

//TODO this should also go isomewhere else
pub trait CraftingTag: Component + PartialEq + Eq {
    /// Gets the category of this tag
    fn get_category(&self) -> CraftingTagCategory;
}

#[derive(Component, PartialEq, Eq)]
pub struct SizeSmall;

impl CraftingTag for SizeSmall {
    fn get_category(&self) -> CraftingTagCategory {
        CraftingTagCategory::Size
    }
}
