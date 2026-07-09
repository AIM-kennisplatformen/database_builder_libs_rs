use std::{fmt::Write as _, time::Duration};

use futures_util::StreamExt;
use thiserror::Error;
use typedb_driver::{Error as TypedbDriverError, Transaction, answer::QueryAnswer};

use crate::{
    models::domain::{
        Author, Authoring, Department, Funding, Institution, InstitutionKind, Literature, Paper,
        Project, PublicationDate,
    },
    stores::typedb::store::{TypedbConnected, TypedbStore},
};

const INITIAL_RETRY_DELAY: Duration = Duration::from_millis(50);
const MAX_RETRY_DELAY: Duration = Duration::from_secs(2);

#[derive(Debug, Error)]
pub enum TypedbExportError {
    #[error("failed to open TypeDB write transaction")]
    OpenTransaction {
        #[source]
        source: anyhow::Error,
    },

    #[error("failed to execute TypeDB export query: {query}")]
    Query {
        query: String,
        #[source]
        source: Box<TypedbDriverError>,
    },

    #[error("failed to commit TypeDB export transaction after {attempts} attempt(s)")]
    Commit {
        attempts: usize,
        #[source]
        source: Box<TypedbDriverError>,
    },
}

pub async fn write_paper_typedb(
    paper: &Paper,
    store: &TypedbStore<TypedbConnected>,
) -> Result<(), TypedbExportError> {
    let queries = paper_typeql_queries(paper);
    let mut attempt = 1;
    let mut retry_delay = INITIAL_RETRY_DELAY;

    loop {
        match write_paper_typedb_once(&queries, store, attempt).await {
            Ok(()) => return Ok(()),
            Err(error) if should_retry_export(&error) => {
                tokio::time::sleep(retry_delay).await;
                attempt += 1;
                retry_delay = next_retry_delay(retry_delay);
            }
            Err(error) => return Err(error),
        }
    }
}

async fn write_paper_typedb_once(
    queries: &[String],
    store: &TypedbStore<TypedbConnected>,
    attempt: usize,
) -> Result<(), TypedbExportError> {
    let transaction = store
        .write_transaction()
        .await
        .map_err(|source| TypedbExportError::OpenTransaction { source })?;

    for query in queries {
        execute_query(&transaction, query).await?;
    }

    transaction
        .commit()
        .await
        .map_err(|source| TypedbExportError::Commit {
            attempts: attempt,
            source: Box::new(source),
        })
}

fn should_retry_export(error: &TypedbExportError) -> bool {
    matches!(
        error,
        TypedbExportError::Commit { source, .. } if is_isolation_conflict(source)
    )
}

fn is_isolation_conflict(error: &TypedbDriverError) -> bool {
    let code = error.code();
    let message = error.to_string();

    code == "STC2"
        || message.contains("[STC2]")
        || message.contains("isolation conflict")
        || message.contains("lock held by a concurrent commit")
}

fn next_retry_delay(current: Duration) -> Duration {
    (current * 2).min(MAX_RETRY_DELAY)
}

async fn execute_query(transaction: &Transaction, query: &str) -> Result<(), TypedbExportError> {
    let answer = transaction
        .query(query)
        .await
        .map_err(|source| TypedbExportError::Query {
            query: query.to_owned(),
            source: Box::new(source),
        })?;

    match answer {
        QueryAnswer::Ok(_) => {}
        QueryAnswer::ConceptRowStream(_, mut rows) => {
            while let Some(row) = rows.next().await {
                row.map_err(|source| TypedbExportError::Query {
                    query: query.to_owned(),
                    source: Box::new(source),
                })?;
            }
        }
        QueryAnswer::ConceptDocumentStream(_, mut documents) => {
            while let Some(document) = documents.next().await {
                document.map_err(|source| TypedbExportError::Query {
                    query: query.to_owned(),
                    source: Box::new(source),
                })?;
            }
        }
    }

    Ok(())
}

