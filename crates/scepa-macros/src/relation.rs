use proc_macro2::TokenStream;
use quote::quote;
use syn::{Fields, Item, LitStr};

use crate::{
    delegation::generate_enum_delegation_from_data,
    utils::{is_option_of, is_optional_role, is_required_role, kebab_case},
};

pub(crate) fn generate_impl(
    item: &Item,
    explicit_name: Option<&LitStr>,
) -> syn::Result<TokenStream> {
    match item {
        Item::Struct(item_struct) => {
            let name = explicit_name
                .map(LitStr::value)
                .unwrap_or_else(|| kebab_case(&item_struct.ident.to_string()));
            let (role_statements, attribute_statements) = relation_fields(&item_struct.fields)?;
            let type_ident = &item_struct.ident;
            Ok(quote! {
                impl crate::models::relations::TypeDbRelation for #type_ident {
                    fn typeql_insert_statement(&self) -> String {
                        let mut query = String::from("match\n");
                        let mut roles: Vec<String> = Vec::new();
                        let mut attributes = String::new();
                        #(#role_statements)*
                        #(#attribute_statements)*
                        query.push_str(&format!(
                            "put $relation isa {}, links ({}){};\n",
                            #name,
                            roles.join(", "),
                            attributes,
                        ));
                        query
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
                "TypeDbRelation",
            )
        }
        _ => Err(syn::Error::new_spanned(
            item,
            "#[typedb_relation] expects a struct or enum",
        )),
    }
}

fn relation_fields(fields: &Fields) -> syn::Result<(Vec<TokenStream>, Vec<TokenStream>)> {
    let Fields::Named(fields) = fields else {
        return Ok((Vec::new(), Vec::new()));
    };

    let mut role_statements = Vec::new();
    let mut attribute_statements = Vec::new();

    for field in &fields.named {
        let field_ident = field.ident.as_ref().unwrap();
        let label = kebab_case(&field_ident.to_string());
        if is_optional_role(&field.ty) {
            role_statements.push(quote! {
                if let Some(entity) = self.#field_ident.as_deref() {
                    let role_index = roles.len();
                    query.push_str(&format!(
                        "  {};\n",
                        entity.typeql_insert_statement(&format!("role_{role_index}")),
                    ));
                    roles.push(format!("{}: $role_{role_index}", #label));
                }
            });
        } else if is_required_role(&field.ty) {
            role_statements.push(quote! {
                let role_index = roles.len();
                query.push_str(&format!(
                    "  {};\n",
                    self.#field_ident
                        .as_ref()
                        .typeql_insert_statement(&format!("role_{role_index}")),
                ));
                roles.push(format!("{}: $role_{role_index}", #label));
            });
        } else if is_option_of(field, "String") {
            attribute_statements.push(quote! {
                    if let Some(value) = self.#field_ident.as_deref() {
                    attributes.push_str(&format!(
                        ", has {} {}",
                        #label,
                        serde_json::to_string(value).unwrap(),
                    ));
                }
            });
        } else if is_option_of(field, "DateTime") {
            attribute_statements.push(quote! {
                    if let Some(value) = self.#field_ident.as_ref() {
                    attributes.push_str(&format!(
                        ", has {} {}",
                        #label,
                        value.to_rfc3339_opts(chrono::SecondsFormat::AutoSi, true),
                    ));
                }
            });
        } else {
            return Err(syn::Error::new_spanned(
                &field.ty,
                "unsupported TypeDB relation field; expected a role, Option<String>, or Option<DateTime<_>>",
            ));
        }
    }

    Ok((role_statements, attribute_statements))
}
