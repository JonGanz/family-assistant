use base64::Engine as _;
use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use chrono::{Duration, Utc};
use serde::Deserialize;
use url::Url;

use crate::{ConnectorError, Http, TokenSet, error::Result};

pub const DEFAULT_AUTH_URL: &str = "https://accounts.google.com/o/oauth2/v2/auth";
pub const DEFAULT_TOKEN_URL: &str = "https://oauth2.googleapis.com/token";

#[derive(Debug, Clone)]
pub struct GoogleOAuth {
    pub client_id: String,
    pub client_secret: String,
    pub auth_url: String,
    pub token_url: String,
}

#[derive(Deserialize)]
struct TokenResponse {
    access_token: String,
    refresh_token: Option<String>,
    expires_in: Option<i64>,
    scope: Option<String>,
    id_token: Option<String>,
}

#[derive(Deserialize)]
struct ErrorResponse {
    error: Option<String>,
}

impl GoogleOAuth {
    pub fn new(client_id: impl Into<String>, client_secret: impl Into<String>) -> Self {
        Self {
            client_id: client_id.into(),
            client_secret: client_secret.into(),
            auth_url: DEFAULT_AUTH_URL.into(),
            token_url: DEFAULT_TOKEN_URL.into(),
        }
    }

    /// Authorisation URL: code flow, PKCE S256, offline access, forced consent.
    pub fn authorize_url(
        &self,
        redirect: &str,
        scopes: &[&str],
        state: &str,
        pkce_challenge: &str,
    ) -> Result<String> {
        let mut url =
            Url::parse(&self.auth_url).map_err(|e| ConnectorError::Parse(e.to_string()))?;
        url.query_pairs_mut()
            .append_pair("client_id", &self.client_id)
            .append_pair("redirect_uri", redirect)
            .append_pair("response_type", "code")
            .append_pair("scope", &scopes.join(" "))
            .append_pair("state", state)
            .append_pair("code_challenge", pkce_challenge)
            .append_pair("code_challenge_method", "S256")
            .append_pair("access_type", "offline")
            .append_pair("include_granted_scopes", "true")
            .append_pair("prompt", "consent");
        Ok(url.into())
    }

    pub async fn exchange_code(
        &self,
        http: &Http,
        code: &str,
        redirect: &str,
        pkce_verifier: &str,
    ) -> Result<TokenSet> {
        self.token_request(
            http,
            &[
                ("grant_type", "authorization_code"),
                ("code", code),
                ("redirect_uri", redirect),
                ("code_verifier", pkce_verifier),
            ],
        )
        .await
    }

    pub async fn refresh(&self, http: &Http, refresh_token: &str) -> Result<TokenSet> {
        self.token_request(
            http,
            &[
                ("grant_type", "refresh_token"),
                ("refresh_token", refresh_token),
            ],
        )
        .await
    }

    async fn token_request(&self, http: &Http, extra: &[(&str, &str)]) -> Result<TokenSet> {
        let url = http.parse_url(&self.token_url)?;
        let mut form: Vec<(&str, &str)> = vec![
            ("client_id", &self.client_id),
            ("client_secret", &self.client_secret),
        ];
        form.extend_from_slice(extra);
        let resp = http
            .send(http.request(reqwest::Method::POST, &url)?.form(&form))
            .await?;
        let status = resp.status();
        let body = http.body(resp).await?;
        if !status.is_success() {
            let err: Option<ErrorResponse> = serde_json::from_slice(&body).ok();
            if err.and_then(|e| e.error).as_deref() == Some("invalid_grant") {
                return Err(ConnectorError::Unauthorized);
            }
            Http::check_status(status)?;
            return Err(ConnectorError::Http(status.as_u16()));
        }
        let tr: TokenResponse =
            serde_json::from_slice(&body).map_err(|e| ConnectorError::Parse(e.to_string()))?;
        Ok(TokenSet {
            access: tr.access_token,
            refresh: tr.refresh_token,
            expires_at: Utc::now() + Duration::seconds(tr.expires_in.unwrap_or(3600)),
            scopes: tr
                .scope
                .map(|s| s.split_whitespace().map(str::to_owned).collect())
                .unwrap_or_default(),
            id_token_email: tr.id_token.as_deref().and_then(email_from_id_token),
        })
    }
}

/// Reads the `email` claim from the (already TLS-delivered) id_token payload. No signature check.
fn email_from_id_token(jwt: &str) -> Option<String> {
    let payload = jwt.split('.').nth(1)?;
    let bytes = URL_SAFE_NO_PAD.decode(payload.trim_end_matches('=')).ok()?;
    let v: serde_json::Value = serde_json::from_slice(&bytes).ok()?;
    v.get("email")?.as_str().map(str::to_owned)
}