fn paper_typeql_queries(paper: &Paper) -> Vec<String> {
    let source = paper.source.as_str();
    let literature = literature_ref(source, &paper.graph.literature, "literature");
    let mut queries = vec![];

    push_unique_query(&mut queries, put_entity_query(&literature));

    for authoring in &paper.graph.authorings {
        push_authoring_queries(&mut queries, source, &literature, authoring);
    }

    for publication in &paper.graph.publications {
        let publisher = institution_ref(source, &publication.publisher, "publisher");

        push_unique_query(&mut queries, put_entity_query(&publisher));
        push_unique_query(
            &mut queries,
            put_relation_query(
                "publication",
                "publication",
                &[("literature", &literature), ("publisher", &publisher)],
            ),
        );
    }

    for funding in &paper.graph.fundings {
        push_funding_queries(&mut queries, source, &literature, funding);
    }

    for citation in &paper.graph.citations {
        let cited_source = citation_source(source, &citation.id);
        let cited = literature_ref(&cited_source, &citation.cited, "cited");

        push_unique_query(&mut queries, put_entity_query(&cited));
        push_unique_query(
            &mut queries,
            put_relation_query(
                "citation",
                "citation",
                &[("citing", &literature), ("cited", &cited)],
            ),
        );

        for authoring in &citation.authorings {
            push_authoring_queries(&mut queries, &cited_source, &cited, authoring);
        }
    }

    queries
}

fn push_authoring_queries(
    queries: &mut Vec<String>,
    source: &str,
    literature: &EntityRef,
    authoring: &Authoring,
) {
    let author = author_ref(source, &authoring.author, "author");

    push_unique_query(queries, put_entity_query(&author));
    push_unique_query(
        queries,
        put_relation_query(
            "authoring",
            "authoring",
            &[("literature", literature), ("author", &author)],
        ),
    );

    for affiliation in &authoring.affiliations {
        let institution = institution_ref(source, &affiliation.institution, "institution");

        push_unique_query(queries, put_entity_query(&institution));
        push_unique_query(
            queries,
            put_relation_query(
                "affiliation",
                "affiliation",
                &[("author", &author), ("institution", &institution)],
            ),
        );

        if let Some(department) = affiliation.department.as_ref() {
            let department = department_ref(source, department, "department");

            push_unique_query(queries, put_entity_query(&department));
            push_unique_query(
                queries,
                put_relation_query(
                    "institutional_structure",
                    "institutional-structure",
                    &[("department", &department), ("institution", &institution)],
                ),
            );
        }
    }
}

fn push_funding_queries(
    queries: &mut Vec<String>,
    source: &str,
    literature: &EntityRef,
    funding: &Funding,
) {
    let institution = institution_ref(source, &funding.funder, "institution");
    let project = project_ref(source, &funding.project, "project");

    push_unique_query(queries, put_entity_query(&institution));
    push_unique_query(queries, put_entity_query(&project));
    push_unique_query(
        queries,
        put_relation_query(
            "research_activity",
            "research-activity",
            &[("institution", &institution), ("project", &project)],
        ),
    );
    push_unique_query(
        queries,
        put_relation_query(
            "project_literature",
            "project-literature",
            &[("project", &project), ("literature", literature)],
        ),
    );
}

fn push_unique_query(queries: &mut Vec<String>, query: String) {
    if !queries.contains(&query) {
        queries.push(query);
    }
}

fn literature_ref(source: &str, literature: &Literature, variable: &'static str) -> EntityRef {
    let (type_label, core, doi) = match literature {
        Literature::Scientific(scientific) => (
            "scientific-literature",
            &scientific.core,
            scientific.doi.as_deref(),
        ),
        Literature::ProjectReport(core) => ("project-reports", core, None),
        Literature::Grey(core) => ("grey-literature", core, None),
        Literature::Survey(core) => ("survey", core, None),
        Literature::Book(core) => ("book", core, None),
        Literature::BookChapter(core) => ("book-chapter", core, None),
    };

    let mut attributes = sourced_attributes(source);
    push_optional_string_attribute(&mut attributes, "title", core.title.as_deref());
    push_optional_date_attribute(
        &mut attributes,
        "publishing-date",
        core.publishing_date.as_ref(),
    );
    push_optional_string_attribute(&mut attributes, "issn", core.issn.as_deref());
    push_optional_string_attribute(&mut attributes, "isbn", core.isbn.as_deref());
    push_optional_string_attribute(&mut attributes, "doi", doi);

    EntityRef {
        variable,
        type_label,
        attributes,
    }
}

