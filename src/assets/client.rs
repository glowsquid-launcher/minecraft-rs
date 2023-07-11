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
    /// The fourth version of the manifest format, used from 19w36a to 20w21a
    V4(V4Manifest),
    /// The fifth version of the manifest format, used from 20w21a to 20w45a
    V5(V5Manifest),
    /// The sixth version of the manifest format, used from 20w45a and ongoing
    V6(V6Manifest),
}

#[derive(Debug)]
pub enum SaveError {
    SerializeError,
    IOError,
}

impl Display for SaveError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SaveError::SerializeError => write!(f, "Failed to serialize manifest to JSON"),
            SaveError::IOError => write!(f, "Failed during IO task"),
        }
    }
}

impl Error for SaveError {}

impl Manifest {
    /// Tries to parse a manifest from a JSON value.
    ///
    /// Returns [`Option::None`] if the value is not a valid manifest.
    pub(super) fn from_value(value: serde_json::Value) -> Result<Self, serde_json::Error> {
        from_value::<V6Manifest>(value.clone())
            .map(Manifest::V6)
            .or_else(|_| from_value::<V5Manifest>(value.clone()).map(Manifest::V5))
            .or_else(|_| from_value::<V4Manifest>(value.clone()).map(Manifest::V4))
            .or_else(|_| from_value::<V3Manifest>(value.clone()).map(Manifest::V3))
            .or_else(|_| from_value::<V2Manifest>(value.clone()).map(Manifest::V2))
            .or_else(|_| from_value::<V1Manifest>(value).map(Manifest::V1))
    }

    pub fn manifest_version(&self) -> u8 {
        match self {
            Manifest::V1(_) => 1,
            Manifest::V2(_) => 2,
            Manifest::V3(_) => 3,
            Manifest::V4(_) => 4,
            Manifest::V5(_) => 5,
            Manifest::V6(_) => 6,
        }
    }

