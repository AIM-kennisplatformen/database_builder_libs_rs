use std::{path::PathBuf, sync::Arc};

use axum::{
    Json, Router,
    extract::{FromRef, Multipart, Path, Query, State},
    http::{
        HeaderValue, Method, StatusCode,
        header::{AUTHORIZATION, CONTENT_TYPE},
    },
    response::{IntoResponse, Redirect, Response},
    routing::get,
};
use axum_extra::extract::cookie::{Cookie, Key, SignedCookieJar};
use openidconnect::{
    AuthorizationCode, CsrfToken, Nonce, PkceCodeChallenge, PkceCodeVerifier, Scope, TokenResponse,
    core::CoreAuthenticationFlow,
};
use serde::Deserialize;
use tower_http::cors::CorsLayer;

use crate::{
    ingestion::{
        extract::grobid::source::GrobidSource, metadata::UploadInterfaceMetadata,
        parse::tei::reader::parse_tei_xml_str, transform::tei::paper_from_tei,
    },
    models::domain::SourceHash,
    models::paths::pdf::PdfPath,
    server::{
        auth::{ApiKeys, AuthorizedCaller},
        metadata_store::MetadataStore,
        oidc::{OidcConfig, oidc_client},
        session::{OAUTH_FLOW_COOKIE, OAuthFlowState, SESSION_COOKIE, SessionUser},
    },
};

#[derive(Clone)]
pub struct AppState {
    pub grobid: Arc<GrobidSource>,
    pub metadata_store: Arc<MetadataStore>,
    pub api_keys: ApiKeys,
    pub oidc: Arc<OidcConfig>,
    pub cookie_key: Key,
    pub frontend_url: String,
    pub logout_url: String,
}

impl FromRef<AppState> for ApiKeys {
    fn from_ref(state: &AppState) -> Self {
        state.api_keys.clone()
    }
}

impl FromRef<AppState> for Key {
    fn from_ref(state: &AppState) -> Self {
        state.cookie_key.clone()
    }
}

pub fn router(state: AppState) -> Router {
    // credentialed cross-origin requests (the browser calling this server
    // directly with its session cookie) require an explicit origin --
    // `allow_credentials(true)` can't be combined with a wildcard.
    let origin: HeaderValue = state
        .frontend_url
        .parse()
        .expect("FRONTEND_URL must be a valid header value");

    let cors = CorsLayer::new()
        .allow_origin(origin)
        .allow_credentials(true)
        .allow_methods([Method::GET, Method::PUT, Method::PATCH])
        .allow_headers([CONTENT_TYPE, AUTHORIZATION]);

    Router::new()
        .route(
            "/metadata/{sha256}",
            get(get_metadata).put(put_metadata).patch(patch_metadata),
        )
        .route("/auth/login", get(login))
        .route("/auth/callback", get(callback))
        .route("/auth/logout", get(logout))
        .route("/me", get(me))
        .layer(cors)
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
    _caller: AuthorizedCaller,
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
    _caller: AuthorizedCaller,
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
    _caller: AuthorizedCaller,
    Json(metadata): Json<UploadInterfaceMetadata>,
) -> Result<Json<UploadInterfaceMetadata>, ApiError> {
    let sha256 = SourceHash::parse(sha256).ok_or(ApiError::InvalidSha256)?;

    state
        .metadata_store
        .save(sha256.as_str(), &metadata)
        .map_err(ApiError::Store)?;

    Ok(Json(metadata))
}

// -------------------------------------------------------
// Authentik OIDC login flow (mirrors studio's auth.py)
// -------------------------------------------------------

