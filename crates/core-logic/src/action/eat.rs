use std::{collections::HashSet, sync::LazyLock};

use bevy_ecs::prelude::*;

use crate::{
    command_format::{
        entity_part_with_validator, literal_part, CommandFormat, CommandParseError, CommandPartId,
        CommandPartValidateError, CommandPartValidateResult, PartValidatorContext,
    },
    component::{ActionEndNotification, AfterActionPerformNotification, Edible},
    despawn_entity,
    input_parser::{input_formats_if_has_component, InputParser},
    notification::VerifyResult,
    ActionTag, BasicTokens, BeforeActionNotification, Description, DynamicMessage,
    DynamicMessageLocation, InternalMessageCategory, MessageCategory, MessageDelay, MessageFormat,
    SurroundingsMessageCategory, VerifyActionNotification,
};

use super::{Action, ActionInterruptResult, ActionNotificationSender, ActionResult};

static TARGET_PART_ID: LazyLock<CommandPartId<Entity>> =
    LazyLock::new(|| CommandPartId::new("target"));
static EAT_FORMAT: LazyLock<CommandFormat> = LazyLock::new(|| {
    CommandFormat::new(literal_part("eat"))
        .then(literal_part(" "))
        .then(entity_part_with_validator(
            TARGET_PART_ID.clone(),
            validate_target,
        ))
});

/// Validates that an entity could be eaten.
fn validate_target(
    context: PartValidatorContext<Entity>,
    world: &World,
) -> CommandPartValidateResult {
    if world.get::<Edible>(context.parsed_value).is_some() {
        CommandPartValidateResult::Valid
    } else {
        let target_name = Description::get_reference_name(
            context.parsed_value,
            Some(context.performing_entity),
            world,
        );
        CommandPartValidateResult::Invalid(CommandPartValidateError {
            details: Some(format!("You can't eat {target_name}.")),
        })
    }
}

pub struct EatParser;

impl InputParser for EatParser {
    fn parse(
        &self,
        input: &str,
        source_entity: Entity,
        world: &World,
    ) -> Result<Box<dyn Action>, CommandParseError> {
        let parsed = EAT_FORMAT.parse(input, source_entity, world)?;

        Ok(Box::new(EatAction {
            target: parsed.get(&TARGET_PART_ID),
            notification_sender: ActionNotificationSender::new(),
        }))
    }

    fn get_input_formats(&self) -> Vec<String> {
        vec![EAT_FORMAT.get_format_description().to_string()]
    }

    fn get_input_formats_for(&self, entity: Entity, _: Entity, world: &World) -> Vec<String> {
        input_formats_if_has_component::<Edible>(
            entity,
            world,
            &[EAT_FORMAT.get_format_description().with_targeted_entity(
                TARGET_PART_ID.clone(),
                entity,
                world,
            )],
        )
    }
}

/// Makes an entity eat something.
#[derive(Debug)]
pub struct EatAction {
    pub target: Entity,
    pub notification_sender: ActionNotificationSender<Self>,
}

impl Action for EatAction {
    fn perform(&mut self, performing_entity: Entity, world: &mut World) -> ActionResult {
        let target = self.target;
        let target_name = Description::get_reference_name(target, Some(performing_entity), world);

        ActionResult::builder()
            .with_message(
                performing_entity,
                format!("You eat {target_name}."),
                MessageCategory::Internal(InternalMessageCategory::Action),
                MessageDelay::Short,
            )
            .with_dynamic_message(
                Some(performing_entity),
                DynamicMessageLocation::SourceEntity,
                DynamicMessage::new_third_person(
                    MessageCategory::Surroundings(SurroundingsMessageCategory::Action),
                    MessageDelay::Short,
                    MessageFormat::new("${performing_entity.Name} eats ${target.name}.")
                        .expect("message format should be valid"),
                    BasicTokens::new()
                        .with_entity("performing_entity".into(), performing_entity)
                        .with_entity("target".into(), self.target),
                ),
                world,
            )
            .with_post_effect(Box::new(move |w| despawn_entity(target, w)))
            .build_complete_should_tick(true)
    }

    fn interrupt(&self, performing_entity: Entity, _: &mut World) -> ActionInterruptResult {
        ActionInterruptResult::message(
            performing_entity,
            "You stop eating.".to_string(),
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

//TODO auto-equip item to eat?

//TODO verify that the item to eat is equipped by the eater?
