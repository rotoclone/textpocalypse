use proc_macro::TokenStream;
use quote::quote;

#[proc_macro_derive(ActionBoilerplate)]
pub fn hello_macro_derive(input: TokenStream) -> TokenStream {
    // Construct a representation of Rust code as a syntax tree
    // that we can manipulate.
    let ast = syn::parse(input).unwrap();

    // Build the trait implementation.
    impl_action_boilerplate(&ast)
}

fn impl_action_boilerplate(ast: &syn::DeriveInput) -> TokenStream {
    let name = &ast.ident;
    let generated = quote! {
        impl crate::action::ActionBoilerplate for #name {
            fn send_before_notification(
                &self,
                notification_type: crate::BeforeActionNotification,
                world: &mut bevy_ecs::world::World,
            ) {
                self.notification_sender
                    .send_before_notification(notification_type, self, world);
            }

            fn send_verify_notification(
                &self,
                notification_type: crate::VerifyActionNotification,
                world: &mut bevy_ecs::world::World,
            ) -> Vec<crate::component::VerifyResult> {
                self.notification_sender
                    .send_verify_notification(notification_type, self, world)
            }

            fn send_after_perform_notification(
                &self,
                notification_type: crate::AfterActionPerformNotification,
                world: &mut bevy_ecs::world::World,
            ) {
                self.notification_sender
                    .send_after_perform_notification(notification_type, self, world);
            }

            fn send_end_notification(&self, notification_type: crate::ActionEndNotification, world: &mut bevy_ecs::world::World) {
                self.notification_sender
                    .send_end_notification(notification_type, self, world);
            }

            fn has_interaction_handlers(&self, world: &bevy_ecs::world::World) -> bool {
                world.contains_resource::<crate::resource::ActionInteractionHandlers<Self>>()
            }

            fn try_interact(
                &self,
                performing_entity: bevy_ecs::entity::Entity,
                other_performing_entity: bevy_ecs::entity::Entity,
                other_action: &dyn crate::action::Action,
                world: &mut bevy_ecs::world::World,
            ) -> crate::resource::ActionInteractionResult {
                let core::option::Option::Some(handlers) = world
                    .get_resource::<crate::resource::ActionInteractionHandlers<Self>>()
                    .cloned()
                else {
                    return crate::resource::ActionInteractionResult::DidNotInteract;
                };

                for handler in handlers.handlers {
                    let result = handler(
                        crate::resource::ActionInteractionContext {
                            performing_entity_1: performing_entity,
                            action_1: self,
                            performing_entity_2: other_performing_entity,
                            action_2: other_action,
                        },
                        world,
                    );

                    if result.interacted() {
                        return result;
                    }
                }

                crate::resource::ActionInteractionResult::DidNotInteract
            }
        }
    };
    generated.into()
}