/// Redirects the browser to Authentik. The CSRF state, PKCE verifier, and
/// nonce generated for this attempt are stashed in a short-lived signed
/// cookie -- this server has no session store to hold them in otherwise.
async fn login(State(state): State<AppState>, jar: SignedCookieJar) -> impl IntoResponse {
    let client = oidc_client!(state.oidc);

    let (pkce_challenge, pkce_verifier) = PkceCodeChallenge::new_random_sha256();
    let (auth_url, csrf_token, nonce) = client
        .authorize_url(
            CoreAuthenticationFlow::AuthorizationCode,
            CsrfToken::new_random,
            Nonce::new_random,
        )
        // CoreAuthenticationFlow::AuthorizationCode already adds "openid".
        .add_scope(Scope::new("email".to_owned()))
        .add_scope(Scope::new("profile".to_owned()))
        .set_pkce_challenge(pkce_challenge)
        .url();

    let flow = OAuthFlowState {
        csrf_token: csrf_token.secret().clone(),
        pkce_verifier: pkce_verifier.secret().clone(),
        nonce: nonce.secret().clone(),
    };

    let jar = jar.add(flow.into_cookie());
    (jar, Redirect::to(auth_url.as_str()))
}

#[derive(Deserialize)]
struct CallbackParams {
    code: String,
    state: String,
}

/// Exchanges the authorization code for tokens, verifies the ID token
/// (signature, issuer, audience, and nonce -- all handled by
/// `openidconnect`), and sets the long-lived session cookie.
async fn callback(
    State(state): State<AppState>,
    jar: SignedCookieJar,
    Query(params): Query<CallbackParams>,
) -> Result<impl IntoResponse, (StatusCode, String)> {
    let flow = jar
        .get(OAUTH_FLOW_COOKIE)
        .and_then(|cookie| OAuthFlowState::from_cookie(&cookie))
        .ok_or((
            StatusCode::BAD_REQUEST,
            "missing or expired OAuth flow state".to_owned(),
        ))?;

    if params.state != flow.csrf_token {
        return Err((StatusCode::BAD_REQUEST, "state mismatch".to_owned()));
    }

    let client = oidc_client!(state.oidc);

    let http_client = openidconnect::reqwest::Client::builder()
        .redirect(openidconnect::reqwest::redirect::Policy::none())
        .build()
        .map_err(|error| (StatusCode::INTERNAL_SERVER_ERROR, error.to_string()))?;

    let token_response = client
        .exchange_code(AuthorizationCode::new(params.code))
        .map_err(|error| (StatusCode::BAD_GATEWAY, error.to_string()))?
        .set_pkce_verifier(PkceCodeVerifier::new(flow.pkce_verifier))
        .request_async(&http_client)
        .await
        .map_err(|error| (StatusCode::BAD_GATEWAY, error.to_string()))?;

    let id_token = token_response.id_token().ok_or((
        StatusCode::BAD_GATEWAY,
        "provider did not return an ID token".to_owned(),
    ))?;

    let verifier = client.id_token_verifier();
    let nonce = Nonce::new(flow.nonce);
    let claims = id_token
        .claims(&verifier, &nonce)
        .map_err(|error| (StatusCode::UNAUTHORIZED, error.to_string()))?;

    let user = SessionUser {
        sub: claims.subject().as_str().to_owned(),
        email: claims.email().map(|email| email.as_str().to_owned()),
    };

    let jar = jar
        .remove(Cookie::from(OAUTH_FLOW_COOKIE))
        .add(user.into_cookie());

    Ok((jar, Redirect::to(&state.frontend_url)))
}

/// Clears the session cookie and sends the browser to Authentik's
/// end-session endpoint.
async fn logout(State(state): State<AppState>, jar: SignedCookieJar) -> impl IntoResponse {
    let jar = jar.remove(Cookie::from(SESSION_COOKIE));
    (jar, Redirect::to(&state.logout_url))
}

/// Returns the current session's user, if any -- lets the frontend check
/// auth status without needing a real data request to fail first.
async fn me(jar: SignedCookieJar) -> Result<Json<serde_json::Value>, StatusCode> {
    let user = jar
        .get(SESSION_COOKIE)
        .and_then(|cookie| SessionUser::from_cookie(&cookie))
        .ok_or(StatusCode::UNAUTHORIZED)?;

    Ok(Json(
        serde_json::json!({ "authenticated": true, "user": user }),
    ))
}
