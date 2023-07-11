use std::{error::Error, fmt::Display, path::Path};

use error_stack::{IntoReport, ResultExt};
use serde::{Deserialize, Serialize};
use serde_json::{from_value, to_string};
use tokio::fs;

pub enum Manifest {
    /// The first version of the manifest format, used until 13w39a
    V1(V1Manifest),
    /// The second version of the manifest format, used from 13w39a to 17w43a
    V2(V2Manifest),
    /// The third version of the manifest format, used from 17w43a to 19w36a
    V3(V3Manifest),
    /// The fourth version of the manifest format, used from 19w36a+
    V4(V4Manifest),
}

#[derive(Debug)]
pub enum SaveError {
    SerializeError,
    IOError,
}

impl Display for SaveError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::SerializeError => write!(f, "Failed to serialize manifest to JSON"),
            Self::IOError => write!(f, "Failed during IO task"),
        }
    }
}

impl Error for SaveError {}

impl Manifest {
    /// Tries to parse a manifest from a JSON value.
    ///
    /// Returns [`Option::None`] if the value is not a valid manifest.
    pub(super) fn from_value(value: serde_json::Value) -> Result<Self, serde_json::Error> {
        from_value::<V4Manifest>(value.clone())
            .map(Manifest::V4)
            .or_else(|_| from_value::<V3Manifest>(value.clone()).map(Manifest::V3))
            .or_else(|_| from_value::<V2Manifest>(value.clone()).map(Manifest::V2))
            .or_else(|_| from_value::<V1Manifest>(value).map(Manifest::V1))
    }

    #[must_use]
    pub const fn manifest_version(&self) -> u8 {
        match self {
            Self::V1(_) => 1,
            Self::V2(_) => 2,
            Self::V3(_) => 3,
            Self::V4(_) => 4,
        }
    }

    /// Serializes the manifest to a JSON string.
    ///
    /// # Errors
    /// Returns a [`serde_json::Error`] if the manifest could not be serialized.
    fn serialize(&self) -> Result<String, serde_json::Error> {
        match self {
            Self::V1(m) => to_string(m),
            Self::V2(m) => to_string(m),
            Self::V3(m) => to_string(m),
            Self::V4(m) => to_string(m),
        }
    }

    /// Saves the manifest to disk.
    ///
    /// # Errors
    /// Returns a [`SaveError`] if the manifest could not be serialized or if an IO error occurred.
    pub async fn save_to_disk(&self, path: &Path) -> error_stack::Result<(), SaveError> {
        let value = self
            .serialize()
            .into_report()
            .change_context(SaveError::SerializeError)?;

        let directory = path.parent().ok_or(SaveError::IOError).into_report()?;

        if !directory.exists() {
            fs::create_dir_all(directory)
                .await
                .into_report()
                .change_context(SaveError::IOError)?;
        }

        fs::write(path, value)
            .await
            .into_report()
            .change_context(SaveError::IOError)
    }
}

// Thank you quicktype, very cool :ferrisBased:

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct V1Manifest {
    asset_index: AssetIndex,
    assets: String,
    downloads: Downloads,
    id: String,
    libraries: Vec<Library>,
    main_class: String,
    minecraft_arguments: String,
    minimum_launcher_version: i64,
    release_time: String,
    time: String,
    #[serde(rename = "type")]
    manifest_type: Type,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum Type {
    Release,
    Snapshot,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AssetIndex {
    id: String,
    sha1: String,
    size: i64,
    total_size: i64,
    url: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Downloads {
    client: DownloadClass,
    server: DownloadClass,
    client_mappings: Option<Mappings>,
    server_mappings: Option<Mappings>,
    /// Only present in version 1 of the manifest it seems
    windows_server: Option<DownloadClass>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct DownloadClass {
    sha1: String,
    size: i64,
    url: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Artifact {
    path: String,
    sha1: String,
    size: i64,
    url: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Extract {
    exclude: Vec<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Rule {
    action: Action,
    os: Option<Os>,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Action {
    Allow,
    Disallow,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Name {
    Osx,
    Linux,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct V2Manifest {
    asset_index: AssetIndex,
    assets: String,
    compliance_level: i64,
    downloads: Downloads,
    id: String,
    java_version: JavaVersion,
    libraries: Vec<Library>,
    logging: Logging,
    main_class: String,
    minecraft_arguments: String,
    minimum_launcher_version: i64,
    release_time: String,
    time: String,
    #[serde(rename = "type")]
    manifest_type: String,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct JavaVersion {
    component: String,
    major_version: i64,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Logging {
    client: LoggingClient,
}

#[derive(Debug, Serialize, Deserialize)]
#[allow(clippy::module_name_repetitions)]
pub struct LoggingClient {
    argument: String,
    file: File,
    #[serde(rename = "type")]
    client_type: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct File {
    id: String,
    sha1: String,
    size: i64,
    url: String,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct V3Manifest {
    arguments: Arguments,
    asset_index: AssetIndex,
    assets: String,
    compliance_level: i64,
    downloads: Downloads,
    id: String,
    java_version: JavaVersion,
    libraries: Vec<Library>,
    logging: Logging,
    main_class: String,
    minimum_launcher_version: i64,
    release_time: String,
    time: String,
    #[serde(rename = "type")]
    v3_type: String,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(untagged)]
pub enum Game {
    GameClass(GameClass),
    String(String),
}

#[derive(Debug, Serialize, Deserialize)]
pub struct GameClass {
    rules: Vec<GameRule>,
    value: Value,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct GameRule {
    action: Action,
    features: Features,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Features {
    is_demo_user: Option<bool>,
    has_custom_resolution: Option<bool>,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(untagged)]
pub enum Value {
    String(String),
    StringArray(Vec<String>),
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct Classifiers {
    natives_linux: Option<Artifact>,
    natives_macos: Option<Artifact>,
    natives_windows: Option<Artifact>,
    natives_osx: Option<Artifact>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Natives {
    linux: Option<String>,
    osx: Option<String>,
    windows: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct V4Manifest {
    arguments: Arguments,
    asset_index: AssetIndex,
    assets: String,
    compliance_level: i64,
    downloads: Downloads,
    id: String,
    java_version: JavaVersion,
    libraries: Vec<Library>,
    logging: Logging,
    main_class: String,
    minimum_launcher_version: i64,
    release_time: String,
    time: String,
    #[serde(rename = "type")]
    manifest_type: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Os {
    name: Option<String>,
    version: Option<String>,
    arch: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Mappings {
    sha1: String,
    size: i64,
    url: String,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(untagged)]
pub enum Jvm {
    String(String),
    Rule(JvmRule),
}

#[derive(Debug, Serialize, Deserialize)]
pub struct JvmRule {
    rules: Vec<Rule>,
    value: Value,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Arguments {
    game: Vec<Game>,
    jvm: Vec<Jvm>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Library {
    downloads: LibraryDownloads,
    name: String,
    rules: Option<Vec<Rule>>,
    natives: Option<Natives>,
    extract: Option<Extract>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct LibraryDownloads {
    /// Seemingly not present in version 1 and 2?
    artifact: Option<Artifact>,
    classifiers: Option<Classifiers>,
}
