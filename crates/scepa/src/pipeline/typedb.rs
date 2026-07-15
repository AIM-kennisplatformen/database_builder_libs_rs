use std::collections::HashSet;

use crate::{
    domain::DocumentWithChunks,
    models::{entities::TypeDbEntity, relations::TypeDbRelation},
};

pub fn typeql_queries(document: &DocumentWithChunks) -> Vec<String> {
    let mut queries = Vec::new();
    let mut seen_entity_ids = HashSet::new();
    let mut entities: Vec<(&dyn TypeDbEntity, bool)> = Vec::new();

    if seen_entity_ids.insert(document.document.entity_id().to_owned()) {
        entities.push((&document.document, true));
    }
    for entity in &document.entities {
        if seen_entity_ids.insert(entity.entity_id().to_owned()) {
            entities.push((entity, false));
        }
    }

    queries.extend(entities.iter().map(|(entity, _)| entity_key_query(*entity)));
    for (entity, authoritative) in entities {
        queries.extend(entity_metadata_queries(entity, authoritative));
    }
    queries.extend(document.relations.iter().map(relation_query));
    queries
}

fn entity_key_query<T: TypeDbEntity + ?Sized>(entity: &T) -> String {
    format!("put {};", entity.typeql_identity_pattern("entity"))
}

fn entity_metadata_queries<T: TypeDbEntity + ?Sized>(
    entity: &T,
    authoritative: bool,
) -> Vec<String> {
    entity
        .typeql_metadata_statements()
        .into_iter()
        .map(|metadata| {
            if authoritative {
                update_metadata_query(entity, &metadata)
            } else {
                fill_metadata_query(entity, &metadata)
            }
        })
        .collect()
}

fn fill_metadata_query<T: TypeDbEntity + ?Sized>(entity: &T, metadata: &str) -> String {
    let attribute = metadata
        .split_whitespace()
        .nth(1)
        .expect("generated metadata always contains an attribute label");
    format!(
        "match\n  {};\n  not {{ $entity has {} $_; }};\ninsert\n  $entity {};",
        entity.typeql_identity_pattern("entity"),
        attribute,
        metadata,
    )
}

fn update_metadata_query<T: TypeDbEntity + ?Sized>(entity: &T, metadata: &str) -> String {
    format!(
        "match\n  {};\nupdate\n  $entity {};",
        entity.typeql_identity_pattern("entity"),
        metadata,
    )
}

fn relation_query<T: TypeDbRelation + ?Sized>(relation: &T) -> String {
    relation.typeql_insert_statement()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::pipeline::tei;

    #[test]
    fn entity_queries_put_keys_before_metadata_and_relations() {
        let document = tei::parse_with_pdf_hash(
            r#"
            <TEI>
              <teiHeader>
                <fileDesc>
                  <titleStmt><title type="main">Paper B</title></titleStmt>
                  <sourceDesc><biblStruct>
                    <analytic/>
                    <idno type="DOI">10.1234/b</idno>
                  </biblStruct></sourceDesc>
                </fileDesc>
              </teiHeader>
              <text><back><listBibl><biblStruct xml:id="a">
                <analytic><title>Paper A</title></analytic>
                <idno type="DOI">https://doi.org/10.1234/a</idno>
              </biblStruct></listBibl></back></text>
            </TEI>
            "#,
            "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
        )
        .unwrap();

        let queries = typeql_queries(&document);
        assert_eq!(
            queries
                .iter()
                .take(2)
                .filter(|query| query.starts_with("put $entity isa "))
                .count(),
            2
        );
        assert!(queries.iter().any(|query| {
            query.contains("not { $entity has title $_; }")
                && query.contains("insert\n  $entity has title")
        }));
        assert!(
            queries
                .iter()
                .any(|query| { query.contains("update\n  $entity has title \"Paper B\";") })
        );
        assert!(queries.iter().any(|query| {
            query.contains("match\n  $role_0 isa research-paper, has entity-id")
                && query.contains("$role_1 isa research-paper, has entity-id")
        }));
        assert!(
            !queries
                .iter()
                .take(2)
                .any(|query| query.contains("has doi"))
        );
    }
}
