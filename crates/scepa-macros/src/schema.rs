use proc_macro2::TokenStream;
use quote::{format_ident, quote};
use std::collections::{BTreeMap, BTreeSet};
use syn::LitStr;

#[derive(Default)]
struct Schema {
    attributes: BTreeMap<String, String>,
    entities: BTreeMap<String, Entity>,
    relations: BTreeMap<String, Relation>,
}

#[derive(Default)]
struct Entity {
    parent: Option<String>,
    abstract_type: bool,
    owns: Vec<String>,
    plays: Vec<(String, String)>,
}

#[derive(Default)]
struct Relation {
    parent: Option<String>,
    abstract_type: bool,
    owns: Vec<String>,
    roles: Vec<(String, String, bool)>, // (field role, player role, required)
}

pub(crate) fn expand(path: &LitStr) -> syn::Result<TokenStream> {
    let path_value = path.value();
    let base = std::env::var("CARGO_MANIFEST_DIR")
        .map_err(|_| syn::Error::new_spanned(path, "CARGO_MANIFEST_DIR is unavailable"))?;
    let file = std::path::Path::new(&base).join(&path_value);
    let source = std::fs::read_to_string(&file).map_err(|error| {
        syn::Error::new_spanned(
            path,
            format!("cannot read TypeDB schema `{path_value}`: {error}"),
        )
    })?;
    let schema = parse_schema(&source).map_err(|message| syn::Error::new_spanned(path, message))?;
    generate(schema)
}

fn parse_schema(source: &str) -> Result<Schema, String> {
    let source = source
        .lines()
        .map(|line| line.split('#').next().unwrap_or_default())
        .collect::<Vec<_>>()
        .join(" ");
    let mut schema = Schema::default();
    for statement in source
        .split(';')
        .map(|statement| statement.trim().trim_start_matches("define").trim())
        .filter(|s| !s.is_empty())
    {
        let words = statement.split_whitespace().collect::<Vec<_>>();
        match words.first().copied() {
            Some("attribute") => {
                if words.len() < 4 || words[2] != "value" {
                    return Err(format!("invalid attribute declaration: `{statement}`"));
                }
                schema.attributes.insert(
                    words[1].trim_end_matches(',').to_owned(),
                    words[3].to_owned(),
                );
            }
            Some("entity") => parse_entity(&mut schema, statement, &words)?,
            Some("relation") => parse_relation(&mut schema, statement, &words)?,
            Some(other) => return Err(format!("unsupported TypeDB declaration `{other}`")),
            None => {}
        }
    }
    Ok(schema)
}

fn parse_entity(schema: &mut Schema, statement: &str, words: &[&str]) -> Result<(), String> {
    let name = words
        .get(1)
        .ok_or_else(|| format!("invalid entity declaration: `{statement}`"))?
        .trim_end_matches(',');
    let parent = words
        .iter()
        .position(|word| *word == "sub")
        .and_then(|i| words.get(i + 1))
        .map(|s| s.trim_end_matches(',').to_owned());
    let entity = schema.entities.entry((*name).to_owned()).or_default();
    entity.parent = parent;
    entity.abstract_type = words.contains(&"@abstract,") || words.contains(&"@abstract");
    let body = statement
        .split_once(',')
        .map(|(_, body)| body)
        .unwrap_or_default();
    for clause in body.split(',').map(str::trim).filter(|s| !s.is_empty()) {
        let words = clause.split_whitespace().collect::<Vec<_>>();
        match words.first().copied() {
            Some("owns") if words.len() >= 2 => entity.owns.push(words[1].to_owned()),
            Some("plays") if words.len() >= 2 => {
                let Some((relation, role)) = words[1].split_once(':') else {
                    return Err(format!("invalid plays clause: `{clause}`"));
                };
                entity.plays.push((relation.to_owned(), role.to_owned()));
            }
            _ => {}
        }
    }
    Ok(())
}

