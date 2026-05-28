use bevy_ecs::prelude::*;

use crate::notification::NotificationHandlers;
use crate::Stat;

mod game_options;
pub use game_options::GameOptions;

mod attribute_name_catalog;
pub use attribute_name_catalog::get_attribute_name;
pub use attribute_name_catalog::AttributeNameCatalog;

mod skill_name_catalog;
pub use skill_name_catalog::get_skill_name;
pub use skill_name_catalog::SkillNameCatalog;

mod action_interaction_handlers;
pub use action_interaction_handlers::*;

pub mod catalog;
use catalog::*;

/// Inserts all the resources into the world.
pub fn insert_resources(world: &mut World) {
    world.insert_resource(FluidNameCatalog::new());
    world.insert_resource(FluidHydrationFactorCatalog::new());
    world.insert_resource(FluidDensityCatalog::new());
    world.insert_resource(AttributeNameCatalog::new());
    world.insert_resource(SkillNameCatalog::new());
    world.insert_resource(SkillBaseAttributeCatalog::new());
    world.insert_resource(WeaponTypeStatCatalog::new());
    world.insert_resource(WeaponTypeNameCatalog::new());
    world.insert_resource(BodyPartTypeNameCatalog::new());
    world.insert_resource(AmmoCaliberNameCatalog::new());
}

/// Registers notification handlers related to resources.
pub fn register_resource_handlers(world: &mut World) {
    NotificationHandlers::add_handler(increase_hydration_on_drink, world);
}

/// Gets the name of a stat. If it's an attribute, the full name will be returned.
pub fn get_stat_name(stat: &Stat, world: &World) -> String {
    match stat {
        Stat::Attribute(attribute) => AttributeNameCatalog::get_name(attribute, world).full,
        Stat::Skill(skill) => SkillNameCatalog::get_name(skill, world),
    }
}
