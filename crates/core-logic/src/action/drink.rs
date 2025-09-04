use std::{
    collections::{HashMap, HashSet},
    sync::LazyLock,
};

use bevy_ecs::prelude::*;

use crate::{
    command_format::{
        entity_part_builder, literal_part, optional_literal_part,
        validate_parsed_value_has_component, CommandFormat, CommandPartId,
    },
    component::{
        ActionEndNotification, AfterActionPerformNotification, FluidContainer, FluidType, Volume,
    },
    input_parser::{input_formats_if_has_component, InputParseError, InputParser},
    notification::VerifyResult,
    ActionTag, BasicTokens, BeforeActionNotification, Description, DynamicMessage,
    DynamicMessageLocation, InternalMessageCategory, MessageCategory, MessageDelay, MessageFormat,
    SurroundingsMessageCategory, VerifyActionNotification,
};

use super::{Action, ActionInterruptResult, ActionNotificationSender, ActionResult};

/// The amount of liquid to consume in one drink.
const LITERS_PER_DRINK: Volume = Volume(0.25);

static TARGET_PART_ID: LazyLock<CommandPartId<Entity>> =
    LazyLock::new(|| CommandPartId::new("target"));
static DRINK_FORMAT: LazyLock<CommandFormat> = LazyLock::new(|| {
    CommandFormat::new(literal_part("drink"))
        .then(literal_part(" ").always_include_in_errors())
        .then(optional_literal_part("from "))
        .then(
            entity_part_builder(TARGET_PART_ID.clone())
                .with_validator(|context, world| {
                    validate_parsed_value_has_component::<FluidContainer>(
                        context,
                        "drink from",
                        world,
                    )
                })
                .build()
                .always_include_in_errors()
                .with_if_unparsed("what")
                .with_placeholder_for_format_string("container"),
        )
});

pub struct DrinkParser;

impl InputParser for DrinkParser {
    fn parse(
        &self,
        input: &str,
        source_entity: Entity,
        world: &World,
    ) -> Result<Box<dyn Action>, InputParseError> {
        let parsed = DRINK_FORMAT.parse(input, source_entity, world)?;

        Ok(Box::new(DrinkAction {
            target: parsed.get(&TARGET_PART_ID),
            amount: LITERS_PER_DRINK,
            fluids_to_volume_drank: HashMap::new(),
            notification_sender: ActionNotificationSender::new(),
        }))
    }

    fn get_input_formats(&self) -> Vec<String> {
        vec![DRINK_FORMAT.get_format_description().to_string()]
    }

    fn get_input_formats_for(&self, entity: Entity, _: Entity, world: &World) -> Vec<String> {
        input_formats_if_has_component::<FluidContainer>(
            entity,
            world,
            &[DRINK_FORMAT.get_format_description().with_targeted_entity(
                TARGET_PART_ID.clone(),
                entity,
                world,
            )],
        )
    }
}

/// Makes an entity drink from a fluid container.
#[derive(Debug)]
pub struct DrinkAction {
    pub target: Entity,
    pub amount: Volume,
    pub fluids_to_volume_drank: HashMap<FluidType, Volume>,
    pub notification_sender: ActionNotificationSender<Self>,
}

impl Action for DrinkAction {
    fn perform(&mut self, performing_entity: Entity, world: &mut World) -> ActionResult {
        let target_name =
            Description::get_reference_name(self.target, Some(performing_entity), world);
        let mut container = match world.get_mut::<FluidContainer>(self.target) {
            Some(s) => s,
            None => {
                return ActionResult::error(
                    performing_entity,
                    format!("You can't drink from {target_name}."),
                );
            }
        };

        let used_volume = container.contents.get_total_volume();
        if used_volume <= Volume(0.0) {
            return ActionResult::error(performing_entity, format!("{target_name} is empty."));
        }

        self.amount = Volume(used_volume.0.min(self.amount.0));

        self.fluids_to_volume_drank = container.contents.reduce(self.amount);

        ActionResult::builder()
            .with_message(
                performing_entity,
                format!("You take a drink from {target_name}."),
                MessageCategory::Internal(InternalMessageCategory::Action),
                MessageDelay::Short,
            )
            .with_dynamic_message(
                Some(performing_entity),
                DynamicMessageLocation::SourceEntity,
                DynamicMessage::new_third_person(
                    MessageCategory::Surroundings(SurroundingsMessageCategory::Action),
                    MessageDelay::Short,
                    MessageFormat::new(
                        "${performing_entity.Name} takes a drink from ${target.name}.",
                    )
                    .expect("message format should be valid"),
                    BasicTokens::new()
                        .with_entity("performing_entity".into(), performing_entity)
                        .with_entity("target".into(), self.target),
                ),
                world,
            )
            .build_complete_should_tick(true)
    }

    fn interrupt(&self, performing_entity: Entity, _: &mut World) -> ActionInterruptResult {
        ActionInterruptResult::message(
            performing_entity,
            "You stop drinking.".to_string(),
            MessageCategory::Internal(InternalMessageCategory::Action),
            MessageDelay::None,
        )
    }

    fn may_require_tick(&self) -> bool {
        true
    }

    fn get_tags(&self) -> HashSet<ActionTag> {
        [].into()
    }

    fn send_before_notification(
        &self,
        notification_type: BeforeActionNotification,
        world: &mut World,
    ) {
        self.notification_sender
            .send_before_notification(notification_type, self, world);
    }

    fn send_verify_notification(
        &self,
        notification_type: VerifyActionNotification,
        world: &mut World,
    ) -> VerifyResult {
        self.notification_sender
            .send_verify_notification(notification_type, self, world)
    }

    fn send_after_perform_notification(
        &self,
        notification_type: AfterActionPerformNotification,
        world: &mut World,
    ) {
        self.notification_sender
            .send_after_perform_notification(notification_type, self, world);
    }

    fn send_end_notification(&self, notification_type: ActionEndNotification, world: &mut World) {
        self.notification_sender
            .send_end_notification(notification_type, self, world);
    }
}

//TODO auto-equip item to drink from?

//TODO verify that the item to drink from is equipped by the drinker?