fn parse_relation(schema: &mut Schema, statement: &str, words: &[&str]) -> Result<(), String> {
    let name = words
        .get(1)
        .ok_or_else(|| format!("invalid relation declaration: `{statement}`"))?
        .trim_end_matches(',');
    let parent = words
        .iter()
        .position(|word| *word == "sub")
        .and_then(|i| words.get(i + 1))
        .map(|s| s.trim_end_matches(',').to_owned());
    let relation = schema.relations.entry((*name).to_owned()).or_default();
    relation.parent = parent;
    relation.abstract_type = words.contains(&"@abstract,") || words.contains(&"@abstract");
    let body = statement
        .split_once(',')
        .map(|(_, body)| body)
        .unwrap_or_default();
    for clause in body.split(',').map(str::trim).filter(|s| !s.is_empty()) {
        let words = clause.split_whitespace().collect::<Vec<_>>();
        match words.first().copied() {
            Some("owns") if words.len() >= 2 => relation.owns.push(words[1].to_owned()),
            Some("relates") if words.len() >= 2 => {
                let schema_role = words[1].to_owned();
                let rust_role = if words.get(2).copied() == Some("as") {
                    words.get(3).unwrap_or(&words[1]).to_string()
                } else {
                    schema_role.clone()
                };
                let required = clause.contains("@card(1)");
                relation.roles.push((schema_role, rust_role, required));
            }
            _ => {}
        }
    }
    Ok(())
}

