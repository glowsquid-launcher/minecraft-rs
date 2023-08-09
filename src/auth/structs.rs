use chrono::{DateTime, Duration, Utc};
use oauth2::{AuthorizationCode, CsrfToken};
use serde::Deserialize;
use veil::Redact;

#[derive(Redact, Deserialize)]
pub struct OauthCode {
    #[redact]
    code: String,
    state: String,
}

impl OauthCode {
    #[must_use]
    pub fn validate(&self, csrf: &CsrfToken) -> bool {
        &self.state == csrf.secret()
    }
}

impl From<OauthCode> for AuthorizationCode {
    fn from(val: OauthCode) -> Self {
        Self::new(val.code)
    }
}

pub struct XboxLiveResponse;
pub struct XstsResponse;

#[derive(Redact, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct XboxResponse<T> {
    #[serde(skip)]
    marker: std::marker::PhantomData<T>,
    #[redact]
    token: String,
    display_claims: DisplayClaims,
}

impl<T> XboxResponse<T> {
    #[must_use]
    pub fn token(&self) -> &str {
        &self.token
    }

    #[must_use]
    pub fn uhs(&self) -> Option<&str> {
        self.display_claims.xui.first().map(|xui| xui.uhs.as_str())
    }
}

#[derive(Debug, Deserialize)]
struct DisplayClaims {
    xui: Vec<Xui>,
}

#[derive(Debug, Deserialize)]
struct Xui {
    uhs: String,
}

#[derive(Redact, Deserialize)]
pub(in crate::auth) struct MinecraftResponse {
    pub username: String,
    #[redact]
    pub access_token: String,
    pub expires_in: i64,
}

#[derive(Redact, Clone)]
pub struct MinecraftToken {
    pub username: String,
    #[redact]
    pub access_token: String,
    pub expires_at: DateTime<Utc>,
}

#[derive(Debug, Deserialize, Clone)]
pub struct MinecraftProfile {
    id: String,
    name: String,
    skins: Vec<Skin>,
    capes: Vec<Option<serde_json::Value>>,
}

impl MinecraftProfile {
    #[must_use]
    pub fn id(&self) -> &str {
        self.id.as_ref()
    }
}

#[derive(Debug, Deserialize, Clone)]
pub struct Skin {
    id: String,
    state: String,
    url: String,
    variant: String,
    alias: Option<String>,
}

impl From<MinecraftResponse> for MinecraftToken {
    fn from(val: MinecraftResponse) -> Self {
        Self {
            username: val.username,
            access_token: val.access_token,
            expires_at: Utc::now() + Duration::seconds(val.expires_in),
        }
    }
}
