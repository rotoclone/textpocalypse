use std::time::Duration;

use bevy_ecs::prelude::*;

#[derive(Resource)]
pub struct GameOptions {
    /// How long a player can go without entering a command before they're considered to be AFK and they no longer prevent other players from performing actions that require ticks.
    ///
    /// If not set, players will never be considered AFK.
    pub afk_timeout: Option<Duration>,
}
