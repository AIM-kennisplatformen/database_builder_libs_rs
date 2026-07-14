use proc_macro::TokenStream;

use quote::{ToTokens, quote};
use syn::{
    Attribute, Fields, Item, LitStr, Meta, Type, parse::Parse, parse::ParseStream,
    parse_macro_input, parse_quote, punctuated::Punctuated, token::Comma,
};

#[proc_macro_attribute]
pub fn typedb_model(_attr: TokenStream, item: TokenStream) -> TokenStream {
    let mut item = parse_macro_input!(item as Item);

    add_model_attributes(&mut item);

    quote!(#item).into()
}

#[proc_macro_attribute]
pub fn typedb_entity(_attr: TokenStream, item: TokenStream) -> TokenStream {
    let mut item = parse_macro_input!(item as Item);

    add_model_attributes(&mut item);
    add_enum_serde_attributes(&mut item, Some(parse_quote!(content = "attrs")));

    quote!(#item).into()
}

#[proc_macro_attribute]
pub fn typedb_relation(_attr: TokenStream, item: TokenStream) -> TokenStream {
    let mut item = parse_macro_input!(item as Item);

    add_model_attributes(&mut item);
    add_enum_serde_attributes(&mut item, None);
    add_trait_object_attributes(&mut item);

    quote!(#item).into()
}

#[proc_macro_attribute]
pub fn typedb_relation_role(attr: TokenStream, item: TokenStream) -> TokenStream {
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

fn add_model_attributes(item: &mut Item) {
    {
        let attrs = item_attrs(item);
        attrs.push(parse_quote!(#[serde(rename_all = "kebab-case")]));
    }
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

fn is_option(ty: &Type) -> bool {
    let Type::Path(ty) = ty else {
        return false;
    };
    ty.path
        .segments
        .last()
        .is_some_and(|segment| segment.ident == "Option")
}

fn add_enum_serde_attributes(item: &mut Item, content: Option<Meta>) {
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

fn add_trait_object_attributes(item: &mut Item) {
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

fn trait_object_default(ty: &Type) -> Option<bool> {
    let Type::Path(ty) = ty else {
        return None;
    };
    let outer = ty.path.segments.last()?;

    match outer.ident.to_string().as_str() {
        "Option" => {
            let inner = type_argument(outer)?;
            is_boxed_trait_object(inner).then_some(true)
        }
        "Box" => is_boxed_trait_object_segment(outer).then_some(false),
        _ => None,
    }
}

fn is_boxed_trait_object(ty: &Type) -> bool {
    let Type::Path(ty) = ty else {
        return false;
    };
    let Some(segment) = ty.path.segments.last() else {
        return false;
    };
    is_boxed_trait_object_segment(segment)
}

fn is_boxed_trait_object_segment(segment: &syn::PathSegment) -> bool {
    if segment.ident != "Box" {
        return false;
    }

    let Some(Type::TraitObject(object)) = type_argument(segment) else {
        return false;
    };
    !object.bounds.is_empty()
}

fn type_argument(segment: &syn::PathSegment) -> Option<&Type> {
    let syn::PathArguments::AngleBracketed(arguments) = &segment.arguments else {
        return None;
    };
    let Some(syn::GenericArgument::Type(ty)) = arguments.args.first() else {
        return None;
    };
    Some(ty)
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

fn item_attrs(item: &mut Item) -> &mut Vec<Attribute> {
    match item {
        Item::Const(item) => &mut item.attrs,
        Item::Enum(item) => &mut item.attrs,
        Item::ExternCrate(item) => &mut item.attrs,
        Item::Fn(item) => &mut item.attrs,
        Item::ForeignMod(item) => &mut item.attrs,
        Item::Impl(item) => &mut item.attrs,
        Item::Macro(item) => &mut item.attrs,
        Item::Mod(item) => &mut item.attrs,
        Item::Static(item) => &mut item.attrs,
        Item::Struct(item) => &mut item.attrs,
        Item::Trait(item) => &mut item.attrs,
        Item::TraitAlias(item) => &mut item.attrs,
        Item::Type(item) => &mut item.attrs,
        Item::Union(item) => &mut item.attrs,
        Item::Use(item) => &mut item.attrs,
        Item::Verbatim(_) => panic!("#[typedb_relation] expects a Rust item"),
        _ => panic!("#[typedb_relation] does not support this Rust item"),
    }
}
