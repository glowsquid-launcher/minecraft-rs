use super::version::Version as VersionManifest;
use serde::{Deserialize, Serialize};
use tracing::{debug, trace};

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct LauncherMeta {
    pub latest: Latest,
    pub versions: Vec<Version>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Latest {
    pub release: String,
    pub snapshot: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Version {
    pub id: String,
    #[serde(rename = "type")]
    pub version_type: Type,
    pub url: String,
    pub time: String,
    #[serde(rename = "releaseTime")]
    pub release_time: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub enum Type {
    #[serde(rename = "old_alpha")]
    OldAlpha,
    #[serde(rename = "old_beta")]
    OldBeta,
    #[serde(rename = "release")]
    Release,
    #[serde(rename = "snapshot")]
    Snapshot,
}

impl Version {
    #[tracing::instrument]
    pub async fn version(&self) -> Result<VersionManifest, reqwest::Error> {
        trace!("Downloading version manifest for {}", self.id);
        // download the version manifest and return a parsed version manifest
        reqwest::get(&self.url)
            .await?
            .json::<VersionManifest>()
            .await
    }
}

impl Latest {
    #[tracing::instrument]
    pub fn version_for_release<'a>(&self, launcher_meta: &'a LauncherMeta) -> &'a Version {
        // get the latest release and find its version
        launcher_meta
            .versions
            .iter()
            .find(|version| version.id == self.release)
            .unwrap()
    }

    #[tracing::instrument]
    pub fn version_for_snapshot<'a>(&self, launcher_meta: &'a LauncherMeta) -> &'a Version {
        // get the latest snapshot and find its version
        launcher_meta
            .versions
            .iter()
            .find(|version| version.id == self.snapshot)
            .unwrap()
    }
}

impl LauncherMeta {
    #[tracing::instrument]
    pub async fn download_meta() -> Result<Self, reqwest::Error> {
        let server_url = "https://launchermeta.mojang.com/mc/game/version_manifest_v2.json";
        debug!("Downloading launcher meta from {}", server_url);

        reqwest::get(server_url).await?.json::<LauncherMeta>().await
    }
}
