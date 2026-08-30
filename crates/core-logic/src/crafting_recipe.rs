use std::{
    collections::{HashMap, HashSet},
    hash::Hash,
};

use bevy_ecs::prelude::*;
use rand_distr::num_traits::ToPrimitive;

use crate::{
    component::{
        Attribute, CombatRange, DescribeAttributes, Description, Fluid, Item, ItemTags, Stat,
        Weapon, WeaponDamageAdjustment, WeaponMessages, WeaponPerformanceAdjustment, WeaponRanges,
        WeaponStatBonuses, WeaponStatRequirement, WeaponStatRequirementNotMetBehavior,
        WeaponToHitAdjustment, WeaponType,
    },
    item_tag::{
        shape::{ShapeRod, ShapeRope},
        size::{SizeMedium, SizeSmall},
        Adhesive, ItemTag, TagCategory,
    },
    message_format::MessageFormat,
    Pronouns, Volume, Weight,
};

static EMPTY_VEC: Vec<Entity> = Vec::new();

/// A recipe used to craft something.
pub struct CraftingRecipe {
    /// The display name of the recipe
    name: String,
    /// The category of thing the recipe makes
    category: CraftingRecipeCategory,
    /// The ingredients of the recipe
    ingredients: Vec<(CraftingRecipeIngredientId, CraftingRecipeIngredientBounds)>,
    /// Function to spawn the crafted item
    output_spawner: fn(CraftingRecipeOutputContext, &mut World),
}

/// TODO just use ContainerEntityCategory instead?
#[derive(Clone, PartialEq, Eq, Hash)]
pub enum CraftingRecipeCategory {
    Weapon,
    Wearable,
    Consumable,
    Container,
    Other,
    Custom(String),
}

pub struct CraftingRecipeOutputContext {
    crafting_entity: Entity,
    ingredients: HashMap<CraftingRecipeIngredientId, CraftingRecipeIngredientBounds>,
    used_ingredients: HashMap<CraftingRecipeIngredientId, Vec<Entity>>,
}

impl CraftingRecipeOutputContext {
    /// Gets the entity or entities used for an ingredient.
    pub fn get_used(&self, id: CraftingRecipeIngredientId) -> &Vec<Entity> {
        self.used_ingredients.get(&id).unwrap_or(&EMPTY_VEC)
    }

    /// Gets the entity used for an ingredient, if any.
    ///
    /// # Panics
    /// Panics if more than one entity was used for the ingredient.
    pub fn get_used_single(&self, id: CraftingRecipeIngredientId) -> Option<Entity> {
        let entities = self.get_used(id);
        if entities.len() > 1 {
            panic!(
                "Expected no more than 1 entity to be used for {:?}, but found {}",
                id,
                entities.len()
            );
        }

        entities.first().copied()
    }

