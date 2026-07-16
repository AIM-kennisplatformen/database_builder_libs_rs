use serde::{
    Deserialize, Deserializer, Serialize, Serializer,
    de::{self, DeserializeOwned},
};
use serde_json::Value;

pub use scepa_macros::typedb_schema;
#[allow(unused_imports)]
use scepa_macros::{typedb_entity, typedb_model, typedb_relation, typedb_relation_role};

pub trait TypeDbEntity {
    fn typeql_type(&self) -> &'static str;
    fn entity_id(&self) -> &str;
    fn typeql_identity_pattern(&self, variable: &str) -> String;
    fn typeql_metadata_statements(&self) -> Vec<String>;
    fn typeql_insert_statement(&self, variable: &str) -> String;
}

pub(crate) trait TypeDbRelation {
    fn typeql_insert_statement(&self) -> String;
}

pub mod chunk;
pub mod generated;
pub use generated::*;

#[cfg(test)]
mod tests;

fn deserialize_flattened<'de, T, D>(deserializer: D) -> Result<T, D::Error>
where
    D: Deserializer<'de>,
    T: DeserializeOwned,
{
    let mut value = Value::deserialize(deserializer)?;
    flatten_attrs(&mut value).map_err(de::Error::custom)?;
    serde_json::from_value(value).map_err(de::Error::custom)
}

fn serialize_flattened<T, S>(value: &T, serializer: S) -> Result<S::Ok, S::Error>
where
    T: ?Sized + Serialize,
    S: Serializer,
{
    let mut value = serde_json::to_value(value).map_err(serde::ser::Error::custom)?;
    let object = value
        .as_object_mut()
        .ok_or_else(|| serde::ser::Error::custom("entity must be an object"))?;
    let entity_type = object
        .remove("type")
        .ok_or_else(|| serde::ser::Error::custom("missing entity type"))?;
    let attrs = Value::Object(std::mem::take(object));

    serde_json::json!({
        "type": entity_type,
        "attrs": attrs,
    })
    .serialize(serializer)
}

fn flatten_attrs(value: &mut Value) -> Result<(), &'static str> {
    let object = value.as_object_mut().ok_or("entity must be an object")?;

    let attrs = object
        .remove("attrs")
        .ok_or("missing entity attrs")?
        .as_object()
        .ok_or("entity attrs must be an object")?
        .clone();

    object.extend(attrs);
    Ok(())
}
