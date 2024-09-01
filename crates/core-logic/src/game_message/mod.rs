use strum::EnumIter;

mod entity_description;
pub use entity_description::DetailedEntityDescription;
pub use entity_description::EntityDescription;

mod players_description;
pub use players_description::PlayerDescription;
pub use players_description::PlayersDescription;

mod ranges_description;
pub use ranges_description::RangeDescription;
pub use ranges_description::RangesDescription;
pub use ranges_description::WeaponRangeJudgement;
pub use ranges_description::WeaponRangeJudgementReason;

mod container_description;
pub use container_description::ContainerDescription;
pub use container_description::ContainerEntityCategory;
pub use container_description::ContainerEntityDescription;

mod worn_items_description;
pub use worn_items_description::WornItemDescription;
pub use worn_items_description::WornItemsDescription;

mod vitals_description;
pub use vitals_description::VitalsDescription;

mod stats_description;
pub use stats_description::SkillDescription;
pub use stats_description::StatAttributeDescription;
pub use stats_description::StatsDescription;

mod vital_change_description;
pub use vital_change_description::VitalChangeDescription;

mod vital_change_short_description;
pub use vital_change_short_description::VitalChangeShortDescription;

mod action_description;
pub use action_description::ActionDescription;

mod room_description;
pub use room_description::ExitDescription;
pub use room_description::RoomConnectionEntityDescription;
pub use room_description::RoomDescription;
pub use room_description::RoomEntityDescription;
pub use room_description::RoomLivingEntityDescription;
pub use room_description::RoomObjectDescription;

mod map_description;
pub use map_description::MapDescription;

mod help_description;
pub use help_description::HelpDescription;

use crate::AdvancementPointType;

/// Resolution of the visualization for short vital change messages.
const SHORT_VITAL_CHANGE_RESOLUTION: u8 = 10;

/// A message from the game, such as the description of a location, a message describing the results of an action, etc.
#[derive(Debug, Clone)]
pub enum GameMessage {
    Room(RoomDescription),
    Entity(EntityDescription),
    DetailedEntity(DetailedEntityDescription),
    Container(ContainerDescription),
    WornItems(WornItemsDescription),
    Vitals(VitalsDescription),
    Stats(StatsDescription),
    Help(HelpDescription),
    Players(PlayersDescription),
    Ranges(RangesDescription),
    AdvancementPointsGained(u32, AdvancementPointType),
    Message {
        content: String,
        category: MessageCategory,
        delay: MessageDelay,
        decorations: Vec<MessageDecoration>,
    },
    Error(String),
}

/// The category of a game message.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum MessageCategory {
    /// A message from an entity's surroundings.
    Surroundings(SurroundingsMessageCategory),
    /// A message from the entity itself.
    Internal(InternalMessageCategory),
    /// A message from the game itself, as opposed to the game world.
    System,
}

/// A message from an entity's surroundings.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, EnumIter)]
pub enum SurroundingsMessageCategory {
    /// Someone saying something.
    Speech,
    /// A non-speech sound.
    Sound,
    /// Messages that are just for flavor, like describing wind whistling through the trees.
    Flavor,
    /// Someone entering or leaving the room.
    Movement,
    /// Someone performing a non-movement action.
    Action,
}

/// A message from the entity itself.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, EnumIter)]
pub enum InternalMessageCategory {
    /// The entity saying something.
    Speech,
    /// A description of an action being performed.
    Action,
    /// A miscellaneous message, perhaps just to provide context to another message.
    Misc,
}

impl MessageCategory {
    /// Gets the internal category that corresponds to this category (internal categories return themselves)
    pub fn into_internal(self) -> MessageCategory {
        let category = match self {
            MessageCategory::Surroundings(s) => match s {
                SurroundingsMessageCategory::Speech => InternalMessageCategory::Speech,
                SurroundingsMessageCategory::Sound => InternalMessageCategory::Misc,
                SurroundingsMessageCategory::Flavor => InternalMessageCategory::Misc,
                SurroundingsMessageCategory::Movement => InternalMessageCategory::Action,
                SurroundingsMessageCategory::Action => InternalMessageCategory::Action,
            },
            MessageCategory::Internal(i) => i,
            MessageCategory::System => InternalMessageCategory::Misc,
        };

        MessageCategory::Internal(category)
    }
}

/// The amount of time to wait before any additional messages are displayed.
#[derive(Debug, Clone, Copy)]
pub enum MessageDelay {
    /// No time should be waited.
    None,
    /// A short amount of time should be waited.
    Short,
    /// A long amount of time should be waited.
    Long,
}

/// Additional bits of information that can be included with messages.
#[derive(Debug, Clone)]
pub enum MessageDecoration {
    /// A description of a change to an entity's vitals.
    VitalChange(VitalChangeDescription),
    /// A short description of a change to an entity's vitals.
    ShortVitalChange(VitalChangeShortDescription<SHORT_VITAL_CHANGE_RESOLUTION>),
}
