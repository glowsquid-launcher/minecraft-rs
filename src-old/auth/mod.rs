use std::time::{Duration as StdDuration, SystemTime};

use async_stream::stream;
use futures::{pin_mut, Stream, StreamExt};
use jwt_simple::prelude::*;
use reqwest::StatusCode;
use serde_json::json;
use time::{format_description::well_known::Rfc3339, Duration, OffsetDateTime};
use tokio::time::sleep;

use crate::{
    auth::structs::JWTEntitlements,
    errors::{
        AuthTokenErrorType, DeviceCodeError, InternalAuthTokenError, InternalJWTError,
        InternalReqwestError, JWTVerificationError, MinecraftProfileError, XstsError,
    },
};

use self::structs::{
    AuthData, AuthToken, AuthTokenError, DeviceCode, MinecraftProductAttachment, MinecraftProfile,
    MinecraftToken, XboxToken, XstsToken,
};
pub mod structs;

const JWT_PUBLIC_KEY: &str = "-----BEGIN PUBLIC KEY-----
MIICIjANBgkqhkiG9w0BAQEFAAOCAg8AMIICCgKCAgEAtz7jy4jRH3psj5AbVS6W
NHjniqlr/f5JDly2M8OKGK81nPEq765tJuSILOWrC3KQRvHJIhf84+ekMGH7iGlO
4DPGDVb6hBGoMMBhCq2jkBjuJ7fVi3oOxy5EsA/IQqa69e55ugM+GJKUndLyHeNn
X6RzRzDT4tX/i68WJikwL8rR8Jq49aVJlIEFT6F+1rDQdU2qcpfT04CBYLM5gMxE
fWRl6u1PNQixz8vSOv8pA6hB2DU8Y08VvbK7X2ls+BiS3wqqj3nyVWqoxrwVKiXR
kIqIyIAedYDFSaIq5vbmnVtIonWQPeug4/0spLQoWnTUpXRZe2/+uAKN1RY9mmaB
pRFV/Osz3PDOoICGb5AZ0asLFf/qEvGJ+di6Ltt8/aaoBuVw+7fnTw2BhkhSq1S/
va6LxHZGXE9wsLj4CN8mZXHfwVD9QG0VNQTUgEGZ4ngf7+0u30p7mPt5sYy3H+Fm
sWXqFZn55pecmrgNLqtETPWMNpWc2fJu/qqnxE9o2tBGy/MqJiw3iLYxf7U+4le4
jM49AUKrO16bD1rdFwyVuNaTefObKjEMTX9gyVUF6o7oDEItp5NHxFm3CqnQRmch
HsMs+NxEnN4E9a8PDB23b4yjKOQ9VHDxBxuaZJU60GBCIOF9tslb7OAkheSJx5Xy
EYblHbogFGPRFU++NrSQRX0CAwEAAQ==
-----END PUBLIC KEY-----";

pub struct Auth {
    /// The client id from azure
    pub client_id: String,
    /// The reqwest http client
    pub http_client: reqwest::Client,
}

impl Auth {
    pub async fn create_device_code(&self) -> Result<DeviceCode, DeviceCodeError> {
        let res = self
            .http_client
            .post("https://login.microsoftonline.com/consumers/oauth2/v2.0/devicecode")
            .form(&[
                ("client_id", &*self.client_id),
                ("scope", "XboxLive.signin offline_access"), // xboxlive is needed for checking if the user actually has minecraft and stuff, offline_access is for refresh token
            ])
            .send()
            .await
            .map_err(InternalReqwestError)?;

        if res.status() == StatusCode::OK {
            Ok(res.json().await.map_err(InternalReqwestError)?)
        } else {
            Err(DeviceCodeError::MicrosoftError(
                res.text().await.map_err(InternalReqwestError)?,
            ))
        }
    }

