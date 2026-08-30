use std::collections::HashSet;

use bevy_ecs::prelude::*;

use crate::item_tag::{ItemTag, TagCategory};

/// The tags assigned to an item.
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
