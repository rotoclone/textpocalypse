use crate::{Direction, PlayerId, World};

pub enum Action {
    Move(Direction),
    Look,
}

pub struct ActionResult {
    pub description: String,
    pub should_tick: bool,
}

pub fn perform_action(action: Action, player_id: PlayerId, world: &mut World) -> ActionResult {
    let player = world.get_player(player_id);

    match action {
        Action::Move(dir) => {
            let current_location = world.get_location(player.location_id);

            if let Some(new_location_id) = current_location.connections.get(&dir) {
                let new_location = world.get_location(*new_location_id);
                let result = ActionResult {
                    description: format!("You move to {}.", new_location.name),
                    should_tick: true,
                };

                world.get_player_mut(player_id).location_id = *new_location_id;

                result
            } else {
                ActionResult {
                    description: "You can't move in that direction.".to_string(),
                    should_tick: false,
                }
            }
        }
        Action::Look => ActionResult {
            description: format!(
                "You are at {}.",
                world.get_state(player_id).location_desc.name
            ),
            should_tick: false,
        },
    }
}