    /// Before using, please pin_mut!(stream) and then you can use stream.next().await to iterate over the stream
    pub fn pull_for_auth(
        &self,
        device_code: &DeviceCode,
    ) -> impl Stream<Item = Result<AuthToken, AuthTokenError>> {
        let request = self
            .http_client
            .post("https://login.microsoftonline.com/consumers/oauth2/v2.0/token")
            .form(&[
                ("client_id", &*self.client_id),
                ("grant_type", "urn:ietf:params:oauth:grant-type:device_code"),
                ("device_code", &device_code.device_code),
                ("tenant", "consumers"),
            ]);

        let interval = device_code.interval.into();

        stream! {
            loop {
                // Since the body is _not_ a stream, it can be cloned
                let req = request.try_clone().expect("Cannot clone request");
                let res = req.send().await.map_err(|e| AuthTokenError::ReqwestError(e.into()));
                if res.is_err() {
                    yield Err(res.unwrap_err());
                    break;
                }
                let res = res.unwrap();
                if res.status() == StatusCode::OK {
                    yield res.json::<AuthToken>().await.map_err(|e| AuthTokenError::ReqwestError(e.into()));
                    break;
                } else {
                    let err: InternalAuthTokenError = res.json().await.map_err(|e| AuthTokenError::ReqwestError(e.into()))?;

                    match err.error {
                        AuthTokenErrorType::AuthorizationPending => {
                            yield Err(AuthTokenError::ExpectedError(err));
                            sleep(StdDuration::from_secs(interval)).await;
                        },
                        _ => {
                            yield Err(AuthTokenError::ExpectedError(err));
                            break;
                        },

                    }
                }
            }
        }
    }

    /// This function will pull for the auth token until it gets it, and then return it
    /// This function does not give a stream of updates during the process, it just returns the token or an error
    pub async fn get_microsoft_auth_token(
        &self,
        device_code: &DeviceCode,
    ) -> Result<AuthToken, AuthTokenError> {
        let stream = self.pull_for_auth(device_code);
        pin_mut!(stream);

        'block: {
            while let Some(r) = stream.next().await {
                match &r {
                    Err(AuthTokenError::ExpectedError(err)) => match err.error {
                        AuthTokenErrorType::AuthorizationPending => continue,
                        _ => break 'block r,
                    },
                    Err(AuthTokenError::ReqwestError(_)) => break 'block r,
                    Ok(_) => break 'block r,
                }
            }

            unreachable!()
        }
    }

    pub async fn get_xbox_auth_token(
        &self,
        token: &AuthToken,
    ) -> Result<XboxToken, reqwest::Error> {
        self.http_client
            .post("https://user.auth.xboxlive.com/user/authenticate")
            .json(&json!({
            "Properties": {
                "AuthMethod": "RPS",
                "SiteName": "user.auth.xboxlive.com",
                "RpsTicket": format!("d={}", token.access_token)
            },
            "RelyingParty": "http://auth.xboxlive.com",
            "TokenType": "JWT"
                }))
            .send()
            .await?
            .json()
            .await
    }

    pub async fn get_xsts_token(&self, xbl_token: &XboxToken) -> Result<XstsToken, XstsError> {
        let res = self
            .http_client
            .post(" https://xsts.auth.xboxlive.com/xsts/authorize")
            .json(&json!({
                "Properties": {
                    "SandboxId": "RETAIL",
                    "UserTokens": [
                        xbl_token.token
                    ]
                },
                "RelyingParty": "rp://api.minecraftservices.com/",
                "TokenType": "JWT"
            }))
            .send()
            .await
            .map_err(InternalReqwestError)?;

        if res.status() == StatusCode::OK {
            Ok(res.json().await.map_err(InternalReqwestError)?)
        } else {
            Err(XstsError::MicrosoftError(
                res.json().await.map_err(InternalReqwestError)?,
            ))
        }
    }

    pub async fn get_minecraft_token(
        &self,
        xsts_token: &XstsToken,
    ) -> Result<MinecraftToken, reqwest::Error> {
        self.http_client
            .post("https://api.minecraftservices.com/authentication/login_with_xbox")
            .json(&json!({
                "identityToken":
                    format!(
                        "XBL3.0 x={};{}",
                        xsts_token
                            .display_claims
                            .xui
                            .get(0)
                            .expect("Expected display claim xui to have at least one element")
                            .uhs,
                        xsts_token.token
                    )
            }))
            .send()
            .await?
            .json()
            .await
    }

    pub async fn verify_minecraft_ownership(
        &self,
        minecraft_token: &MinecraftToken,
    ) -> Result<bool, JWTVerificationError> {
        let json: MinecraftProductAttachment = self
            .http_client
            .get("https://api.minecraftservices.com/entitlements/mcstore")
            .bearer_auth(&minecraft_token.access_token)
            .send()
            .await
            .map_err(InternalReqwestError)?
            .json()
            .await
            .map_err(InternalReqwestError)?;

        let public_key =
            RS256PublicKey::from_pem(JWT_PUBLIC_KEY).expect("Could not parse public key");
        let claims = public_key
            .verify_token::<JWTEntitlements>(&json.signature, None)
            .map_err(InternalJWTError)?;

        Ok(!json.items.is_empty()
            && claims
                .custom
                .entitlements
                .iter()
                .any(|e| e.name == "game_minecraft"))
    }

    pub async fn get_minecraft_profile(
        &self,
        minecraft_token: &MinecraftToken,
    ) -> Result<MinecraftProfile, MinecraftProfileError> {
        let res = self
            .http_client
            .get("https://api.minecraftservices.com/minecraft/profile")
            .bearer_auth(&minecraft_token.access_token)
            .send()
            .await?;

        if res.status() == StatusCode::OK {
            Ok(res.json().await?)
        } else {
            Err(MinecraftProfileError::NotFound)
        }
    }

    pub fn create_auth_data(
        &self,
        minecraft_token: &MinecraftToken,
        minecraft_profile: &MinecraftProfile,
        microsoft_token: &AuthToken,
        xbox_token: &XboxToken,
    ) -> AuthData {
        let system_time: OffsetDateTime = SystemTime::now().into();
        let expires_in = Duration::seconds(microsoft_token.expires_in as i64);
        let new_time = system_time + expires_in;
        let formatted_time = new_time.format(&Rfc3339).expect("Could not format date");

        AuthData {
            access_token: minecraft_token.access_token.clone(),
            refresh_token: microsoft_token.refresh_token.clone(),
            username: minecraft_profile.name.clone(),
            uuid: minecraft_profile.id.clone(),
            // TODO: figure out of this is correct or not (I forgor :skull:)
            xuid: xbox_token
                .display_claims
                .xui
                .get(0)
                .expect("Failed to get xbox UID")
                .uhs
                .clone(),
            expires_at: formatted_time,
        }
    }

    /// Refreshes the current auth token with a new one
    ///
    /// You will have to re-run the auth flow from `get_xbox_auth_token` if this succeeds, else, you will have to re-run the entire auth flow.
    pub async fn refresh_token(&self, auth_data: &AuthData) -> Result<AuthToken, reqwest::Error> {
        self.http_client
            .post("https://login.microsoftonline.com/consumers/oauth2/v2.0/token")
            .form(&[
                ("client_id", &*self.client_id),
                ("refresh_token", &*auth_data.refresh_token),
                ("grant_type", "refresh_token"),
                ("scope", "XboxLive.signin offline_access"),
            ])
            .send()
            .await?
            .json()
            .await
    }
}

