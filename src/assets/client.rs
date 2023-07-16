use std::{error::Error, fmt::Display, path::Path};

use error_stack::{IntoReport, ResultExt};
use serde::{Deserialize, Serialize};
use serde_json::to_string;
use tokio::fs;
use tracing::debug;

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
    /// Saves the manifest to disk.
    ///
    /// # Errors
    /// Returns a [`SaveError`] if the manifest could not be serialized or if an IO error occurred.
    #[tracing::instrument]
    pub async fn save_to_disk(&self, path: &Path) -> error_stack::Result<(), SaveError> {
        debug!("Saving manifest to disk");
        debug!("Serializing manifest to JSON");
        let value = to_string(self)
            .into_report()
            .change_context(SaveError::SerializeError)?;

        let directory = path.parent().ok_or(SaveError::IOError).into_report()?;

        if !directory.exists() {
            debug!("Creating directory {}", directory.display());
            fs::create_dir_all(directory)
                .await
                .into_report()
                .change_context(SaveError::IOError)?;
        }

        debug!("Writing manifest to {}", path.display());
        fs::write(path, value)
            .await
            .into_report()
            .change_context(SaveError::IOError)
    }

    #[must_use]
    pub const fn get_java_version(&self) -> u8 {
        match &self.java_version {
            Some(m) => m.major_version,
            None => 8,
        }
    }

    #[must_use]
    pub fn libraries(&self) -> &[Library] {
        self.libraries.as_ref()
    }
}

// Thank you quicktype, very cool :ferrisBased:

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct Manifest {
    asset_index: AssetIndex,
    assets: String,
    downloads: Downloads,
    id: String,
    libraries: Vec<Library>,
    main_class: String,
    /// Used before 1.13
    minecraft_arguments: Option<String>,
    arguments: Option<Arguments>,
    minimum_launcher_version: i64,
    java_version: Option<JavaVersion>,
    release_time: String,
    time: String,
    #[serde(rename = "type")]
    manifest_type: Type,
    logging: Option<Logging>,
}

impl Manifest {
    /// Gets the arguments for a manifest
    ///
    /// # Panics
    /// Panics if the manifest does not any types of arguments. This should never happen in a valid
    /// manifest
    #[must_use]
    pub fn get_arguments(&self) -> Args {
        self.arguments.as_ref().map_or_else(
            || {
                Args::MinecraftArguments(
                    self.minecraft_arguments
                        .as_ref()
                        .expect("Minecraft arguments to exist when arguments are not present"),
                )
            },
            Args::Arguments,
        )
    }
}

