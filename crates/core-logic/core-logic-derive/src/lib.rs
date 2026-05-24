use proc_macro::TokenStream;

mod action_boilerplate;
use action_boilerplate::*;

mod catalog_boilerplate;
use catalog_boilerplate::*;

#[proc_macro_derive(ActionBoilerplate)]
pub fn action_boilerplate_derive(input: TokenStream) -> TokenStream {
    // Construct a representation of Rust code as a syntax tree
    // that we can manipulate.
    let ast = syn::parse(input).unwrap();

    // Build the trait implementation.
    impl_action_boilerplate(&ast)
}

#[proc_macro_derive(CatalogBoilerplate, attributes(catalog_type))]
pub fn catalog_boilerplate_derive(input: TokenStream) -> TokenStream {
    let ast = syn::parse(input).unwrap();
    impl_catalog_boilerplate(&ast)
}
