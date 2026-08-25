use std::{
    any::Any,
    collections::{HashMap, HashSet},
    hash::{Hash, Hasher},
};

use bevy_ecs::prelude::*;

use crate::{
    component::{
        CombatRange, DescribeAttributes, Description, Fluid, Item, Weapon, WeaponDamageAdjustment,
        WeaponMessages, WeaponRanges, WeaponStatBonuses, WeaponType,
    },
    message_format::MessageFormat,
    Pronouns, Volume, Weight,
};

static EMPTY_VEC: Vec<Entity> = Vec::new();

pub struct CraftingRecipe {
    name: String,
    ingredients: Vec<(CraftingRecipeIngredientId, CraftingRecipeIngredientBounds)>,
    output_spawner: fn(CraftingRecipeOutputContext, &mut World),
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

pub struct CraftingRecipeIngredient {
    name: String,
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

/// An item in the shape of a rod (long, rigid)
pub struct ShapeRod;

impl ItemTag for ShapeRod {
    fn category(&self) -> Option<TagCategory> {
        Some(TagCategory::Shape)
    }
}

/// An item in the shape of rope (long, thin, flexible)
pub struct ShapeRope;

impl ItemTag for ShapeRope {
    fn category(&self) -> Option<TagCategory> {
        Some(TagCategory::Shape)
    }
}

/// An item that is an adhesive, like glue
pub struct Adhesive;

impl ItemTag for Adhesive {
    fn category(&self) -> Option<TagCategory> {
        None
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
        ingredients: vec![
            (
                HANDLE_INGREDIENT_ID,
                CraftingRecipeIngredientBounds::Just {
                    required: true,
                    ingredient: CraftingRecipeIngredient {
                        name: "handle".to_string(),
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
                            amount: CraftingIngredientAmount::Items(1),
                            tags: vec![
                                TagBounds::Just(TagDescriptor::Tag(Box::new(SizeSmall))),
                                TagBounds::Just(TagDescriptor::Tag(Box::new(ShapeRope))),
                            ],
                        },
                        CraftingRecipeIngredient {
                            name: "adhesive".to_string(),
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
                base_damage_range: 10..=15,
                critical_damage_behavior: WeaponDamageAdjustment::Multiply(2.0),
                ranges: WeaponRanges {
                    usable: CombatRange::Shortest..=CombatRange::Short,
                    optimal: CombatRange::Short..=CombatRange::Short,
                    to_hit_penalty: 1,
                    damage_penalty: 4,
                },
                stat_requirements: Vec::new(),
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
        Volume(0.5),
        Weight(2.0)
    ));
}
