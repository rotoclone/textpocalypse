use crate::{
    action::Action,
    entity::Entity,
    world::{EntityId, World},
    LocationId,
};

pub struct Player {
    pub name: String,
    pub location_id: LocationId,
}

impl Entity for Player {
    fn get_name(&self) -> &str {
        &self.name
    }

    fn get_location_id(&self) -> LocationId {
        self.location_id
    }

    fn set_location_id(&mut self, location_id: LocationId) {
        self.location_id = location_id;
    }

    fn on_tick(&mut self) {
        //TODO increase hunger and stuff
    }

    fn on_command(&self, _: EntityId, _: String, _: &World) -> Option<Box<dyn Action>> {
        None
    }
}
