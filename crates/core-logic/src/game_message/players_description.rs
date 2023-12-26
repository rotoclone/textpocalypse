use bevy_ecs::prelude::*;

use crate::{
    component::{ActionQueue, Description, Player},
    GameOptions,
};

/// A description of all the players on a server.
#[derive(Debug, Clone)]
pub struct PlayersDescription {
    /// Descriptions of the players on the server.
    pub players: Vec<PlayerDescription>,
}

/// A description of a player.
#[derive(Debug, Clone)]
pub struct PlayerDescription {
    /// The name of the player.
    pub name: String,
    /// Whether the player has any actions queued.
    pub has_queued_action: bool,
    /// Whether the player is AFK.
    pub is_afk: bool,
    /// Whether the player is the player that asked for player info.
    pub is_self: bool,
}

impl PlayersDescription {
    /// Creates a players description to be sent to the provided entity.
    ///
    /// `world` needs to be mutable so it can be queried.
    pub fn for_entity(entity: Entity, world: &mut World) -> PlayersDescription {
        PlayersDescription {
            players: build_player_descriptions(entity, world),
        }
    }
}

/// Builds a list of descriptions of players on the server.
///
/// `world` needs to be mutable so it can be queried.
fn build_player_descriptions(pov_entity: Entity, world: &mut World) -> Vec<PlayerDescription> {
    let mut descriptions = Vec::new();

    let mut query = world.query::<(Entity, &ActionQueue, &Description, &Player)>();
    for (entity, queue, desc, player) in query.iter(world) {
        descriptions.push(PlayerDescription {
            name: desc.name.clone(),
            has_queued_action: !queue.is_empty(),
            is_afk: player.is_afk(world.resource::<GameOptions>().afk_timeout),
            is_self: entity == pov_entity,
        });
    }

    descriptions
}
