use qdrant_client::{Payload, qdrant::PointStruct};
use serde_json::json;
use thiserror::Error;
use uuid::Uuid;

use crate::{
    ingestion::extract::embedding::{error::EmbeddingError, source::EmbeddingSource},
    models::domain::Paper,
    stores::qdrant::store::{QdrantConnected, QdrantStore},
};

/// Namespaces every chunk's point id off a fixed, app-specific string so
/// point ids are deterministic across runs (re-embedding the same paper
/// upserts the same points instead of creating duplicates).
const POINT_ID_NAMESPACE: &str = "database-builder-scepa-rs";

#[derive(Debug, Error)]
pub enum QdrantExportError {
    #[error("failed to embed paper text chunks")]
    Embed {
        #[from]
        source: EmbeddingError,
    },

    #[error("expected embedding vector dimension {expected}, got {actual}")]
    VectorDimensionMismatch { expected: u64, actual: usize },

    #[error("failed to upsert paper chunks into Qdrant")]
    Upsert {
        #[source]
        source: anyhow::Error,
    },
}

pub async fn write_paper_qdrant(
    paper: &Paper,
    embeddings: &EmbeddingSource,
    store: &QdrantStore<QdrantConnected>,
) -> Result<(), QdrantExportError> {
    let source = paper.source.as_str();
    let chunks = collect_chunks(paper);

    if chunks.is_empty() {
        return Ok(());
    }

    let texts: Vec<String> = chunks.iter().map(|chunk| chunk.text.to_owned()).collect();
    let vectors = embeddings.embed(&texts).await?;

    let expected_dimension = store.vector_dimension();
    let mut points = Vec::with_capacity(chunks.len());

    for (chunk, vector) in chunks.iter().zip(vectors) {
        if vector.len() as u64 != expected_dimension {
            return Err(QdrantExportError::VectorDimensionMismatch {
                expected: expected_dimension,
                actual: vector.len(),
            });
        }

        let id = point_id(source, &chunk.locator);
        let payload = chunk_payload(source, chunk);
        points.push(PointStruct::new(id.to_string(), vector, payload));
    }

    store
        .upsert_points(points)
        .await
        .map_err(|source| QdrantExportError::Upsert { source })?;

    Ok(())
}

struct PaperChunk<'a> {
    locator: String,
    kind: &'static str,
    section_index: Option<usize>,
    section_title: Option<&'a str>,
    chunk_index: Option<usize>,
    text: &'a str,
}

fn collect_chunks(paper: &Paper) -> Vec<PaperChunk<'_>> {
    let mut chunks = Vec::new();

    if let Some(abstract_text) = non_empty(paper.metadata.abstract_text.as_deref()) {
        chunks.push(PaperChunk {
            locator: "abstract".to_owned(),
            kind: "abstract",
            section_index: None,
            section_title: None,
            chunk_index: None,
            text: abstract_text,
        });
    }

    for (section_index, section) in paper.content.sections.iter().enumerate() {
        for (chunk_index, text) in section.text_chunks.iter().enumerate() {
            let Some(text) = non_empty(Some(text.as_str())) else {
                continue;
            };

            chunks.push(PaperChunk {
                locator: format!("section:{section_index}:chunk:{chunk_index}"),
                kind: "section",
                section_index: Some(section_index),
                section_title: non_empty(Some(section.title.as_str())),
                chunk_index: Some(chunk_index),
                text,
            });
        }
    }

    chunks
}

fn non_empty(text: Option<&str>) -> Option<&str> {
    text.filter(|text| !text.is_empty())
}

fn point_id(source: &str, locator: &str) -> Uuid {
    Uuid::new_v5(
        &Uuid::NAMESPACE_OID,
        format!("{POINT_ID_NAMESPACE}:{source}:{locator}").as_bytes(),
    )
}