    /// Gets the entity used for an ingredient.
    ///
    /// # Panics
    /// Panics if no entity was used for the ingredient.
    pub fn get_used_single_required(&self, id: CraftingRecipeIngredientId) -> Entity {
        self.get_used_single(id)
            .unwrap_or_else(|| panic!("{id:?} should have an entity used for it"))
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub struct CraftingRecipeIngredientId(&'static str);

/// Describes one or more ingredients used to fulfill part of a crafting recipe
enum CraftingRecipeIngredientBounds {
    /// One specific ingredient
    Just {
        required: bool,
        ingredient: CraftingRecipeIngredient,
    },
    /// One of many possible ingredients
    OneOf {
        required: bool,
        ingredients: Vec<CraftingRecipeIngredient>,
    },
}

/// An ingredient in a crafting recipe.
pub struct CraftingRecipeIngredient {
    /// The display name of the ingredient
    name: String,
    /// Whether the ingredient is consumed when the recipe is crafted
    consumed: bool,
    /// The amount of the ingredient needed
    amount: CraftingIngredientAmount,
    /// The tags describing the ingredient
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

/// Describes one or more tags an entity should or should not have.
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

static HANDLE_INGREDIENT_ID: CraftingRecipeIngredientId = CraftingRecipeIngredientId("handle");
static HEAD_INGREDIENT_ID: CraftingRecipeIngredientId = CraftingRecipeIngredientId("head");
static CONNECTOR_INGREDIENT_ID: CraftingRecipeIngredientId =
    CraftingRecipeIngredientId("connector");

//TODO remove
#[allow(unused)]
fn build_test_recipe() -> CraftingRecipe {
    CraftingRecipe {
        name: "Axe".to_string(),
        category: CraftingRecipeCategory::Weapon,
        ingredients: vec![
            (
                HANDLE_INGREDIENT_ID,
                CraftingRecipeIngredientBounds::Just {
                    required: true,
                    ingredient: CraftingRecipeIngredient {
                        name: "handle".to_string(),
                        consumed: true,
                        amount: CraftingIngredientAmount::Items(1),
                        tags: vec![
                            TagBounds::Just(TagDescriptor::Tag(Box::new(SizeMedium))),
                            TagBounds::Just(TagDescriptor::Tag(Box::new(ShapeRod))),
                        ],
                    },
                },
            ),
            (
                HEAD_INGREDIENT_ID,
                CraftingRecipeIngredientBounds::Just {
                    required: true,
                    ingredient: CraftingRecipeIngredient {
                        name: "head".to_string(),
                        consumed: true,
                        amount: CraftingIngredientAmount::Items(1),
                        tags: vec![
                            TagBounds::Just(TagDescriptor::Tag(Box::new(SizeMedium))),
                            TagBounds::Just(TagDescriptor::Category(TagCategory::Sharpness)),
                        ],
                    },
                },
            ),
            (
                CONNECTOR_INGREDIENT_ID,
                CraftingRecipeIngredientBounds::OneOf {
                    required: true,
                    ingredients: vec![
                        CraftingRecipeIngredient {
                            name: "rope".to_string(),
                            consumed: true,
                            amount: CraftingIngredientAmount::Items(1),
                            tags: vec![
                                TagBounds::Just(TagDescriptor::Tag(Box::new(SizeSmall))),
                                TagBounds::Just(TagDescriptor::Tag(Box::new(ShapeRope))),
                            ],
                        },
                        CraftingRecipeIngredient {
                            name: "adhesive".to_string(),
                            consumed: true,
                            amount: CraftingIngredientAmount::Items(1),
                            tags: vec![TagBounds::Just(TagDescriptor::Tag(Box::new(Adhesive)))],
                        },
                    ],
                },
            ),
        ],
        output_spawner: spawn_crafted_axe,
    }
}

fn spawn_crafted_axe(context: CraftingRecipeOutputContext, world: &mut World) {
    let handle_entity = context.get_used_single_required(HANDLE_INGREDIENT_ID);
    let head_entity = context.get_used_single_required(HEAD_INGREDIENT_ID);
    let connector_entity = context.get_used_single_required(CONNECTOR_INGREDIENT_ID);

    let name = Description::get_name(head_entity, world)
        .map_or_else(|| "axe".to_string(), |head_name| format!("{head_name} axe"));

    let head_description = world.get::<Description>(head_entity);
    let head_weight = Weight::get(head_entity, world);

    let volume = Volume::get(handle_entity, world) + Volume::get(head_entity, world);
    let weight =
        Weight::get(handle_entity, world) + head_weight + Weight::get(connector_entity, world);

    let stat_requirements = if head_weight < Weight(2.0) {
        Vec::new()
    } else {
        vec![WeaponStatRequirement {
            stat: Stat::Attribute(Attribute::Strength),
            min: head_weight.0 * 4.0,
            below_min_behavior: WeaponStatRequirementNotMetBehavior::AdjustmentsPerPointBelowMin(
                vec![WeaponPerformanceAdjustment::ToHit(
                    WeaponToHitAdjustment::Add(-1),
                )],
            ),
        }]
    };

    let low_damage_bound = (head_weight.0 * 5.0)
        .to_u32()
        .expect("low damage bound should be convertable to u32");
    let high_damage_bound = (head_weight.0 * 7.0)
        .to_u32()
        .expect("high damage bound should be convertable to u32");

    world.spawn((
        Description {
            name: name.clone(),
            room_name: name.clone(),
            plural_name: format!("{name} axes"),
            indefinite_article: Some(
                head_description
                    .and_then(|d| d.indefinite_article.clone())
                    .unwrap_or_else(|| "an".to_string()),
            ),
            pronouns: Pronouns::it(),
            aliases: vec!["axe".to_string()],
            description: format!(
                "An axe with {} as its head attached to {} with {}.",
                Description::get_article_reference_name(head_entity, world),
                Description::get_article_reference_name(handle_entity, world),
                Description::get_article_reference_name(connector_entity, world)
            ),
            attribute_describers: vec![
                Item::get_attribute_describer(),
                Volume::get_attribute_describer(),
                Weight::get_attribute_describer(),
                Weapon::get_attribute_describer(),
            ],
        },
        Item::new_two_handed(),
        Weapon {
                weapon_type: WeaponType::Blade,
                base_damage_range: low_damage_bound..=high_damage_bound,
                critical_damage_behavior: WeaponDamageAdjustment::Multiply(2.0),
                ranges: WeaponRanges {
                    usable: CombatRange::Shortest..=CombatRange::Short,
                    optimal: CombatRange::Short..=CombatRange::Short,
                    to_hit_penalty: 1,
                    damage_penalty: 4,
                },
                stat_requirements,
                stat_bonuses: WeaponStatBonuses {
                    damage_bonus_stat_range: 10.0..=20.0,
                    damage_bonus_per_stat_point: 1.0,
                    to_hit_bonus_stat_range: 10.0..=20.0,
                    to_hit_bonus_per_stat_point: 1.0,
                },
                default_attack_messages: WeaponMessages {
                    miss: vec![MessageFormat::new("${attacker.Name} ${attacker.you:swing/swings} ${weapon.name} wide of ${target.name}.").expect("message format should be valid")],
                    minor_hit: vec![MessageFormat::new("${attacker.Name} ${attacker.you:swing/swings} ${weapon.name} near ${target.name}, and ${attacker.you:nick/nicks} ${target.them} in the ${body_part.plain_name}.").expect("message format should be valid")],
                    regular_hit: vec![MessageFormat::new("${attacker.Name} ${attacker.you:catch/catches} ${target.name} on the ${body_part.plain_name} with the blade of ${weapon.name}.").expect("message format should be valid")],
                    major_hit: vec![MessageFormat::new("${attacker.Name} ${attacker.you:bury/buries} the head of ${weapon.name} into ${target.name's} ${body_part.plain_name}.").expect("message format should be valid")],
                    self_hit: vec![MessageFormat::new("${attacker.Name} ${attacker.you:slice/slices} ${attacker.themself} in the ${body_part.plain_name} with ${weapon.name}.").expect("message format should be valid")]
                },
            },
        volume,
        weight,
    ));
}