fn generate(schema: Schema) -> syn::Result<TokenStream> {
    let mut entity_defs = Vec::new();
    let entity_names = schema.entities.keys().cloned().collect::<Vec<_>>();
    for name in &entity_names {
        if schema.entities[name].abstract_type {
            continue;
        }
        let ident = format_ident!("{}", pascal_case(name));
        let mut attrs = Vec::new();
        collect_entity_attrs(&schema, name, &mut attrs)?;
        let fields = attrs.iter().map(|(name, ty)| {
            let field = format_ident!("{}", snake_case(name));
            let ty = rust_type(ty);
            quote!(#[serde(skip_serializing_if = "Option::is_none")] pub #field: ::core::option::Option<#ty>)
        });
        entity_defs.push(quote! {
            #[derive(::serde::Serialize, ::serde::Deserialize, ::core::fmt::Debug, ::core::clone::Clone, ::core::cmp::PartialEq, ::core::cmp::Eq)]
            #[serde(rename_all = "kebab-case")]
            #[crate::models::typedb_entity(name = #name)]
            pub struct #ident {
                pub entity_id: ::std::string::String,
                #(#fields,)*
            }
        });
    }

    let mut role_names = BTreeSet::new();
    for relation in schema.relations.values() {
        for (_, player_role, _) in &relation.roles {
            role_names.insert(player_role.clone());
        }
    }
    let role_defs = role_names.iter().map(|role| {
        let ident = format_ident!("{}", pascal_case(role));
        quote! {
            #[crate::models::typedb_relation_role]
            pub trait #ident: ::std::fmt::Debug + crate::models::TypeDbEntity {}
        }
    });
    let role_traits = schema
        .relations
        .values()
        .flat_map(|relation| {
            relation
                .roles
                .iter()
                .map(|(field, player, _)| (field.clone(), player.clone()))
        })
        .collect::<BTreeMap<_, _>>();
    let mut seen_impls = BTreeSet::new();
    let role_impls = schema
        .entities
        .iter()
        .flat_map(|(entity_name, entity)| {
            if entity.abstract_type {
                return Vec::new();
            }
            let ident = format_ident!("{}", pascal_case(entity_name));
            let mut plays = Vec::new();
            collect_entity_plays(&schema, entity_name, &mut plays);
            plays
                .iter()
                .filter_map(|(_, role)| {
                    let player_role = role_traits.get(role).unwrap_or(role);
                    if !seen_impls.insert((entity_name.clone(), player_role.clone())) {
                        return None;
                    }
                    let role_ident = format_ident!("{}", pascal_case(player_role));
                    Some(quote! {
                        #[crate::models::typedb_relation_role(name = #entity_name)]
                        impl #role_ident for #ident {}
                    })
                })
                .collect::<Vec<_>>()
        })
        .flatten();
    let relation_defs = schema.relations.iter().filter_map(|(name, relation)| {
        if relation.abstract_type { return None; }
        let ident = relation_type_name(&schema, name);
        let mut roles = relation
            .parent
            .as_ref()
            .and_then(|parent| schema.relations.get(parent))
            .map(|parent| parent.roles.clone())
            .unwrap_or_default();
        for (_, player_role, _) in &relation.roles {
            roles.retain(|(_, inherited_player, _)| inherited_player != player_role);
        }
        roles.extend(relation.roles.clone());
        let owns = relation.owns.clone();
        let role_fields = roles.iter().map(|(field_role, player_role, required)| {
            let field = format_ident!("{}", snake_case(field_role));
            let role = format_ident!("{}", pascal_case(player_role));
            if *required && name != "contribution" {
                quote!(
                    #[serde(serialize_with = "crate::models::serialize_flattened", deserialize_with = "crate::models::deserialize_flattened")]
                    pub #field: ::std::boxed::Box<dyn #role>
                )
            } else {
                quote!(
                    #[serde(skip_serializing_if = "Option::is_none", serialize_with = "crate::models::serialize_flattened", deserialize_with = "crate::models::deserialize_flattened", default)]
                    pub #field: ::core::option::Option<::std::boxed::Box<dyn #role>>
                )
            }
        });
        let attr_fields = owns.iter().map(|attr| {
            let field = format_ident!("{}", snake_case(attr));
            let ty = rust_type(
                schema
                    .attributes
                    .get(attr)
                    .map(String::as_str)
                    .unwrap_or("string"),
            );
            quote!(#[serde(skip_serializing_if = "Option::is_none")] pub #field: ::core::option::Option<#ty>)
        });
        Some(quote! {
            #[derive(::serde::Serialize, ::serde::Deserialize, ::core::fmt::Debug)]
            #[serde(rename_all = "kebab-case")]
            #[crate::models::typedb_relation(name = #name)]
            pub struct #ident { #(#role_fields,)* #(#attr_fields,)* }
        })
    });
    let entity_groups = generate_entity_groups(&schema);
    let relation_groups = generate_relation_groups(&schema);
    Ok(quote! {
        #(#entity_defs)*
        #(#entity_groups)*
        #(#role_defs)*
        #(#role_impls)*
        #(#relation_defs)*
        #(#relation_groups)*
    })
}

fn generate_entity_groups(schema: &Schema) -> Vec<TokenStream> {
    let roots = schema
        .entities
        .iter()
        .filter(|(_, entity)| entity.parent.is_none());
    let mut groups = Vec::new();
    let mut variants = Vec::new();
    for (root, entity) in roots {
        let members = descendants(schema, root);
        let enum_name = if entity.abstract_type {
            pascal_case(root)
        } else {
            format!("{}Entity", pascal_case(root))
        };
        let enum_ident = format_ident!("{}", enum_name);
        let member_variants = members.iter().filter_map(|member| {
            if schema.entities[member].abstract_type {
                return None;
            }
            let variant = format_ident!("{}", pascal_case(member));
            let ty = variant.clone();
            Some(quote!(#variant(#ty)))
        });
        groups.push(quote! {
            #[derive(::serde::Serialize, ::serde::Deserialize, ::core::fmt::Debug, ::core::clone::Clone)]
            #[serde(tag = "type", content = "attrs", rename_all = "kebab-case")]
            #[crate::models::typedb_model(entity)]
            pub enum #enum_ident { #(#member_variants),* }
        });
        variants.push((pascal_case(root), enum_ident));
    }
    let entity_variants = variants.iter().map(|(variant, ty)| {
        let variant = format_ident!("{}", variant);
        quote!(#variant(#ty))
    });
    groups.push(quote! {
        #[derive(::serde::Serialize, ::serde::Deserialize, ::core::fmt::Debug)]
        #[serde(rename_all = "kebab-case")]
        #[crate::models::typedb_model(entity)]
        pub enum Entity { #(#entity_variants),* }
    });
    groups
}

fn generate_relation_groups(schema: &Schema) -> Vec<TokenStream> {
    let roots = schema
        .relations
        .iter()
        .filter(|(_, relation)| relation.parent.is_none());
    let mut groups = Vec::new();
    let mut variants = Vec::new();
    for (root, _relation) in roots {
        let members = relation_descendants(schema, root);
        let enum_name = if root == "publication-event" {
            "PublicationEventRelation".to_owned()
        } else {
            pascal_case(root)
        };
        let enum_ident = format_ident!("{}", enum_name);
        let member_variants = members.iter().filter_map(|member| {
            if schema.relations[member].abstract_type {
                return None;
            }
            let variant = format_ident!("{}", pascal_case(member));
            let type_name = relation_type_name(schema, member);
            Some(quote!(#variant(#type_name)))
        });
        if members.len() > 1 {
            groups.push(quote! {
                #[derive(::serde::Serialize, ::serde::Deserialize, ::core::fmt::Debug)]
                #[serde(tag = "type", rename_all = "kebab-case")]
                #[crate::models::typedb_model(relation)]
                pub enum #enum_ident { #(#member_variants),* }
            });
            variants.push((pascal_case(root), enum_ident));
        } else {
            variants.push((pascal_case(root), relation_type_name(schema, root)));
        }
    }
    let relation_variants = variants.iter().map(|(variant, ty)| {
        let variant = if variant == "PublicationEvent" {
            format_ident!("PublicationEvent")
        } else {
            format_ident!("{}", variant)
        };
        quote!(#variant(#ty))
    });
    groups.push(quote! {
        #[derive(::serde::Serialize, ::serde::Deserialize, ::core::fmt::Debug)]
        #[serde(rename_all = "kebab-case")]
        #[crate::models::typedb_model(relation)]
        pub enum Relation { #(#relation_variants),* }
    });
    groups
}

fn descendants(schema: &Schema, root: &str) -> Vec<String> {
    schema
        .entities
        .keys()
        .filter(|name| {
            let mut current = name.as_str();
            loop {
                if current == root {
                    return true;
                }
                let Some(parent) = schema
                    .entities
                    .get(current)
                    .and_then(|entity| entity.parent.as_deref())
                else {
                    return false;
                };
                current = parent;
            }
        })
        .cloned()
        .collect()
}

fn relation_descendants(schema: &Schema, root: &str) -> Vec<String> {
    schema
        .relations
        .keys()
        .filter(|name| {
            let mut current = name.as_str();
            loop {
                if current == root {
                    return true;
                }
                let Some(parent) = schema
                    .relations
                    .get(current)
                    .and_then(|relation| relation.parent.as_deref())
                else {
                    return false;
                };
                current = parent;
            }
        })
        .cloned()
        .collect()
}

fn relation_type_name(schema: &Schema, name: &str) -> syn::Ident {
    if name == "contribution" && relation_descendants(schema, name).len() > 1 {
        format_ident!("BaseContribution")
    } else {
        format_ident!("{}", pascal_case(name))
    }
}

fn collect_entity_plays(schema: &Schema, name: &str, output: &mut Vec<(String, String)>) {
    if let Some(entity) = schema.entities.get(name) {
        if let Some(parent) = &entity.parent {
            collect_entity_plays(schema, parent, output);
        }
        output.extend(entity.plays.iter().cloned());
    }
}

fn collect_entity_attrs(
    schema: &Schema,
    name: &str,
    output: &mut Vec<(String, String)>,
) -> syn::Result<()> {
    if let Some(entity) = schema.entities.get(name) {
        if let Some(parent) = &entity.parent {
            collect_entity_attrs(schema, parent, output)?;
        }
        for attr in &entity.owns {
            if attr == "entity-id" {
                continue;
            }
            let ty = schema.attributes.get(attr).ok_or_else(|| {
                syn::Error::new(
                    proc_macro2::Span::call_site(),
                    format!("entity `{name}` owns undeclared attribute `{attr}`"),
                )
            })?;
            if !output.iter().any(|(existing, _)| existing == attr) {
                output.push((attr.clone(), ty.clone()));
            }
        }
    }
    Ok(())
}

fn rust_type(type_name: &str) -> TokenStream {
    match type_name {
        "datetime" => quote!(::chrono::DateTime<::chrono::FixedOffset>),
        _ => quote!(::std::string::String),
    }
}

fn pascal_case(value: &str) -> String {
    value
        .split('-')
        .map(|part| {
            let mut chars = part.chars();
            chars
                .next()
                .map(|c| c.to_uppercase().collect::<String>())
                .unwrap_or_default()
                + chars.as_str()
        })
        .collect()
}
fn snake_case(value: &str) -> String {
    value.replace('-', "_")
}
