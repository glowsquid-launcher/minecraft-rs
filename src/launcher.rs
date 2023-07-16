use derive_builder::Builder;
use std::{
    io::BufReader,
    path::PathBuf,
    process::{ChildStderr, ChildStdout, ExitStatus},
};

use itertools::Itertools;
use tokio::task::JoinHandle;

#[cfg(target_os = "windows")]
use winsafe::IsWindows10OrGreater;

use crate::{
    assets::client,
    auth::structs::{MinecraftProfile, MinecraftToken},
};

#[derive(Debug, Clone)]
pub struct AuthenticationDetails {
    pub auth_details: MinecraftToken,
    pub minecraft_profile: MinecraftProfile,
    pub client_id: Option<String>,
    pub is_demo_user: bool,
}

#[derive(Debug, Clone)]
pub struct CustomResolution {
    pub width: i32,
    pub height: i32,
}

#[derive(Debug, Clone)]
pub struct RamSize {
    pub min: String,
    pub max: String,
}

pub struct GameOutput {
    pub stdout: BufReader<ChildStdout>,
    pub stderr: BufReader<ChildStderr>,
    pub exit_handle: JoinHandle<Option<ExitStatus>>,
}

#[derive(Debug, Eq, PartialEq, Clone)]
pub enum Quickplay {
    /// Singleplayer quickplay. Inner value is a world name
    Singleplayer(String),
    /// Multiplayer quickplay. Inner value is a server address
    Multiplayer(String),
    /// Realms quickplay. Inner value is a realm ID
    Realms(String),
}

impl Quickplay {
    #[must_use]
    pub const fn is_singleplayer(&self) -> bool {
        matches!(self, Self::Singleplayer(_))
    }

    #[must_use]
    pub const fn is_multiplayer(&self) -> bool {
        matches!(self, Self::Multiplayer(_))
    }

    #[must_use]
    pub const fn is_realms(&self) -> bool {
        matches!(self, Self::Realms(_))
    }
}

#[derive(Debug, Builder)]
#[builder(setter(into))]
pub struct Launcher {
    /// The authentication details (username, uuid, access token, xbox uid, etc)
    authentication_details: AuthenticationDetails,
    /// A custom resolution to use instead of the default
    custom_resolution: Option<CustomResolution>,
    /// The minecraft jar file path
    jar_path: PathBuf,
    /// The root .minecraft folder
    game_directory: PathBuf,
    /// The assets directory, this is the root of the assets folder
    assets_directory: PathBuf,
    /// The libraries directory, this is the root of the libraries folder
    libraries_directory: PathBuf,
    /// The path to <version>.json
    version_manifest_path: PathBuf,
    /// is this version a snapshot
    is_snapshot: bool,
    /// The version name
    version_name: String,
    /// The min/max amount of ram to use
    ram_size: RamSize,
    /// The path to javaw.exe
    java_path: PathBuf,
    /// The launcher name (e.g glowsquid)
    launcher_name: String,
    /// The launcher version
    launcher_version: String,
    /// If you want to launch with quickplay
    quickplay: Option<Quickplay>,
    /// The reqwest client
    http_client: reqwest::Client,
    /// The manifest the launcher will use
    manifest: client::Manifest,
}

impl Launcher {
    #[must_use]
    pub const fn authentication_details(&self) -> &AuthenticationDetails {
        &self.authentication_details
    }

    #[must_use]
    pub const fn custom_resolution(&self) -> Option<&CustomResolution> {
        self.custom_resolution.as_ref()
    }

    #[must_use]
    pub const fn jar_path(&self) -> &PathBuf {
        &self.jar_path
    }

    #[must_use]
    pub const fn game_directory(&self) -> &PathBuf {
        &self.game_directory
    }

    #[must_use]
    pub const fn assets_directory(&self) -> &PathBuf {
        &self.assets_directory
    }

    #[must_use]
    pub const fn libraries_directory(&self) -> &PathBuf {
        &self.libraries_directory
    }

    #[must_use]
    pub const fn version_manifest_path(&self) -> &PathBuf {
        &self.version_manifest_path
    }

    #[must_use]
    pub const fn is_snapshot(&self) -> bool {
        self.is_snapshot
    }

    #[must_use]
    pub fn version_name(&self) -> &str {
        self.version_name.as_ref()
    }

    #[must_use]
    pub const fn ram_size(&self) -> &RamSize {
        &self.ram_size
    }

    #[must_use]
    pub const fn java_path(&self) -> &PathBuf {
        &self.java_path
    }

    #[must_use]
    pub fn launcher_name(&self) -> &str {
        self.launcher_name.as_ref()
    }

    #[must_use]
    pub fn launcher_version(&self) -> &str {
        self.launcher_version.as_ref()
    }

    #[must_use]
    pub const fn quickplay(&self) -> Option<&Quickplay> {
        self.quickplay.as_ref()
    }
}
