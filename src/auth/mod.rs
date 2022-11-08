use async_stream::try_stream;
use futures::{pin_mut, Stream, StreamExt};
use jwt_simple::prelude::*;
use reqwest::StatusCode;
use serde_json::json;
use tokio::time::{sleep, Duration};

use crate::{
    auth::structs::JWTEntitlements,
    errors::{
        AuthTokenError, AuthTokenErrorType, DeviceCodeError, JWTVerificationError,
        MinecraftProfileError, XstsError,
    },
};

use self::structs::{
    AuthData, AuthToken, AuthTokenRes, DeviceCode, MinecraftProductAttachment, MinecraftProfile,
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
    client_id: String,
    /// The reqwest http client
    http_client: reqwest::Client,
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
            .await?;

        if res.status() == StatusCode::OK {
            Ok(res.json().await?)
        } else {
            Err(DeviceCodeError::MicrosoftError(res.text().await?))
        }
    }

    /// Before using, please pin_mut!(stream) and then you can use stream.next().await to iterate over the stream
    pub fn pull_for_auth(
        &self,
        device_code: &DeviceCode,
    ) -> impl Stream<Item = Result<AuthTokenRes, reqwest::Error>> {
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

        try_stream! {
            loop {
                // Since the body is _not_ a stream, it can be cloned
                let req = request.try_clone().expect("Cannot clone request");
                let res = req.send().await?;
                if res.status() == StatusCode::OK {
                    yield AuthTokenRes::Token(res.json().await?);
                    break;
                } else {
                    let err: AuthTokenError = res.json().await?;

                    match err.error {
                        AuthTokenErrorType::AuthorizationPending => {
                            yield AuthTokenRes::ExpectedError(err);
                            sleep(Duration::from_secs(interval)).await;
                        },
                        _ => {
                            yield AuthTokenRes::ExpectedError(err);
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
    ) -> Result<AuthTokenRes, reqwest::Error> {
        let stream = self.pull_for_auth(device_code);
        pin_mut!(stream);

        let auth_token = 'block: {
            while let Some(v) = stream.next().await {
                let v = v?;

                match &v {
                    AuthTokenRes::ExpectedError(err) => match err.error {
                        AuthTokenErrorType::AuthorizationPending => continue,
                        _ => break 'block v,
                    },
                    AuthTokenRes::Token(_) => break 'block v,
                }
            }

            unreachable!()
        };

        Ok(auth_token)
    }

    pub async fn authenticate_xbox_live(
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
            .await?;

        if res.status() == StatusCode::OK {
            Ok(res.json().await.map_err(XstsError::RequestError)?)
        } else {
            Err(XstsError::MicrosoftError(
                res.json().await.map_err(XstsError::RequestError)?,
            ))
        }
    }

    pub async fn get_minecraft_token(
        &self,
        xsts_token: &XboxToken,
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
            .await?
            .json()
            .await?;

        let public_key =
            RS256PublicKey::from_pem(JWT_PUBLIC_KEY).expect("Could not parse public key");
        let claims = public_key.verify_token::<JWTEntitlements>(&json.signature, None)?;

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
    ) -> AuthData {
        AuthData {
            access_token: minecraft_token.access_token.clone(),
            username: minecraft_profile.name.clone(),
            uuid: minecraft_profile.id.clone(),
        }
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
    // #[tokio::test]
    async fn test_full_flow() {
        let auth = Auth {
            client_id: "2aa32806-92e3-4242-babc-392ac0f0fd30".to_string(),
            http_client: Client::new(),
        };

        let code = auth.create_device_code().await.unwrap();
        println!("Code: {} | Goto: {}", code.user_code, code.verification_uri);

        let stream = auth.pull_for_auth(&code);
        pin_mut!(stream);

        let auth_token = 'block: {
            while let Some(v) = stream.next().await {
                println!("{:?}", v);
                match v.unwrap() {
                    AuthTokenRes::ExpectedError(_) => continue,
                    AuthTokenRes::Token(token) => break 'block token,
                }
            }

            panic!("Could not get token");
        };

        println!("{:?}", auth_token);

        let xbl_token = auth
            .authenticate_xbox_live(&auth_token)
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
    }
}