    fn serialize(&self) -> Result<String, serde_json::Error> {
        match self {
            Manifest::V1(m) => to_string(m),
            Manifest::V2(m) => to_string(m),
            Manifest::V3(m) => to_string(m),
            Manifest::V4(m) => to_string(m),
            Manifest::V5(m) => to_string(m),
            Manifest::V6(m) => to_string(m),
        }
    }

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
    downloads: V1Downloads,
    id: String,
    libraries: Vec<V1Library>,
    main_class: String,
    minecraft_arguments: String,
    minimum_launcher_version: i64,
    release_time: String,
    time: String,
    #[serde(rename = "type")]
    v1_type: String,
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
pub struct V1Downloads {
    client: ServerClass,
    server: ServerClass,
    windows_server: WindowsServer,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ServerClass {
    sha1: String,
    size: i64,
    url: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct WindowsServer {
    sha1: String,
    size: i64,
    url: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct V1Library {
    downloads: PurpleDownloads,
    name: String,
    rules: Option<Vec<Rule>>,
    extract: Option<Extract>,
    natives: Option<Natives>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct PurpleDownloads {
    artifact: Option<Artifact>,
    classifiers: Option<PurpleClassifiers>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Artifact {
    path: String,
    sha1: String,
    size: i64,
    url: String,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct PurpleClassifiers {
    natives_linux: Option<Artifact>,
    natives_osx: Artifact,
    natives_windows: Option<Artifact>,
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
    downloads: V1Downloads,
    id: String,
    java_version: JavaVersion,
    libraries: Vec<V2Library>,
    logging: Logging,
    main_class: String,
    minecraft_arguments: String,
    minimum_launcher_version: i64,
    release_time: String,
    time: String,
    #[serde(rename = "type")]
    v2_type: String,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct JavaVersion {
    component: String,
    major_version: i64,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct V2Library {
    downloads: FluffyDownloads,
    name: String,
    rules: Option<Vec<Rule>>,
    extract: Option<Extract>,
    natives: Option<Natives>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct FluffyDownloads {
    artifact: Option<Artifact>,
    classifiers: Option<PurpleClassifiers>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Logging {
    client: LoggingClient,
}

#[derive(Debug, Serialize, Deserialize)]
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
    arguments: V3Arguments,
    asset_index: AssetIndex,
    assets: String,
    compliance_level: i64,
    downloads: V3Downloads,
    id: String,
    java_version: JavaVersion,
    libraries: Vec<V3Library>,
    logging: Logging,
    main_class: String,
    minimum_launcher_version: i64,
    release_time: String,
    time: String,
    #[serde(rename = "type")]
    v3_type: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct V3Arguments {
    game: Vec<PurpleGame>,
    jvm: Vec<Jvm>,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(untagged)]
pub enum PurpleGame {
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
pub struct V3Downloads {
    client: ServerClass,
    server: ServerClass,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct V3Library {
    downloads: TentacledDownloads,
    name: String,
    natives: Option<Natives>,
    extract: Option<Extract>,
    rules: Option<Vec<Rule>>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct TentacledDownloads {
    artifact: Artifact,
    classifiers: Option<FluffyClassifiers>,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct FluffyClassifiers {
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
    arguments: V4Arguments,
    asset_index: AssetIndex,
    assets: String,
    compliance_level: i64,
    downloads: V4Downloads,
    id: String,
    java_version: JavaVersion,
    libraries: Vec<V4Library>,
    logging: Logging,
    main_class: String,
    minimum_launcher_version: i64,
    release_time: String,
    time: String,
    #[serde(rename = "type")]
    v4_type: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct V4Arguments {
    game: Vec<FluffyGame>,
    jvm: Vec<IndecentJvm>,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(untagged)]
pub enum FluffyGame {
    GameClass(GameClass),
    String(String),
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(untagged)]
pub enum IndecentJvm {
    FluffyJvm(FluffyJvm),
    String(String),
}

#[derive(Debug, Serialize, Deserialize)]
pub struct FluffyJvm {
    rules: Vec<Rule>,
    value: Value,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Os {
    name: Option<String>,
    version: Option<String>,
    arch: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct V4Downloads {
    client: ServerClass,
    client_mappings: Mappings,
    server: ServerClass,
    server_mappings: Mappings,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Mappings {
    sha1: String,
    size: i64,
    url: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct V4Library {
    downloads: StickyDownloads,
    name: String,
    rules: Option<Vec<IndigoRule>>,
    natives: Option<Natives>,
    extract: Option<Extract>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct StickyDownloads {
    artifact: Artifact,
    classifiers: Option<FluffyClassifiers>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct IndigoRule {
    action: Action,
    os: Option<Os>,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct V5Manifest {
    arguments: V5Arguments,
    asset_index: AssetIndex,
    assets: String,
    compliance_level: i64,
    downloads: V4Downloads,
    id: String,
    java_version: JavaVersion,
    libraries: Vec<V5Library>,
    logging: Logging,
    main_class: String,
    minimum_launcher_version: i64,
    release_time: String,
    time: String,
    #[serde(rename = "type")]
    v5_type: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct V5Arguments {
    game: Vec<TentacledGame>,
    jvm: Vec<Jvm>,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(untagged)]
pub enum TentacledGame {
    GameClass(GameClass),
    String(String),
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
pub struct V5Library {
    downloads: IndigoDownloads,
    name: String,
    rules: Option<Vec<IndigoRule>>,
    natives: Option<Natives>,
    extract: Option<Extract>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct IndigoDownloads {
    artifact: Artifact,
    classifiers: Option<FluffyClassifiers>,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct V6Manifest {
    arguments: V6Arguments,
    asset_index: AssetIndex,
    assets: String,
    compliance_level: i64,
    downloads: V4Downloads,
    id: String,
    java_version: JavaVersion,
    libraries: Vec<V6Library>,
    logging: Logging,
    main_class: String,
    minimum_launcher_version: i64,
    release_time: String,
    time: String,
    #[serde(rename = "type")]
    v6_type: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct V6Arguments {
    game: Vec<StickyGame>,
    jvm: Vec<AmbitiousJvm>,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(untagged)]
pub enum StickyGame {
    GameClass(GameClass),
    String(String),
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(untagged)]
pub enum AmbitiousJvm {
    StickyJvm(StickyJvm),
    String(String),
}

#[derive(Debug, Serialize, Deserialize)]
pub struct StickyJvm {
    rules: Vec<Rule>,
    value: Value,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct V6Library {
    downloads: IndecentDownloads,
    name: String,
    rules: Option<Vec<IndigoRule>>,
    natives: Option<Natives>,
    extract: Option<Extract>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct IndecentDownloads {
    artifact: Artifact,
    classifiers: Option<FluffyClassifiers>,
}