pub enum Args<'a> {
    MinecraftArguments(&'a str),
    Arguments(&'a Arguments),
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub enum Type {
    Release,
    Snapshot,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct AssetIndex {
    id: String,
    sha1: String,
    size: i64,
    total_size: i64,
    url: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Downloads {
    client: DownloadClass,
    server: DownloadClass,
    client_mappings: Option<Mappings>,
    server_mappings: Option<Mappings>,
    /// Only present in version 1 of the manifest it seems
    windows_server: Option<DownloadClass>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct DownloadClass {
    sha1: String,
    size: i64,
    url: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Artifact {
    path: String,
    sha1: String,
    size: i64,
    url: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Extract {
    exclude: Vec<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct JvmRule {
    action: Action,
    os: Option<Os>,
}

impl JvmRule {
    #[must_use]
    pub const fn action(&self) -> &Action {
        &self.action
    }

    #[must_use]
    pub const fn os(&self) -> Option<&Os> {
        self.os.as_ref()
    }

    pub fn java_rule_passes(&self) -> bool {
        match self.action() {
            Action::Allow => {
                let Some(os) = self.os() else {
                    return true;
                };

                let arch_rule = match os.arch().map(String::as_str) {
                    Some("x86") => cfg!(target_arch = "x86"),
                    Some(_) => todo!("Unknown arch"),
                    None => true,
                };

                let os_rule = match os.name().map(String::as_str) {
                    // windows users pls test
                    #[cfg(target_os = "windows")]
                    Some("windows") => {
                        if let Some(ver) = &rule.os.version {
                            if ver != "^10\\." {
                                panic!("unrecognised windows version: {:?}, please report to https://github.com/glowsquid-launcher/copper/issues with the version you are using", ver);
                            }

                            IsWindows10OrGreater().unwrap_or(false)
                        } else {
                            true
                        }
                    }
                    #[cfg(not(target_os = "windows"))]
                    Some("windows") => false,
                    Some("osx") => cfg!(target_os = "macos"),
                    Some("linux") => cfg!(target_os = "linux"),
                    Some(_) => todo!("Unknown os"),
                    None => true,
                };

                arch_rule && os_rule
            }
            Action::Disallow => todo!("No disallow rules for jvm args"),
        }
    }
}

#[derive(Debug, Serialize, Deserialize, Clone)]
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

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct JavaVersion {
    component: String,
    major_version: u8,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Logging {
    client: LoggingClient,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[allow(clippy::module_name_repetitions)]
pub struct LoggingClient {
    argument: String,
    file: File,
    #[serde(rename = "type")]
    client_type: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct File {
    id: String,
    sha1: String,
    size: i64,
    url: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(untagged)]
pub enum Game {
    GameClass(GameClass),
    String(String),
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct GameClass {
    rules: Vec<GameRule>,
    value: Value,
}

impl GameClass {
    #[must_use]
    pub fn rules(&self) -> &[GameRule] {
        self.rules.as_ref()
    }

    #[must_use]
    pub const fn value(&self) -> &Value {
        &self.value
    }
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct GameRule {
    action: Action,
    features: Features,
}

impl GameRule {
    #[must_use]
    pub const fn action(&self) -> &Action {
        &self.action
    }

    #[must_use]
    pub const fn features(&self) -> &Features {
        &self.features
    }
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Features {
    is_demo_user: Option<bool>,
    has_custom_resolution: Option<bool>,
    has_quick_plays_support: Option<bool>,
    is_quick_play_singleplayer: Option<bool>,
    is_quick_play_multiplayer: Option<bool>,
    is_quick_play_realms: Option<bool>,
}

impl Features {
    #[must_use]
    pub const fn demo_user(&self) -> Option<bool> {
        self.is_demo_user
    }

    #[must_use]
    pub const fn custom_resolution(&self) -> Option<bool> {
        self.has_custom_resolution
    }

    #[must_use]
    pub const fn quick_plays_support(&self) -> Option<bool> {
        self.has_quick_plays_support
    }

    #[must_use]
    pub const fn quick_play_singleplayer(&self) -> Option<bool> {
        self.is_quick_play_singleplayer
    }

    #[must_use]
    pub const fn quick_play_multiplayer(&self) -> Option<bool> {
        self.is_quick_play_multiplayer
    }

    #[must_use]
    pub const fn quick_play_realms(&self) -> Option<bool> {
        self.is_quick_play_realms
    }
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(untagged)]
pub enum Value {
    String(String),
    StringArray(Vec<String>),
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "kebab-case")]
pub struct Classifiers {
    natives_linux: Option<Artifact>,
    natives_macos: Option<Artifact>,
    natives_windows: Option<Artifact>,
    natives_osx: Option<Artifact>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Natives {
    linux: Option<String>,
    osx: Option<String>,
    windows: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Os {
    name: Option<String>,
    version: Option<String>,
    arch: Option<String>,
}

impl Os {
    #[must_use]
    pub const fn name(&self) -> Option<&String> {
        self.name.as_ref()
    }

    #[must_use]
    pub const fn version(&self) -> Option<&String> {
        self.version.as_ref()
    }

    #[must_use]
    pub const fn arch(&self) -> Option<&String> {
        self.arch.as_ref()
    }
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Mappings {
    sha1: String,
    size: i64,
    url: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(untagged)]
pub enum Jvm {
    String(String),
    Class(JvmClassRule),
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct JvmClassRule {
    rules: Vec<JvmRule>,
    value: Value,
}

impl JvmClassRule {
    #[must_use]
    pub const fn value(&self) -> &Value {
        &self.value
    }

    #[must_use]
    pub fn rules(&self) -> &[JvmRule] {
        self.rules.as_ref()
    }
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Arguments {
    game: Vec<Game>,
    jvm: Vec<Jvm>,
}

impl Arguments {
    #[must_use]
    pub fn game(&self) -> &[Game] {
        self.game.as_ref()
    }

    #[must_use]
    pub fn jvm(&self) -> &[Jvm] {
        self.jvm.as_ref()
    }
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Library {
    downloads: LibraryDownloads,
    name: String,
    rules: Option<Vec<JvmRule>>,
    natives: Option<Natives>,
    extract: Option<Extract>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct LibraryDownloads {
    /// Seemingly not present in version 1 and 2?
    artifact: Option<Artifact>,
    classifiers: Option<Classifiers>,
}