fn author_ref(source: &str, author: &Author, variable: &'static str) -> EntityRef {
    let mut attributes = sourced_attributes(source);
    push_optional_string_attribute(&mut attributes, "forename", author.forename.as_deref());
    push_optional_string_attribute(&mut attributes, "surname", author.surname.as_deref());

    EntityRef {
        variable,
        type_label: "author",
        attributes,
    }
}

fn institution_ref(source: &str, institution: &Institution, variable: &'static str) -> EntityRef {
    let mut attributes = sourced_attributes(source);
    push_optional_string_attribute(&mut attributes, "name", institution.name.as_deref());
    push_optional_string_attribute(&mut attributes, "ror-id", institution.ror_id.as_deref());

    EntityRef {
        variable,
        type_label: institution_type_label(&institution.kind),
        attributes,
    }
}

fn department_ref(source: &str, department: &Department, variable: &'static str) -> EntityRef {
    let mut attributes = sourced_attributes(source);
    push_optional_string_attribute(&mut attributes, "name", department.name.as_deref());

    EntityRef {
        variable,
        type_label: "department",
        attributes,
    }
}

fn project_ref(source: &str, project: &Project, variable: &'static str) -> EntityRef {
    let mut attributes = sourced_attributes(source);
    push_optional_string_attribute(&mut attributes, "project-name", project.name.as_deref());
    push_optional_integer_attribute(&mut attributes, "project-number", project.number);

    EntityRef {
        variable,
        type_label: "project",
        attributes,
    }
}

fn institution_type_label(kind: &InstitutionKind) -> &'static str {
    match kind {
        InstitutionKind::Institution => "institution",
        InstitutionKind::GovernmentInstitution => "government-institution",
        InstitutionKind::SemiGovernmentInstitution => "semi-government-institution",
        InstitutionKind::KnowledgeInstitution => "knowledge-institution",
        InstitutionKind::University => "university",
        InstitutionKind::UniversityOfAppliedSciences => "university-of-applied-sciences",
    }
}

fn citation_source(source: &str, citation_id: &str) -> String {
    format!("{source}#citation:{citation_id}")
}

fn sourced_attributes(source: &str) -> Vec<Attribute> {
    vec![Attribute::string("source", source)]
}

fn push_optional_string_attribute(
    attributes: &mut Vec<Attribute>,
    label: &'static str,
    value: Option<&str>,
) {
    if let Some(value) = value.filter(|value| !value.is_empty()) {
        attributes.push(Attribute::string(label, value));
    }
}

fn push_optional_date_attribute(
    attributes: &mut Vec<Attribute>,
    label: &'static str,
    value: Option<&PublicationDate>,
) {
    if let Some(value) = value.and_then(typeql_date_value) {
        attributes.push(Attribute {
            label,
            value: TypeqlValue::Date(value),
        });
    }
}

fn typeql_date_value(date: &PublicationDate) -> Option<String> {
    Some(format!(
        "{:04}-{:02}-{:02}",
        date.year, date.month?, date.day?
    ))
}

fn push_optional_integer_attribute(
    attributes: &mut Vec<Attribute>,
    label: &'static str,
    value: Option<i64>,
) {
    if let Some(value) = value {
        attributes.push(Attribute {
            label,
            value: TypeqlValue::Integer(value),
        });
    }
}

fn put_entity_query(entity: &EntityRef) -> String {
    format!("put {};", entity_pattern(entity))
}

fn put_relation_query(
    variable: &str,
    type_label: &str,
    role_players: &[(&str, &EntityRef)],
) -> String {
    let mut query = String::from("match\n");

    for (_, player) in role_players {
        let _ = writeln!(query, "  {};", entity_pattern(player));
    }

    let _ = write!(query, "put ${variable} isa {type_label}, links (");

    for (index, (role, player)) in role_players.iter().enumerate() {
        if index > 0 {
            query.push_str(", ");
        }
        let _ = write!(query, "{role}: ${}", player.variable);
    }

    query.push_str(");");
    query
}

