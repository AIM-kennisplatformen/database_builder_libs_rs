mod args;
mod attributes;
mod delegation;
mod entity;
mod relation;
mod role;
mod utils;

use proc_macro::TokenStream;

use args::{ModelArgs, TypeDbNameArgs};
use quote::quote;
use syn::{Item, parse_macro_input, parse_quote};

#[proc_macro_attribute]
pub fn typedb_model(attr: TokenStream, item: TokenStream) -> TokenStream {
    let args = parse_macro_input!(attr as ModelArgs);
    let mut item = parse_macro_input!(item as Item);

    attributes::add_model_attributes(&mut item);

    let generated = match args.kind.as_deref() {
        None => quote!(),
        Some("entity") => delegation::generate_enum_delegation(&item, "TypeDbEntity"),
        Some("relation") => delegation::generate_enum_delegation(&item, "TypeDbRelation"),
        Some(_) => {
            return syn::Error::new_spanned(args.kind.unwrap(), "expected `entity` or `relation`")
                .to_compile_error()
                .into();
        }
    };

    quote!(#item #generated).into()
}

#[proc_macro_attribute]
pub fn typedb_entity(attr: TokenStream, item: TokenStream) -> TokenStream {
    let args = parse_macro_input!(attr as TypeDbNameArgs);
    let mut item = parse_macro_input!(item as Item);

    attributes::add_model_attributes(&mut item);
    attributes::add_enum_serde_attributes(&mut item, Some(parse_quote!(content = "attrs")));

    let generated = match entity::generate_impl(&item, args.name.as_ref()) {
        Ok(generated) => generated,
        Err(error) => return error.to_compile_error().into(),
    };

    quote!(#item #generated).into()
}

#[proc_macro_attribute]
pub fn typedb_relation(attr: TokenStream, item: TokenStream) -> TokenStream {
    let args = parse_macro_input!(attr as TypeDbNameArgs);
    let mut item = parse_macro_input!(item as Item);

    attributes::add_model_attributes(&mut item);
    attributes::add_enum_serde_attributes(&mut item, None);
    attributes::add_trait_object_attributes(&mut item);

    let generated = match relation::generate_impl(&item, args.name.as_ref()) {
        Ok(generated) => generated,
        Err(error) => return error.to_compile_error().into(),
    };

    quote!(#item #generated).into()
}

#[proc_macro_attribute]
pub fn typedb_relation_role(attr: TokenStream, item: TokenStream) -> TokenStream {
    role::expand(attr, item)
}
