use std::sync::Arc;

use anyhow::{Context, Result};
use axum_extra::extract::cookie::Key;
use clap::Parser;
use database_builder_scepa_rs::{
    ingestion::extract::grobid::{config::GrobidConfig, source::GrobidSource},
    server::{
        auth::ApiKeys,
        metadata_store::MetadataStore,
        oidc::OidcConfig,
        routes::{AppState, router},
    },
};
use tokio::net::TcpListener;

#[derive(Parser)]
#[command(author, version, about)]
struct Env {
    #[arg(long, env = "GROBID_URL")]
    grobid_url: String,

    #[arg(long, env = "METADATA_SERVER_ADDRESS", default_value = "0.0.0.0:8081")]
    metadata_server_address: String,

    #[arg(long, env = "METADATA_DIR", default_value = "./metadata")]
    metadata_dir: String,

    /// "app-name:key,other-app:key" pairs, e.g. upload_interface's key.
    #[arg(long, env = "METADATA_API_KEYS", default_value = "")]
    metadata_api_keys: String,

    /// This server's own externally-reachable base URL, used to build the
    /// OAuth `redirect_uri` (must match what's registered in Authentik).
    #[arg(long, env = "METADATA_SERVER_BASE_URL")]
    metadata_server_base_url: String,

    #[arg(long, env = "OAUTH_CLIENT_ID")]
    oauth_client_id: String,

    #[arg(long, env = "OAUTH_CLIENT_SECRET")]
    oauth_client_secret: String,

    /// Authentik's full `.../.well-known/openid-configuration` URL.
    #[arg(long, env = "OAUTH_DISCOVERY_URL")]
    oauth_discovery_url: String,

    /// Authentik's end-session URL, redirected to after `/auth/logout`.
    #[arg(long, env = "OAUTH_LOGOUT_URL")]
    oauth_logout_url: String,

    /// Signs the session and OAuth-flow cookies; any length, HKDF-derived
    /// into a proper key.
    #[arg(long, env = "SESSION_SECRET")]
    session_secret: String,

    /// upload_interface's origin -- the CORS-allowed origin and the
    /// post-login redirect target.
    #[arg(long, env = "FRONTEND_URL")]
    frontend_url: String,
}

#[tokio::main]
async fn main() -> Result<()> {
    dotenvy::dotenv().ok();

    let env = Env::parse();

    let grobid = Arc::new(GrobidSource::new(GrobidConfig {
        url: env.grobid_url,
    }));
    let metadata_store = Arc::new(
        MetadataStore::new(env.metadata_dir).context("preparing metadata storage directory")?,
    );
    let api_keys = ApiKeys::parse(&env.metadata_api_keys);

    let oidc = Arc::new(
        OidcConfig::discover(
            &env.oauth_discovery_url,
            env.oauth_client_id,
            env.oauth_client_secret,
            format!("{}/auth/callback", env.metadata_server_base_url),
        )
        .await
        .context("discovering the Authentik OIDC provider")?,
    );
    let cookie_key = Key::derive_from(env.session_secret.as_bytes());

    let state = AppState {
        grobid,
        metadata_store,
        api_keys,
        oidc,
        cookie_key,
        frontend_url: env.frontend_url,
        logout_url: env.oauth_logout_url,
    };

    let listener = TcpListener::bind(&env.metadata_server_address)
        .await
        .with_context(|| format!("binding to {}", env.metadata_server_address))?;

    println!(
        "Listening on {} (PUT/GET/PATCH /metadata/{{sha256}}, /auth/login, /me)",
        env.metadata_server_address
    );

    axum::serve(listener, router(state))
        .await
        .context("running metadata server")
}
