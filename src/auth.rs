use std::{error::Error, fmt::Display};

use error_stack::{ensure, IntoReport, Result, ResultExt};
use oauth2::{
    basic::{BasicClient, BasicTokenType},
    reqwest::async_http_client,
    url::{ParseError, Url},
    AuthUrl, AuthorizationCode, ClientId, ClientSecret, CsrfToken, EmptyExtraTokenFields,
    PkceCodeChallenge, PkceCodeVerifier, RedirectUrl, Scope, StandardTokenResponse, TokenResponse,
    TokenUrl,
};
use serde::Deserialize;
use serde_json::json;

#[derive(Debug, Clone)]
pub struct MSOauth(BasicClient, reqwest::Client);

const AUTH_URL: &str = "https://login.microsoftonline.com/consumers/oauth2/v2.0/authorize";
const TOKEN_URL: &str = "https://login.microsoftonline.com/consumers/oauth2/v2.0/token";

#[derive(Debug)]
pub enum GetXboxTokenError {
    OauthError,
    XboxLiveError,
}

impl Display for GetXboxTokenError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            Self::OauthError => "Error during oauth2 protocol",
            Self::XboxLiveError => "Error during xbox live protocol",
        })
    }
}
impl Error for GetXboxTokenError {}

impl MSOauth {
    /// Create a new [`MSOauth`] client.
    ///
    /// # Errors
    /// Errors if parsing the redirect uri fails
    pub fn new(
        redirect_uri: String,
        client_id: String,
        client_secret: String,
    ) -> Result<Self, ParseError> {
        let client = BasicClient::new(
            ClientId::new(client_id),
            Some(ClientSecret::new(client_secret)),
            AuthUrl::new(AUTH_URL.to_string())?,
            Some(TokenUrl::new(TOKEN_URL.to_string())?),
        )
        .set_redirect_uri(RedirectUrl::new(redirect_uri)?);

        Ok(Self(client, reqwest::Client::new()))
    }

    pub fn get_auth_url(&self) -> (Url, CsrfToken, PkceCodeVerifier) {
        let (pkce_challenge, pkce_verifier) = PkceCodeChallenge::new_random_sha256();

        let (auth_url, csrf_token) = self
            .0
            .authorize_url(CsrfToken::new_random)
            .add_scope(Scope::new("XboxLive.signin".to_string()))
            .add_scope(Scope::new("offline_access".to_string()))
            .set_pkce_challenge(pkce_challenge)
            .url();

        (auth_url, csrf_token, pkce_verifier)
    }

    async fn get_xbox_token(
        &self,
        access_token: &str,
    ) -> Result<XboxLiveResponse, GetXboxTokenError> {
        let xbox_live_request = self
            .1
            .post("https://user.auth.xboxlive.com/user/authenticate")
            .json(&json!({
                "Properties": {
                    "AuthMethod": "RPS",
                    "SiteName": "user.auth.xboxlive.com",
                    "RpsTicket": format!("d={}", access_token)
                },
                "RelyingParty": "http://auth.xboxlive.com",
                "TokenType": "JWT"
            }))
            .send()
            .await
            .into_report()
            .attach_printable("Failed to send xbox live request")
            .change_context(GetXboxTokenError::XboxLiveError)?;

        xbox_live_request
            .json()
            .await
            .into_report()
            .attach_printable("Failed to deserialize body")
            .change_context(GetXboxTokenError::XboxLiveError)
    }

    async fn get_xsts_token(&self, access_token: &str) -> Result<XstsResponse, GetXboxTokenError> {
        let xsts_request = self
            .1
            .post("https://xsts.auth.xboxlive.com/xsts/authorize")
            .json(&json!({
                "Properties": {
                    "SandboxId": "RETAIL",
                    "UserTokens": [access_token]
                },
                "RelyingParty": "rp://api.minecraftservices.com/",
                "TokenType": "JWT"
            }))
            .send()
            .await
            .into_report()
            .attach_printable("Failed to send xsts request")
            .change_context(GetXboxTokenError::XboxLiveError)?;

        xsts_request
            .json()
            .await
            .into_report()
            .attach_printable("Failed to deserialize body")
            .change_context(GetXboxTokenError::XboxLiveError)
    }