fn chunk_payload(source: &str, chunk: &PaperChunk<'_>) -> Payload {
    json!({
        "source": source,
        "kind": chunk.kind,
        "section_index": chunk.section_index,
        "section_title": chunk.section_title,
        "chunk_index": chunk.chunk_index,
        "text": chunk.text,
    })
    .try_into()
    .expect("chunk payload literal is always a JSON object")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::domain::{
        DocumentContent, Literature, LiteratureCore, Paper, PaperGraph, PaperMetadata,
        PdfExtractionData, ScientificLiterature, Section, SourceHash,
    };

    fn paper_with(abstract_text: Option<&str>, sections: Vec<Section>) -> Paper {
        Paper {
            source: SourceHash::from_bytes(b"chunk-test"),
            graph: PaperGraph {
                literature: Literature::Scientific(ScientificLiterature {
                    core: LiteratureCore {
                        title: None,
                        publishing_date: None,
                        issn: None,
                        isbn: None,
                    },
                    doi: None,
                }),
                authorings: Vec::new(),
                publications: Vec::new(),
                citations: Vec::new(),
            },
            metadata: PaperMetadata {
                abstract_text: abstract_text.map(str::to_owned),
                ..PaperMetadata::default()
            },
            content: DocumentContent {
                sections,
                ..DocumentContent::default()
            },
            extraction_data: PdfExtractionData::default(),
        }
    }

    #[test]
    fn collects_the_abstract_and_every_non_empty_text_chunk() {
        let paper = paper_with(
            Some("this paper is about graphs"),
            vec![
                Section {
                    title: "Introduction".to_owned(),
                    text_chunks: vec!["chunk one".to_owned(), String::new()],
                },
                Section {
                    title: "Methods".to_owned(),
                    text_chunks: vec!["chunk two".to_owned()],
                },
            ],
        );

        let chunks = collect_chunks(&paper);
        let texts: Vec<&str> = chunks.iter().map(|chunk| chunk.text).collect();

        assert_eq!(
            texts,
            vec!["this paper is about graphs", "chunk one", "chunk two"]
        );
        assert_eq!(chunks[0].kind, "abstract");
        assert_eq!(chunks[0].locator, "abstract");
        assert_eq!(chunks[1].kind, "section");
        assert_eq!(chunks[1].locator, "section:0:chunk:0");
        assert_eq!(chunks[1].section_title, Some("Introduction"));
        assert_eq!(chunks[2].locator, "section:1:chunk:0");
    }

    #[test]
    fn skips_papers_with_no_abstract_and_no_chunks() {
        let paper = paper_with(None, vec![]);

        assert!(collect_chunks(&paper).is_empty());
    }

    #[test]
    fn point_ids_are_deterministic_and_distinct_per_locator() {
        let first = point_id("source-a", "abstract");
        let second = point_id("source-a", "abstract");
        let third = point_id("source-a", "section:0:chunk:0");
        let other_source = point_id("source-b", "abstract");

        assert_eq!(first, second);
        assert_ne!(first, third);
        assert_ne!(first, other_source);
    }

    #[test]
    fn chunk_payload_carries_locator_metadata_for_a_section_chunk() {
        let chunk = PaperChunk {
            locator: "section:0:chunk:1".to_owned(),
            kind: "section",
            section_index: Some(0),
            section_title: Some("Introduction"),
            chunk_index: Some(1),
            text: "chunk text",
        };

        let payload: serde_json::Value = chunk_payload("source-a", &chunk).into();

        assert_eq!(payload["source"], "source-a");
        assert_eq!(payload["kind"], "section");
        assert_eq!(payload["section_index"], 0);
        assert_eq!(payload["section_title"], "Introduction");
        assert_eq!(payload["chunk_index"], 1);
        assert_eq!(payload["text"], "chunk text");
    }

    #[test]
    fn chunk_payload_leaves_section_fields_null_for_the_abstract() {
        let chunk = PaperChunk {
            locator: "abstract".to_owned(),
            kind: "abstract",
            section_index: None,
            section_title: None,
            chunk_index: None,
            text: "this paper is about graphs",
        };

        let payload: serde_json::Value = chunk_payload("source-a", &chunk).into();

        assert_eq!(payload["kind"], "abstract");
        assert!(payload["section_index"].is_null());
        assert!(payload["section_title"].is_null());
        assert!(payload["chunk_index"].is_null());
        assert_eq!(payload["text"], "this paper is about graphs");
    }
}
