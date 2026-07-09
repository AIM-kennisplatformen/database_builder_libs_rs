use anyhow::{Context, Result};
use openidconnect::core::CoreProviderMetadata;
use openidconnect::{ClientId, ClientSecret, IssuerUrl, RedirectUrl};

const WELL_KNOWN_SUFFIX: &str = "/.well-known/openid-configuration";

/// Discovered Authentik OIDC provider configuration, fetched once at
/// startup. Deliberately *not* a single fully-built `CoreClient`: each
/// `set_*` builder call changes that type's generic parameters, so it's
/// simpler to keep the raw pieces here and let each call site build (and
/// have the compiler infer the type of) its own client from cached,
/// network-free discovery metadata -- see `oidc_client!`.
pub struct OidcConfig {
    pub metadata: CoreProviderMetadata,
    pub client_id: ClientId,
    pub client_secret: ClientSecret,
    pub redirect_url: RedirectUrl,
}

impl OidcConfig {
    /// Fetches the provider's discovery document and JSON Web Key Set.
    /// `discovery_url` is the full `.../.well-known/openid-configuration`
    /// URL (matching Authentik's own advertised URL, and studio's
    /// `OAUTH_DISCOVERY_URL` convention) -- openidconnect's discovery API
    /// wants the bare issuer URL instead, so the well-known suffix is
    /// stripped back off before handing it to the crate.
    pub async fn discover(
        discovery_url: &str,
        client_id: String,
        client_secret: String,
        redirect_url: String,
    ) -> Result<Self> {
        let issuer = discovery_url
            .strip_suffix(WELL_KNOWN_SUFFIX)
            .unwrap_or(discovery_url);
        let issuer_url =
            IssuerUrl::new(issuer.to_owned()).context("invalid OAUTH_DISCOVERY_URL")?;

        let http_client = openidconnect::reqwest::Client::builder()
            .redirect(openidconnect::reqwest::redirect::Policy::none())
            .build()
            .context("building the discovery HTTP client")?;

        let metadata = CoreProviderMetadata::discover_async(issuer_url, &http_client)
            .await
            .context("discovering the OIDC provider's metadata")?;

        Ok(Self {
            metadata,
            client_id: ClientId::new(client_id),
            client_secret: ClientSecret::new(client_secret),
            redirect_url: RedirectUrl::new(redirect_url)
                .context("invalid METADATA_SERVER_BASE_URL")?,
        })
    }
}

/// Rebuilds a (network-free) `CoreClient` from cached discovery metadata.
/// A function can't name the built client's type (each `set_*` builder call
/// changes its generic parameters), so this is a macro purely to let call
/// sites get a freshly-built client via local type inference.
macro_rules! oidc_client {
    ($oidc:expr) => {
        openidconnect::core::CoreClient::from_provider_metadata(
            $oidc.metadata.clone(),
            $oidc.client_id.clone(),
            Some($oidc.client_secret.clone()),
        )
        .set_redirect_uri($oidc.redirect_url.clone())
    };
}

pub(crate) use oidc_client;
