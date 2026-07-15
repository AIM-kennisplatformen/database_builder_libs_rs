use syn::{Attribute, Item, Type};

pub(crate) fn is_option_of(field: &syn::Field, name: &str) -> bool {
    let Some(inner) = option_inner(&field.ty) else {
        return false;
    };
    type_path_ends_with(inner, name)
}

pub(crate) fn is_type(field: &syn::Field, name: &str) -> bool {
    type_path_ends_with(&field.ty, name)
}

pub(crate) fn is_option(ty: &Type) -> bool {
    let Type::Path(ty) = ty else {
        return false;
    };
    ty.path
        .segments
        .last()
        .is_some_and(|segment| segment.ident == "Option")
}

pub(crate) fn is_optional_role(ty: &Type) -> bool {
    option_inner(ty).is_some_and(is_required_role)
}

pub(crate) fn is_required_role(ty: &Type) -> bool {
    let Type::Path(path) = ty else {
        return false;
    };
    let Some(segment) = path.path.segments.last() else {
        return false;
    };
    if segment.ident != "Box" {
        return false;
    }
    let syn::PathArguments::AngleBracketed(arguments) = &segment.arguments else {
        return false;
    };
    arguments
        .args
        .first()
        .and_then(|argument| match argument {
            syn::GenericArgument::Type(Type::TraitObject(_)) => Some(()),
            _ => None,
        })
        .is_some()
}

pub(crate) fn trait_object_default(ty: &Type) -> Option<bool> {
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

fn option_inner(ty: &Type) -> Option<&Type> {
    let Type::Path(path) = ty else {
        return None;
    };
    let segment = path.path.segments.last()?;
    if segment.ident != "Option" {
        return None;
    }
    type_argument(segment)
}

fn type_path_ends_with(ty: &Type, name: &str) -> bool {
    let Type::Path(path) = ty else {
        return false;
    };
    path.path
        .segments
        .last()
        .is_some_and(|segment| segment.ident == name)
}

pub(crate) fn kebab_case(value: &str) -> String {
    let mut result = String::new();
    for (index, character) in value.chars().enumerate() {
        if character == '_' {
            result.push('-');
        } else {
            if character.is_uppercase() && index > 0 {
                result.push('-');
            }
            result.extend(character.to_lowercase());
        }
    }
    result
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

pub(crate) fn item_attrs(item: &mut Item) -> &mut Vec<Attribute> {
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
