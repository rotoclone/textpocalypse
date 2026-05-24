use proc_macro::TokenStream;
use quote::quote;
use syn::Ident;

/// Creates a token stream containing an implementation of `CatalogBoilerplate`
pub fn impl_catalog_boilerplate(ast: &syn::DeriveInput) -> TokenStream {
    let attribute = ast
        .attrs
        .iter()
        .find(|a| a.path().segments.len() == 1 && a.path().segments[0].ident == "catalog_type")
        .expect("catalog_type attribute required to derive CatalogBoilerplate: #[catalog_type(T)]");

    let attribute_tokens = attribute
        .meta
        .require_list()
        .map(|list| list.tokens.clone())
        .expect("Invalid catalog_type attribute");

    let thing_type: Ident = syn::parse2(attribute_tokens).expect("Invalid catalog_type attribute");

    let name = &ast.ident;
    let generated = quote! {
        impl crate::resource::CatalogBoilerplate<#thing_type> for #name {
            fn new() -> Self {
                let standard = <#thing_type as strum::IntoEnumIterator>::iter()
                    .filter_map(|thing| Self::get_default_value(&thing).map(|value| (thing, value)))
                    .collect();

                Self {
                    standard,
                    custom: std::collections::HashMap::new(),
                }
            }

            fn get_value(thing: &#thing_type, world: &bevy_ecs::world::World) -> Self::V {
                world.resource::<#name>().get(thing)
            }

            fn set(&mut self, thing: &#thing_type, value: Self::V) {
                match thing {
                    #thing_type::Custom(id) => self.custom.insert(id.clone(), value),
                    _ => self.standard.insert(thing.clone(), value),
                };
            }

            fn get(&self, thing: &#thing_type) -> Self::V {
                match thing {
                    #thing_type::Custom(id) => self.custom.get(id),
                    _ => self.standard.get(thing),
                }
                .cloned()
                .unwrap_or_else(|| Self::get_not_found_value())
            }
        }
    };
    generated.into()
}
