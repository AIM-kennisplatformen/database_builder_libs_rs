use proc_macro2::TokenStream;
use quote::quote;
use syn::{Item, punctuated::Punctuated, token::Comma};

pub(crate) fn generate_enum_delegation(item: &Item, trait_name: &str) -> TokenStream {
    match item {
        Item::Enum(item_enum) => {
            generate_enum_delegation_from_data(&item_enum.ident, &item_enum.variants, trait_name)
                .unwrap_or_else(|error| error.to_compile_error())
        }
        _ => syn::Error::new_spanned(item, "delegating TypeDB models must be enums")
            .to_compile_error(),
    }
}

pub(crate) fn generate_enum_delegation_from_data(
    ident: &syn::Ident,
    variants: &Punctuated<syn::Variant, Comma>,
    trait_name: &str,
) -> syn::Result<TokenStream> {
    let is_entity = match trait_name {
        "TypeDbEntity" => true,
        "TypeDbRelation" => false,
        _ => unreachable!(),
    };

    let variant_idents = variants
        .iter()
        .map(|variant| {
            let syn::Fields::Unnamed(fields) = &variant.fields else {
                return Err(syn::Error::new_spanned(
                    &variant.fields,
                    "TypeDB model variants must contain one value",
                ));
            };
            if fields.unnamed.len() != 1 {
                return Err(syn::Error::new_spanned(
                    fields,
                    "TypeDB model variants must contain one value",
                ));
            }
            Ok(&variant.ident)
        })
        .collect::<syn::Result<Vec<_>>>()?;

    let body = if is_entity {
        let query_arms = variant_idents.iter().map(|variant_ident| {
            quote! {
                Self::#variant_ident(value) =>
                    <_ as crate::models::entities::TypeDbEntity>::typeql_insert_statement(value, variable),
            }
        });
        quote! {
            impl crate::models::entities::TypeDbEntity for #ident {
                fn typeql_insert_statement(&self, variable: &str) -> String {
                    match self {
                        #(#query_arms)*
                    }
                }
            }
        }
    } else {
        let query_arms = variant_idents.iter().map(|variant_ident| {
            quote! {
                Self::#variant_ident(value) =>
                    <_ as crate::models::relations::TypeDbRelation>::typeql_insert_statement(value),
            }
        });
        quote! {
            impl crate::models::relations::TypeDbRelation for #ident {
                fn typeql_insert_statement(&self) -> String {
                    match self {
                        #(#query_arms)*
                    }
                }
            }
        }
    };

    Ok(body)
}
