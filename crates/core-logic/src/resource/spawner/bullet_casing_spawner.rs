use bevy_ecs::prelude::*;

use crate::{
    component::{AmmoCaliber, DescribeAttributes, Description, Item, Location, Volume, Weight},
    move_entity,
    resource::catalog::{AmmoCaliberNameCatalog, CatalogBoilerplate},
    Pronouns,
};

/// Spawns bullet casings.
/// TODO make this dynamic somehow
pub struct BulletCasingSpawner;

impl BulletCasingSpawner {
    /// Spawns a bullet casing in the provided location.
    pub fn spawn(caliber: AmmoCaliber, location: Location, world: &mut World) {
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
        move_entity(casing_id, location.id, world);
    }
}
