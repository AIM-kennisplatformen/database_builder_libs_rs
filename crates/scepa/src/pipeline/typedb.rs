use crate::{
    domain::DocumentWithChunks,
    models::{entities::TypeDbEntity, relations::TypeDbRelation},
};

pub fn typeql_queries(document: &DocumentWithChunks) -> Vec<String> {
    let mut queries = vec![entity_query(&document.document)];
    queries.extend(document.entities.iter().map(entity_query));
    queries.extend(document.relations.iter().map(relation_query));
    queries
}

fn entity_query<T: TypeDbEntity + ?Sized>(entity: &T) -> String {
    format!("put {};", entity.typeql_insert_statement("entity"))
}

fn relation_query<T: TypeDbRelation + ?Sized>(relation: &T) -> String {
    relation.typeql_insert_statement()
}