fn entity_pattern(entity: &EntityRef) -> String {
    let mut pattern = format!("${} isa {}", entity.variable, entity.type_label);

    for attribute in &entity.attributes {
        let _ = write!(pattern, ", has {} {}", attribute.label, attribute.value);
    }

    pattern
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct EntityRef {
    variable: &'static str,
    type_label: &'static str,
    attributes: Vec<Attribute>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct Attribute {
    label: &'static str,
    value: TypeqlValue,
}

impl Attribute {
    fn string(label: &'static str, value: &str) -> Self {
        Self {
            label,
            value: TypeqlValue::String(value.to_owned()),
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
enum TypeqlValue {
    String(String),
    Date(String),
    Integer(i64),
}

impl std::fmt::Display for TypeqlValue {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            TypeqlValue::String(value) => formatter.write_str(&typeql_string_literal(value)),
            TypeqlValue::Date(value) => formatter.write_str(value),
            TypeqlValue::Integer(value) => write!(formatter, "{value}"),
        }
    }
}

fn typeql_string_literal(value: &str) -> String {
    serde_json::to_string(value).expect("serializing a string literal cannot fail")
}

#[cfg(test)]
mod tests {
    use crate::models::domain::{
        Affiliation, Authoring, Citation, DocumentContent, Funding, Literature, LiteratureCore,
        Paper, PaperGraph, PaperMetadata, PdfExtractionData, Project, Publication,
        ScientificLiterature, SourceHash,
    };
    use typedb_driver::Error as TypedbDriverError;

    use super::*;

    #[test]
    fn typeql_string_literals_escape_quotes_backslashes_and_control_characters() {
        assert_eq!(
            typeql_string_literal("A \"quoted\" value\\with\nnewline"),
            r#""A \"quoted\" value\\with\nnewline""#
        );
    }

    #[test]
    fn incomplete_publication_dates_are_not_exported_as_fabricated_typeql_dates() {
        let year_only = PublicationDate {
            year: 2024,
            month: None,
            day: None,
        };
        let year_month = PublicationDate {
            year: 2024,
            month: Some(6),
            day: None,
        };
        let complete = PublicationDate {
            year: 2024,
            month: Some(6),
            day: Some(9),
        };

        assert_eq!(typeql_date_value(&year_only), None);
        assert_eq!(typeql_date_value(&year_month), None);
        assert_eq!(typeql_date_value(&complete).as_deref(), Some("2024-06-09"));
    }

    #[test]
    fn type_db_isolation_conflicts_are_retryable() {
        let error = TypedbDriverError::Other(
            "[STC2] Commit in database 'scepa' failed with isolation conflict: Transaction uses a lock held by a concurrent commit."
                .to_owned(),
        );
        let wrapped = TypedbExportError::Commit {
            attempts: 1,
            source: Box::new(error),
        };

        assert!(is_isolation_conflict(match &wrapped {
            TypedbExportError::Commit { source, .. } => source,
            _ => unreachable!(),
        }));
        assert!(should_retry_export(&wrapped));
    }

    #[test]
    fn retry_delay_doubles_until_the_maximum() {
        assert_eq!(
            next_retry_delay(Duration::from_millis(50)),
            Duration::from_millis(100)
        );
        assert_eq!(next_retry_delay(MAX_RETRY_DELAY), MAX_RETRY_DELAY);
    }

    #[test]
    fn paper_queries_export_literature_people_institutions_and_relations() {
        let source = SourceHash::from_bytes(b"paper");
        let paper = Paper {
            source: source.clone(),
            graph: PaperGraph {
                literature: Literature::Scientific(ScientificLiterature {
                    core: LiteratureCore {
                        title: Some("Understanding TypeDB".to_owned()),
                        publishing_date: Some(PublicationDate {
                            year: 2024,
                            month: Some(6),
                            day: Some(9),
                        }),
                        issn: Some("1234-5678".to_owned()),
                        isbn: None,
                    },
                    doi: Some("10.1234/example".to_owned()),
                }),
                authorings: vec![Authoring {
                    author: Author {
                        forename: Some("Ada".to_owned()),
                        surname: Some("Lovelace".to_owned()),
                    },
                    affiliations: vec![Affiliation {
                        institution: Institution {
                            name: Some("Analytical Engines Lab".to_owned()),
                            kind: InstitutionKind::University,
                            ror_id: Some("https://ror.org/00b30xv10".to_owned()),
                        },
                        department: Some(Department {
                            name: Some("Computing".to_owned()),
                        }),
                    }],
                }],
                publications: vec![Publication {
                    publisher: Institution {
                        name: Some("Knowledge Press".to_owned()),
                        kind: InstitutionKind::Institution,
                        ror_id: None,
                    },
                }],
                citations: vec![Citation {
                    id: "ref-1".to_owned(),
                    cited: Literature::Scientific(ScientificLiterature {
                        core: LiteratureCore {
                            title: Some("Cited Work".to_owned()),
                            publishing_date: Some(PublicationDate {
                                year: 2020,
                                month: None,
                                day: None,
                            }),
                            issn: None,
                            isbn: None,
                        },
                        doi: Some("10.1234/cited".to_owned()),
                    }),
                    authorings: vec![Authoring {
                        author: Author {
                            forename: Some("Grace".to_owned()),
                            surname: Some("Hopper".to_owned()),
                        },
                        affiliations: vec![],
                    }],
                    journal: Some("Journal of Citations".to_owned()),
                }],
                fundings: vec![Funding {
                    funder: Institution {
                        name: Some("National Science Foundation".to_owned()),
                        kind: InstitutionKind::GovernmentInstitution,
                        ror_id: Some("https://ror.org/021nxhr62".to_owned()),
                    },
                    project: Project {
                        name: None,
                        number: Some(1234567),
                    },
                }],
            },
            metadata: PaperMetadata::default(),
            content: DocumentContent::default(),
            extraction_data: PdfExtractionData::default(),
        };

        let source_literal = typeql_string_literal(source.as_str());
        let citation_source_literal =
            typeql_string_literal(&citation_source(source.as_str(), "ref-1"));
        let queries = paper_typeql_queries(&paper);

        assert!(queries.iter().any(|query| {
            query == &format!(
                "put $literature isa scientific-literature, has source {source_literal}, has title \"Understanding TypeDB\", has publishing-date 2024-06-09, has issn \"1234-5678\", has doi \"10.1234/example\";"
            )
        }));
        assert!(queries.iter().any(|query| {
            query.contains(
                "put $authoring isa authoring, links (literature: $literature, author: $author);",
            )
        }));
        assert!(queries.iter().any(|query| {
            query.contains("isa university, has source")
                && query.contains("has name \"Analytical Engines Lab\"")
                && query.contains("has ror-id \"https://ror.org/00b30xv10\"")
        }));
        assert!(queries.iter().any(|query| {
            query.contains("put $institutional_structure isa institutional-structure, links (department: $department, institution: $institution);")
        }));
        assert!(queries.iter().any(|query| {
            query.contains("put $publication isa publication, links (literature: $literature, publisher: $publisher);")
        }));
        assert!(queries.iter().any(|query| {
            query == &format!(
                "put $cited isa scientific-literature, has source {citation_source_literal}, has title \"Cited Work\", has doi \"10.1234/cited\";"
            )
        }));
        assert!(queries.iter().any(|query| {
            query
                .contains("put $citation isa citation, links (citing: $literature, cited: $cited);")
        }));
        assert!(queries.iter().any(|query| {
            query.contains("isa government-institution, has source")
                && query.contains("has name \"National Science Foundation\"")
                && query.contains("has ror-id \"https://ror.org/021nxhr62\"")
        }));
        assert!(queries.iter().any(|query| {
            query.contains("isa project, has source")
                && query.contains("has project-number 1234567")
        }));
        assert!(queries.iter().any(|query| {
            query.contains(
                "put $research_activity isa research-activity, links (institution: $institution, project: $project);",
            )
        }));
        assert!(queries.iter().any(|query| {
            query.contains(
                "put $project_literature isa project-literature, links (project: $project, literature: $literature);",
            )
        }));
    }
}
