use proc_macro2::TokenStream;
use quote::quote;
use syn::{Fields, Item, LitStr};

use crate::{delegation::generate_enum_delegation_from_data, utils::kebab_case};

pub(crate) fn generate_impl(
    item: &Item,
    explicit_name: Option<&LitStr>,
) -> syn::Result<TokenStream> {
    if !matches!(item, Item::Struct(_) | Item::Enum(_)) {
        return Err(syn::Error::new_spanned(
            item,
            "#[typedb_entity] expects a struct or enum",
        ));
    }

    match item {
        Item::Struct(item_struct) => {
            let name = explicit_name
                .map(LitStr::value)
                .unwrap_or_else(|| kebab_case(&item_struct.ident.to_string()));
            let attributes = entity_attributes(&item_struct.fields)?;
            let type_ident = &item_struct.ident;
            Ok(quote! {
                impl crate::models::entities::TypeDbEntity for #type_ident {
                    fn typeql_insert_statement(&self, variable: &str) -> String {
                        let mut attributes = String::new();
                        #(#attributes)*
                        format!("${variable} isa {}{attributes}", #name)
                    }
                }
            })
        }
        Item::Enum(item_enum) => {
            if let Some(explicit_name) = explicit_name {
                return Err(syn::Error::new_spanned(
                    explicit_name,
                    "enum TypeDB names are delegated to their variants",
                ));
            }
            generate_enum_delegation_from_data(
                &item_enum.ident,
                &item_enum.variants,
                "TypeDbEntity",
            )
        }
        _ => unreachable!(),
    }
}

fn entity_attributes(fields: &Fields) -> syn::Result<Vec<TokenStream>> {
    let Fields::Named(fields) = fields else {
        return Ok(Vec::new());
    };

    fields
        .named
        .iter()
        .map(|field| {
            let field_ident = field.ident.as_ref().unwrap();
            let label = kebab_case(&field_ident.to_string());
            if crate::utils::is_option_of(field, "String") {
                Ok(quote! {
                    if let Some(value) = self.#field_ident.as_deref() {
                        attributes.push_str(&format!(
                            ", has {} {}",
                            #label,
                            serde_json::to_string(value).unwrap(),
                        ));
                    }
                })
            } else if crate::utils::is_option_of(field, "DateTime") {
                Ok(quote! {
                    if let Some(value) = self.#field_ident.as_ref() {
                        attributes.push_str(&format!(
                            ", has {} {}",
                            #label,
                            value.to_rfc3339_opts(chrono::SecondsFormat::AutoSi, true),
                        ));
                    }
                })
            } else {
                Err(syn::Error::new_spanned(
                    &field.ty,
                    "unsupported TypeDB entity field; expected Option<String> or Option<DateTime<_>>",
                ))
            }
        })
        .collect()
}