#[cfg(test)]
mod tests {
    use reqwest::Client;

    use super::*;

    #[tokio::test]
    async fn can_create_device_code() {
        let auth = Auth {
            client_id: "2aa32806-92e3-4242-babc-392ac0f0fd30".to_string(),
            http_client: Client::new(),
        };

        auth.create_device_code()
            .await
            .expect("Could not create device code");
    }

    #[allow(dead_code)]
    #[tokio::test]
    async fn test_full_flow() {
        let auth = Auth {
            client_id: "2aa32806-92e3-4242-babc-392ac0f0fd30".to_string(),
            http_client: Client::new(),
        };

        let code = auth.create_device_code().await.unwrap();
        println!("Code: {} | Goto: {}", code.user_code, code.verification_uri);

        let auth_token = auth.get_microsoft_auth_token(&code).await.unwrap();

        println!("{:?}", auth_token);

        let xbl_token = auth
            .get_xbox_auth_token(&auth_token)
            .await
            .expect("Could not authenticate with xbox live");

        println!("{:?}", xbl_token);

        let xsts_token = auth
            .get_xsts_token(&xbl_token)
            .await
            .expect("Could not get xsts token");

        println!("{:?}", xsts_token);

        let minecraft_token = auth
            .get_minecraft_token(&xsts_token)
            .await
            .expect("Could not get minecraft token");

        println!("{:?}", minecraft_token);

        let owns_minecraft = auth
            .verify_minecraft_ownership(&minecraft_token)
            .await
            .expect("Could not verify minecraft ownership");

        if owns_minecraft {
            println!("User owns minecraft");
        } else {
            panic!("User does not own minecraft");
        }

        let minecraft_profile = auth
            .get_minecraft_profile(&minecraft_token)
            .await
            .expect("Could not get minecraft profile");

        println!("{:?}", minecraft_profile);

        let auth_data = auth.create_auth_data(
            &minecraft_token,
            &minecraft_profile,
            &auth_token,
            &xbl_token,
        );

        println!("{:?}", auth_data);

        let new_auth_token = auth
            .refresh_token(&auth_data)
            .await
            .expect("Could not refresh token");

        println!("{:?}", new_auth_token);
    }
}
