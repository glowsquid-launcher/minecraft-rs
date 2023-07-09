use serde::{Deserialize, Serialize};

use crate::errors::{InternalAuthTokenError, InternalReqwestError};

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct DeviceCode {
    pub device_code: String,
    pub user_code: String,
    pub verification_uri: String,
    pub expires_in: u32,
    pub interval: u32,
    pub message: String,
}

#[derive(Debug, Serialize)]
pub enum AuthTokenError {
    ExpectedError(InternalAuthTokenError),
    ReqwestError(InternalReqwestError),
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct AuthToken {
    pub token_type: String,
    pub scope: String,
    pub expires_in: u32,
    pub access_token: String,
    pub refresh_token: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "PascalCase")]
pub struct XboxToken {
    pub issue_instant: String,
    pub not_after: String,
    /// important. This is your XBL token
    pub token: String,
    pub display_claims: DisplayClaims,
}

pub type XstsToken = XboxToken;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct DisplayClaims {
    pub xui: Vec<Xui>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Xui {
    /// Important
    pub uhs: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
/// This is the token you need to use for launching minecraft. Note that before you even attempting to launch the game, you should make sure the account even owns Minecraft
pub struct MinecraftToken {
    /// This is NOT your Minecraft username, nor your Minecraft UUID
    pub username: String,
    /// TODO: figure out type of this
    pub roles: Vec<String>,
    /// This is your Minecraft access token, keep in mind that this will expire soon
    pub access_token: String,
    /// Usually `Bearer`
    pub token_type: String,
    /// This is (in seconds) how long your access token will last
    pub expires_in: u32,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct MinecraftProductAttachment {
    pub items: Vec<MinecraftProductAttachmentItem>,
    pub signature: String,
    #[serde(rename = "keyId")]
    pub key_id: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct MinecraftProductAttachmentItem {
    pub name: String,
    pub signature: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct JWTEntitlements {
    pub entitlements: Vec<JWTEntitlement>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct JWTEntitlement {
    pub name: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct MinecraftProfile {
    pub id: String,
    pub name: String,
    pub skins: Vec<Skin>,
    pub capes: Vec<Cape>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Skin {
    pub id: String,
    pub state: String,
    pub url: String,
    pub variant: Option<String>,
    pub alias: Option<String>,
}

pub type Cape = Skin;

#[derive(Default, Debug, Serialize, Deserialize, Clone)]
pub struct AuthData {
    pub access_token: String,
    pub refresh_token: String,
    pub uuid: String,
    pub username: String,
    pub xuid: String,
    pub expires_at: String,
}
