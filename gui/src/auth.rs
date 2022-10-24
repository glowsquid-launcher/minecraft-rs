use eyre::Result;
use reqwest::Url;
use serde::{Deserialize, Serialize};

// some code taken from glowsquid

#[derive(Debug, Serialize, Deserialize)]
pub struct PlayerDbMinecraftProfile {
    #[serde(rename = "data")]
    pub(crate) data: Data,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Data {
    #[serde(rename = "player")]
    pub(crate) player: Player,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Player {
    #[serde(rename = "username")]
    pub(crate) username: String,

    #[serde(rename = "id")]
    pub(crate) id: String,

    #[serde(rename = "avatar")]
    pub(crate) avatar: String,
}

pub async fn get_profile(uuid: &String) -> Result<PlayerDbMinecraftProfile> {
    let url = format!("https://playerdb.co/api/player/minecraft/{}", uuid);

    let resp = reqwest::get(&url).await?;

    Ok(resp.json().await?)
}

#[derive(Debug, Serialize, Deserialize, PartialEq, Eq, Clone)]
#[serde(rename_all = "camelCase")]
pub struct MinecraftProfile {
    pub uuid: String,
    pub access_token: String,
    pub refresh_token: String,
    pub expires_in: u32,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Account {
    pub id: String,
    pub username: String,
    pub access_token: String,
    pub refresh_token: String,
    pub expires_at: String,
    pub last_refreshed: String,
}
