use axum_extra::extract::cookie::{Cookie, SameSite};
use serde::{Deserialize, Serialize};

pub const SESSION_COOKIE: &str = "scepa_session";
pub const OAUTH_FLOW_COOKIE: &str = "scepa_oauth_flow";

/// The authenticated browser session, held in a signed (tamper-evident, not
/// encrypted -- these claims aren't secret) cookie. Mirrors what studio's
/// `SessionMiddleware` stores in `request.session["user"]`.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SessionUser {
    pub sub: String,
    pub email: Option<String>,
}

impl SessionUser {
    pub fn into_cookie(self) -> Cookie<'static> {
        let value = serde_json::to_string(&self).expect("SessionUser always serializes");
        build_cookie(SESSION_COOKIE, value)
    }

    pub fn from_cookie(cookie: &Cookie) -> Option<Self> {
        serde_json::from_str(cookie.value()).ok()
    }
}

/// Short-lived state carried between `/auth/login` and `/auth/callback`,
/// since this server has no session store to hold it in server-side.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct OAuthFlowState {
    pub csrf_token: String,
    pub pkce_verifier: String,
    pub nonce: String,
}

impl OAuthFlowState {
    pub fn into_cookie(self) -> Cookie<'static> {
        let value = serde_json::to_string(&self).expect("OAuthFlowState always serializes");
        build_cookie(OAUTH_FLOW_COOKIE, value)
    }

    pub fn from_cookie(cookie: &Cookie) -> Option<Self> {
        serde_json::from_str(cookie.value()).ok()
    }
}

fn build_cookie(name: &'static str, value: String) -> Cookie<'static> {
    Cookie::build((name, value))
        .path("/")
        .http_only(true)
        .same_site(SameSite::Lax)
        // TODO: set true once this server is deployed behind HTTPS.
        .secure(false)
        .build()
}