    /// Get a minecraft token from a microsoft token
    ///
    /// # Errors
    /// Errors if the token is invalid or one of the requests fails.
    /// This can happen if the user does not own minecraft of if the token is expired
    pub async fn get_minecraft_token<T: oauth2::TokenType>(
        &self,
        ms_token: impl TokenResponse<T>,
    ) -> Result<MinecraftResponse, GetXboxTokenError> {
        let access_token = ms_token.access_token().secret();
        let xbox_live_response = self.get_xbox_token(access_token).await?;

        let token = &xbox_live_response.token;
        let user_hash = &xbox_live_response
            .display_claims
            .xui
            .first()
            .ok_or(GetXboxTokenError::XboxLiveError)
            .into_report()
            .attach_printable("No xui claims found")?
            .uhs;

        let xsts_response = self.get_xsts_token(token).await?;

        let xsts_token = &xsts_response.token;
        let uhs = &xsts_response
            .display_claims
            .xui
            .first()
            .ok_or(GetXboxTokenError::XboxLiveError)
            .into_report()
            .attach_printable("No xui claims found")?
            .uhs;

        ensure!(uhs == user_hash, GetXboxTokenError::XboxLiveError);

        let minecraft_request = self
            .1
            .post("https://api.minecraftservices.com/authentication/login_with_xbox")
            .json(&json!({
                "identityToken": format!("XBL3.0 x={};{}", uhs, xsts_token),
            }))
            .send()
            .await
            .into_report()
            .attach_printable("Failed to send minecraft request")
            .change_context(GetXboxTokenError::XboxLiveError)?;

        minecraft_request
            .json()
            .await
            .into_report()
            .attach_printable("Failed to deserialize body")
            .change_context(GetXboxTokenError::XboxLiveError)
    }

    /// Requests a microsoft access token
    ///
    /// # Errors
    /// Errors if the code is invalid, the CSRF fails, the PKCE fails, or the request fails
    pub async fn get_ms_access_token(
        &self,
        code: OauthCode,
        csrf: CsrfToken,
        pkce: &PkceCodeVerifier,
    ) -> Result<StandardTokenResponse<EmptyExtraTokenFields, BasicTokenType>, GetXboxTokenError>
    {
        ensure!(&code.state == csrf.secret(), GetXboxTokenError::OauthError);

        let pkce = PkceCodeVerifier::new(pkce.secret().to_string());
        self.0
            .exchange_code(AuthorizationCode::new(code.code))
            .set_pkce_verifier(pkce)
            .request_async(async_http_client)
            .await
            .into_report()
            .change_context(GetXboxTokenError::OauthError)
    }

    /// Refreshes a microsoft access token
    ///
    /// # Errors
    /// Errors if the refresh token does not exist or the request fails.
    /// This can happen if the refresh token has been revoked by the user
    pub async fn refresh_ms_access_token<T: oauth2::TokenType>(
        &self,
        response: impl TokenResponse<T>,
    ) -> Result<StandardTokenResponse<EmptyExtraTokenFields, BasicTokenType>, GetXboxTokenError>
    {
        let refresh_token = response
            .refresh_token()
            .ok_or(GetXboxTokenError::OauthError)
            .into_report()
            .attach_printable("No refresh token found")?;

        self.0
            .exchange_refresh_token(refresh_token)
            .request_async(async_http_client)
            .await
            .into_report()
            .change_context(GetXboxTokenError::OauthError)
    }
}

#[derive(Debug, Deserialize)]
pub struct OauthCode {
    code: String,
    state: String,
}

#[derive(Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct XboxLiveResponse {
    token: String,
    display_claims: DisplayClaims,
}

#[derive(Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct XstsResponse {
    token: String,
    display_claims: DisplayClaims,
}

#[derive(Deserialize)]
struct DisplayClaims {
    xui: Vec<Xui>,
}

#[derive(Deserialize)]
struct Xui {
    uhs: String,
}

#[derive(Debug, Deserialize)]
pub struct MinecraftResponse {
    pub username: String,
    pub access_token: String,
    pub expires_in: i64,
}
