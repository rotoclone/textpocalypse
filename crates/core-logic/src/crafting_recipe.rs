use std::{
    any::Any,
    collections::HashSet,
    hash::{Hash, Hasher},
};

use bevy_ecs::prelude::*;

use crate::{component::Fluid, Volume, Weight};

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

impl CraftingRecipeIngredient {
    /// Determines whether the provided entity can be used as this ingredient.
    fn matches(&self, entity: Entity, world: &World) -> bool {
        self.amount.matches(entity, world)
            && self
                .tags
                .iter()
                .all(|tag_bounds| tag_bounds.matches(entity, world))
    }
}

/// Describes the tags to check for on an entity.
pub enum TagBounds {
    /// At least this tag must be present
    Just(TagDescriptor),
    /// At least one of these tags must be present
    OneOf(HashSet<TagDescriptor>),
    /// All of these tags must be present
    AllOf(HashSet<TagDescriptor>),
    /// None of these tags can be present
    NoneOf(HashSet<TagDescriptor>),
}

impl TagBounds {
    /// Determines whether the provided entity is within these bounds.
    fn matches(&self, entity: Entity, world: &World) -> bool {
        match self {
            TagBounds::Just(d) => d.matches(entity, world),
            TagBounds::OneOf(descriptors) => descriptors.iter().any(|d| d.matches(entity, world)),
            TagBounds::AllOf(descriptors) => descriptors.iter().all(|d| d.matches(entity, world)),
            TagBounds::NoneOf(descriptors) => !descriptors.iter().any(|d| d.matches(entity, world)),
        }
    }
}

/// References a specific tag or tag category.
#[derive(Hash)]
pub enum TagDescriptor {
    Category(TagCategory),
    Tag(Box<dyn ItemTag>),
}

impl TagDescriptor {
    /// Determines whether the provided entity has a tag matching this descriptor
    fn matches(&self, entity: Entity, world: &World) -> bool {
        match self {
            TagDescriptor::Category(category) => {
                ItemTags::has_tag_with_category(entity, category, world)
            }
            TagDescriptor::Tag(tag) => ItemTags::has_tag(entity, tag.as_ref(), world),
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

impl CraftingIngredientAmount {
    /// Determines whether the provided entity has at least this amount.
    fn matches(&self, entity: Entity, world: &World) -> bool {
        match self {
            CraftingIngredientAmount::Items(n) => *n == 1, //TODO handle multiple items
            CraftingIngredientAmount::Weight(w) => *w == Weight::get(entity, world),
            CraftingIngredientAmount::Fluid(v) => {
                *v == Volume::get(entity, world) && world.get::<Fluid>(entity).is_some()
            }
        }
    }
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

#[derive(Component, Default)]
pub struct ItemTags(HashSet<Box<dyn ItemTag>>);

impl ItemTags {
    /// Adds a tag to an entity.
    pub fn add_to<T: ItemTag>(entity: Entity, tag: T, world: &mut World) {
        ensure_has_component_and::<ItemTags>(entity, |c| c.add(tag), world);
    }

    /// Adds multiple tags to an entity.
    pub fn add_all_to(entity: Entity, tags: HashSet<Box<dyn ItemTag>>, world: &mut World) {
        ensure_has_component_and::<ItemTags>(
            entity,
            |c| {
                c.0.extend(tags);
            },
            world,
        );
    }

    /// Removes a tag from an entity.
    pub fn remove_from<T: ItemTag>(entity: Entity, tag: &dyn ItemTag, world: &mut World) {
        ensure_has_component_and::<ItemTags>(entity, |c| c.remove(tag), world);
    }

    /// Removes multiple tags from an entity.
    pub fn remove_all_from(entity: Entity, tags: &[&dyn ItemTag], world: &mut World) {
        ensure_has_component_and::<ItemTags>(
            entity,
            |c| {
                for tag in tags {
                    c.remove(*tag);
                }
            },
            world,
        );
    }

    /// Adds a tag.
    pub fn add<T: ItemTag>(&mut self, tag: T) {
        self.0.insert(Box::new(tag));
    }

    /// Removes a tag.
    pub fn remove(&mut self, tag: &dyn ItemTag) {
        self.0.remove(tag);
    }

    /// Determines whether the provided entity has the provided tag.
    pub fn has_tag(entity: Entity, tag: &dyn ItemTag, world: &World) -> bool {
        world
            .get::<ItemTags>(entity)
            .is_some_and(|tags| tags.0.contains(tag))
    }

    /// Determines whether the provided entity has any tag with the provided category.
    pub fn has_tag_with_category(entity: Entity, category: &TagCategory, world: &World) -> bool {
        world.get::<ItemTags>(entity).is_some_and(|tags| {
            tags.0
                .iter()
                .any(|tag| tag.category().as_ref() == Some(category))
        })
    }
}

// TODO move this to a common place
/// If the entity has the component, passes it to `f`. Otherwise, makes a new default version of the component, passes it to `f`, and adds it to the entity.
fn ensure_has_component_and<C: Component + Default>(
    entity: Entity,
    f: impl FnOnce(&mut C),
    world: &mut World,
) {
    if let Some(mut component) = world.get_mut::<C>(entity) {
        f(&mut component)
    } else {
        let mut component = C::default();
        f(&mut component);
        world.entity_mut(entity).insert(component);
    }
}

//TODO this should also go isomewhere else
/// Trait for item tags.
/// Equality is based solely on types, so item tag structs should not have any fields.
pub trait ItemTag: AsAny + Send + Sync + 'static {
    /// Gets the category of this tag, if it has one
    fn category(&self) -> Option<TagCategory>;
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

//TODO these tag structs should go somewhere else

pub struct SizeSmall;

//TODO make a proc macro to auto-derive this
impl ItemTag for SizeSmall {
    fn category(&self) -> Option<TagCategory> {
        Some(TagCategory::Size)
    }
}

pub struct SizeMedium;

impl ItemTag for SizeMedium {
    fn category(&self) -> Option<TagCategory> {
        Some(TagCategory::Size)
    }
}

pub struct SizeLarge;

impl ItemTag for SizeLarge {
    fn category(&self) -> Option<TagCategory> {
        Some(TagCategory::Size)
    }
}

pub struct ShapeRod;

impl ItemTag for ShapeRod {
    fn category(&self) -> Option<TagCategory> {
        Some(TagCategory::Shape)
    }
}

pub struct ShapeRope;

impl ItemTag for ShapeRope {
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
