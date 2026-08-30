use bevy_ecs::prelude::*;

use crate::crafting_recipe::CraftingRecipe;

/// The crafting recipes known by an entity.
#[derive(Component)]
pub struct KnownCraftingRecipes(Vec<CraftingRecipe>);
