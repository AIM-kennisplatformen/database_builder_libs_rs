use quote::{ToTokens, quote};
use syn::{Attribute, Fields, Item, Meta, parse_quote, punctuated::Punctuated, token::Comma};

use crate::utils::{is_option, trait_object_default};

pub(crate) fn add_model_attributes(item: &mut Item) {
    let attrs = crate::utils::item_attrs(item);
    attrs.push(parse_quote!(#[serde(rename_all = "kebab-case")]));
    add_option_skip_attributes(item);
}

fn add_option_skip_attributes(item: &mut Item) {
    let fields = match item {
        Item::Struct(item) => std::iter::once(&mut item.fields).collect::<Vec<_>>(),
        Item::Enum(item) => item
            .variants
            .iter_mut()
            .map(|variant| &mut variant.fields)
            .collect::<Vec<_>>(),
        _ => Vec::new(),
    };

    for fields in fields {
        for field in fields {
            if is_option(&field.ty)
                && !field.attrs.iter().any(|attribute| {
                    attribute.path().is_ident("serde")
                        && attribute
                            .meta
                            .to_token_stream()
                            .to_string()
                            .contains("skip_serializing_if")
                })
            {
                field
                    .attrs
                    .push(parse_quote!(#[serde(skip_serializing_if = "Option::is_none")]));
            }
        }
    }
}

pub(crate) fn add_enum_serde_attributes(item: &mut Item, content: Option<Meta>) {
    let Item::Enum(item) = item else {
        return;
    };

    let mut items = serde_items(&item.attrs);
    add_serde_item(&mut items, "tag", parse_quote!(tag = "type"));
    if let Some(content) = content {
        add_serde_item(&mut items, "content", content);
    }
    replace_serde_attributes(&mut item.attrs, items);
}

pub(crate) fn add_trait_object_attributes(item: &mut Item) {
    match item {
        Item::Struct(item) => add_trait_object_attributes_to_fields(&mut item.fields),
        Item::Enum(item) => {
            for variant in &mut item.variants {
                add_trait_object_attributes_to_fields(&mut variant.fields);
            }
        }
        _ => {}
    }
}

fn add_trait_object_attributes_to_fields(fields: &mut Fields) {
    for field in fields {
        let Some(default) = trait_object_default(&field.ty) else {
            continue;
        };

        let mut serde_items = serde_items(&field.attrs);
        add_serde_item(&mut serde_items, "deserialize_with", deserialize_path());
        add_serde_item(&mut serde_items, "serialize_with", serialize_path());
        if default {
            add_serde_item(&mut serde_items, "default", parse_quote!(default));
        }
        replace_serde_attributes(&mut field.attrs, serde_items);
    }
}

fn serde_items(attrs: &[Attribute]) -> Punctuated<Meta, Comma> {
    let mut items = Punctuated::new();
    for attr in attrs {
        if !attr.path().is_ident("serde") {
            continue;
        }
        if let Meta::List(list) = &attr.meta
            && let Ok(parsed) = list.parse_args_with(Punctuated::<Meta, Comma>::parse_terminated)
        {
            items.extend(parsed);
        }
    }
    items
}

fn add_serde_item(items: &mut Punctuated<Meta, Comma>, name: &str, value: Meta) {
    if items
        .iter()
        .any(|item| meta_name(item).is_some_and(|item| item == name))
    {
        return;
    }

    items.push(value);
}

fn meta_name(meta: &Meta) -> Option<String> {
    match meta {
        Meta::Path(path) => path.get_ident().map(ToString::to_string),
        Meta::NameValue(value) => value.path.get_ident().map(ToString::to_string),
        Meta::List(list) => list.path.get_ident().map(ToString::to_string),
    }
}

fn deserialize_path() -> Meta {
    parse_quote!(deserialize_with = "crate::models::deserialize_flattened")
}

fn serialize_path() -> Meta {
    parse_quote!(serialize_with = "crate::models::serialize_flattened")
}

fn replace_serde_attributes(attrs: &mut Vec<Attribute>, items: Punctuated<Meta, Comma>) {
    let mut replaced = false;
    let mut new_attrs = Vec::with_capacity(attrs.len() + 1);

    for attr in attrs.drain(..) {
        if !attr.path().is_ident("serde") {
            new_attrs.push(attr);
            continue;
        }

        if !replaced {
            let item_tokens = items.iter();
            let tokens = quote!(#(#item_tokens),*);
            new_attrs.push(parse_quote!(#[serde(#tokens)]));
            replaced = true;
        }
    }

    if !replaced {
        let item_tokens = items.iter();
        let tokens = quote!(#(#item_tokens),*);
        new_attrs.push(parse_quote!(#[serde(#tokens)]));
    }

    *attrs = new_attrs;
}
