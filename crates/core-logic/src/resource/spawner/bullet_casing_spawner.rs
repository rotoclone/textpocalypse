use bevy_ecs::prelude::*;

use crate::{
    component::{AmmoCaliber, DescribeAttributes, Description, Item, Volume, Weight},
    move_entity,
    resource::{
        catalog::{AmmoCaliberNameCatalog, CatalogBoilerplate},
        spawner::Spawner,
    },
    Pronouns,
};

/// Spawns bullet casings.
#[derive(Resource)]
pub struct BulletCasingSpawner(fn(AmmoCaliber, Entity, &mut World));

impl Spawner for BulletCasingSpawner {
    type Context = AmmoCaliber;

    fn register(world: &mut World) {
        world.insert_resource(BulletCasingSpawner(default_spawn_fn));
    }

    fn spawn(context: Self::Context, location: Entity, world: &mut World) {
        world.resource::<BulletCasingSpawner>().0(context, location, world);
    }
}

/// Default spawn function for bullet casings.
fn default_spawn_fn(caliber: AmmoCaliber, location: Entity, world: &mut World) {
    let caliber_name = AmmoCaliberNameCatalog::get_value(&caliber, world);
    let casing_id = world
        .spawn((
            Description {
                name: format!("{caliber_name} casing"),
                room_name: format!("{caliber_name} casing"),
                plural_name: format!("{caliber_name} casings"),
                article: Some("a".to_string()),
                pronouns: Pronouns::it(),
                aliases: vec!["casing".to_string()],
                description: format!("A empty brass casing for a {caliber_name} bullet."),
                attribute_describers: vec![
                    Item::get_attribute_describer(),
                    Volume::get_attribute_describer(),
                    Weight::get_attribute_describer(),
                ],
            },
            Item::new_one_handed(),
            Volume(0.01),
            Weight(0.01),
        ))
        .id();
    move_entity(casing_id, location, world);
}
