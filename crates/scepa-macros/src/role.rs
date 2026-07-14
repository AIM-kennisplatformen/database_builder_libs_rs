use proc_macro::TokenStream;
use quote::quote;
use syn::{Item, LitStr, parse::Parse, parse::ParseStream, parse_macro_input, parse_quote};

pub(crate) fn expand(attr: TokenStream, item: TokenStream) -> TokenStream {
    let mut item = parse_macro_input!(item as Item);

    match &mut item {
        Item::Trait(item) => {
            if !attr.is_empty() {
                return syn::Error::new_spanned(
                    proc_macro2::TokenStream::from(attr),
                    "typedb_relation_role attributes on traits do not take arguments",
                )
                .to_compile_error()
                .into();
            }
            item.attrs
                .insert(0, parse_quote!(#[typetag::serde(tag = "type")]));
        }
        Item::Impl(item) => {
            let name = match syn::parse::<RelationRoleArgs>(attr) {
                Ok(args) => args.name,
                Err(error) => return error.to_compile_error().into(),
            };
            item.attrs
                .insert(0, parse_quote!(#[typetag::serde(name = #name)]));
        }
        _ => {
            return syn::Error::new_spanned(
                &item,
                "#[typedb_relation_role] expects a trait or an impl",
            )
            .to_compile_error()
            .into();
        }
    }

    quote!(#item).into()
}

struct RelationRoleArgs {
    name: LitStr,
}

impl Parse for RelationRoleArgs {
    fn parse(input: ParseStream<'_>) -> syn::Result<Self> {
        let name = if input.peek(LitStr) {
            input.parse()?
        } else {
            let ident: syn::Ident = input.parse()?;
            if ident != "name" {
                return Err(syn::Error::new(ident.span(), "expected `name`"));
            }
            input.parse::<syn::Token![=]>()?;
            input.parse()?
        };

        if !input.is_empty() {
            return Err(input.error("unexpected tokens after relation role name"));
        }

        Ok(Self { name })
    }
}
