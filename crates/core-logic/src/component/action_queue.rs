use std::collections::VecDeque;

use bevy_ecs::prelude::*;

use crate::action::Action;

#[derive(Component)]
pub struct ActionQueue {
    pub actions: VecDeque<Box<dyn Action>>,
}
