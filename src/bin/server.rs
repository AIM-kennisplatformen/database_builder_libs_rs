use std::sync::Arc;

use anyhow::{Context, Result};
use clap::Parser;
use database_builder_scepa_rs::{
    ingestion::extract::grobid::{config::GrobidConfig, source::GrobidSource},
    server::{
        auth::ApiKeys,
        metadata_store::MetadataStore,
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

    let state = AppState {
        grobid,
        metadata_store,
        api_keys,
    };

    let listener = TcpListener::bind(&env.metadata_server_address)
        .await
        .with_context(|| format!("binding to {}", env.metadata_server_address))?;

    println!(
        "Listening on {} (PUT/GET /metadata/{{sha256}})",
        env.metadata_server_address
    );

    axum::serve(listener, router(state))
        .await
        .context("running metadata server")
}
