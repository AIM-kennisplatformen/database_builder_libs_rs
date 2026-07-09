use std::{path::PathBuf, sync::Arc};

use axum::{
    Json, Router,
    extract::{FromRef, Multipart, Path, State},
    http::StatusCode,
    response::{IntoResponse, Response},
    routing::get,
};

use crate::{
    ingestion::{
        extract::grobid::source::GrobidSource, metadata::UploadInterfaceMetadata,
        parse::tei::reader::parse_tei_xml_str, transform::tei::paper_from_tei,
    },
    models::domain::SourceHash,
    models::paths::pdf::PdfPath,
    server::{
        auth::{ApiKeys, AuthorizedApp},
        metadata_store::MetadataStore,
    },
};

#[derive(Clone)]
pub struct AppState {
    pub grobid: Arc<GrobidSource>,
    pub metadata_store: Arc<MetadataStore>,
    pub api_keys: ApiKeys,
}

impl FromRef<AppState> for ApiKeys {
    fn from_ref(state: &AppState) -> Self {
        state.api_keys.clone()
    }
}

pub fn router(state: AppState) -> Router {
    Router::new()
        .route(
            "/metadata/{sha256}",
            get(get_metadata).put(put_metadata).patch(patch_metadata),
        )
        .with_state(state)
}

enum ApiError {
    InvalidSha256,
    MissingFile,
    HashMismatch { expected: String, actual: String },
    Grobid(anyhow::Error),
    Parse(anyhow::Error),
    Store(anyhow::Error),
    NotFound,
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        let (status, message) = match self {
            ApiError::InvalidSha256 => (
                StatusCode::BAD_REQUEST,
                "sha256 must be a 64-character lowercase hex digest".to_owned(),
            ),
            ApiError::MissingFile => (
                StatusCode::BAD_REQUEST,
                "multipart body must include a 'file' field".to_owned(),
            ),
            ApiError::HashMismatch { expected, actual } => (
                StatusCode::BAD_REQUEST,
                format!("content hash {actual} does not match provided identifier {expected}"),
            ),
            ApiError::Grobid(error) => (
                StatusCode::BAD_GATEWAY,
                format!("GROBID extraction failed: {error}"),
            ),
            ApiError::Parse(error) => (
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("failed to parse GROBID's TEI XML: {error}"),
            ),
            ApiError::Store(error) => (
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("metadata store error: {error}"),
            ),
            ApiError::NotFound => (
                StatusCode::NOT_FOUND,
                "no metadata stored under that sha256".to_owned(),
            ),
        };

        (status, message).into_response()
    }
}

/// Source: accepts a PDF, runs it through GROBID + TEI parsing + the domain
/// transform, and returns the resulting Field-schema metadata. Purely
/// transient -- nothing is written to disk here. upload_interface's form
/// holds the result in memory for editing; only clicking "save" (PATCH)
/// persists anything, so a document is never live-written to storage
/// before the user has actually chosen to keep it.
async fn put_metadata(
    State(state): State<AppState>,
    Path(sha256): Path<String>,
    _app: AuthorizedApp,
    mut multipart: Multipart,
) -> Result<Json<UploadInterfaceMetadata>, ApiError> {
    let sha256 = SourceHash::parse(sha256).ok_or(ApiError::InvalidSha256)?;

    let mut pdf_bytes = None;
    while let Some(field) = multipart
        .next_field()
        .await
        .map_err(|error| ApiError::Grobid(error.into()))?
    {
        if field.name() == Some("file") {
            pdf_bytes = Some(
                field
                    .bytes()
                    .await
                    .map_err(|error| ApiError::Grobid(error.into()))?
                    .to_vec(),
            );
        }
    }
    let pdf_bytes = pdf_bytes.ok_or(ApiError::MissingFile)?;

    let actual = SourceHash::from_bytes(&pdf_bytes);
    if actual.as_str() != sha256.as_str() {
        return Err(ApiError::HashMismatch {
            expected: sha256.as_str().to_owned(),
            actual: actual.as_str().to_owned(),
        });
    }

    // Only used to give GROBID's multipart upload a filename; nothing here
    // reads from disk (see extract_pdf_bytes_to_tei_xml).
    let synthetic_path = PdfPath::try_from(PathBuf::from(format!("{}.pdf", actual.as_str())))
        .expect("a filename built as \"<hex>.pdf\" always has a .pdf extension");

    let tei_xml = state
        .grobid
        .extract_pdf_bytes_to_tei_xml(&synthetic_path, pdf_bytes)
        .await
        .map_err(|error| ApiError::Grobid(error.into()))?;

    let tei_document =
        parse_tei_xml_str(&tei_xml).map_err(|error| ApiError::Parse(error.into()))?;
    let paper = paper_from_tei(&tei_document, actual.clone());
    let metadata = UploadInterfaceMetadata::from_paper(&paper);

    Ok(Json(metadata))
}

/// Sink: retrieves a previously-*saved* result (i.e. the user has clicked
/// "save" on this document at least once before) -- 404 for a document
/// that's only ever been extracted (PUT) but never saved.
async fn get_metadata(
    State(state): State<AppState>,
    Path(sha256): Path<String>,
    _app: AuthorizedApp,
) -> Result<Json<UploadInterfaceMetadata>, ApiError> {
    let sha256 = SourceHash::parse(sha256).ok_or(ApiError::InvalidSha256)?;

    state
        .metadata_store
        .load(sha256.as_str())
        .map_err(ApiError::Store)?
        .map(Json)
        .ok_or(ApiError::NotFound)
}

/// Sink: persists the user's edited fields (upload_interface's save
/// button). The only endpoint that writes to disk -- creates the record on
/// its first call for a given hash, overwrites it on every call after.
async fn patch_metadata(
    State(state): State<AppState>,
    Path(sha256): Path<String>,
    _app: AuthorizedApp,
    Json(metadata): Json<UploadInterfaceMetadata>,
) -> Result<Json<UploadInterfaceMetadata>, ApiError> {
    let sha256 = SourceHash::parse(sha256).ok_or(ApiError::InvalidSha256)?;

    state
        .metadata_store
        .save(sha256.as_str(), &metadata)
        .map_err(ApiError::Store)?;

    Ok(Json(metadata))
}
