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
    assets::client::{self, Args},
    auth::structs::{MinecraftProfile, MinecraftToken},
};

#[derive(Debug)]
pub struct AuthenticationDetails {
    pub auth_details: MinecraftToken,
    pub minecraft_profile: MinecraftProfile,
    pub client_id: Option<String>,
    pub is_demo_user: bool,
}

#[derive(Debug)]
pub struct CustomResolution {
    pub width: i32,
    pub height: i32,
}

#[derive(Debug)]
pub struct RamSize {
    pub min: String,
    pub max: String,
}

pub struct GameOutput {
    pub stdout: BufReader<ChildStdout>,
    pub stderr: BufReader<ChildStderr>,
    pub exit_handle: JoinHandle<Option<ExitStatus>>,
}

#[derive(Debug, Eq, PartialEq)]
pub enum Quickplay {
    /// Singleplayer quickplay. Inner value is a world name
    Singleplayer(String),
    /// Multiplayer quickplay. Inner value is a server address
    Multiplayer(String),
    /// Realms quickplay. Inner value is a realm ID
    Realms(String),
}

impl Quickplay {
    pub fn is_singleplayer(&self) -> bool {
        matches!(self, Self::Singleplayer(_))
    }

    pub fn is_multiplayer(&self) -> bool {
        matches!(self, Self::Multiplayer(_))
    }

    pub fn is_realms(&self) -> bool {
        matches!(self, Self::Realms(_))
    }
}

#[derive(Debug)]
pub struct Launcher {
    /// the authentication details (username, uuid, access token, xbox uid, etc)
    authentication_details: AuthenticationDetails,
    /// a custom resolution to use instead of the default
    custom_resolution: Option<CustomResolution>,
    /// the minecraft jar file path
    jar_path: PathBuf,
    /// the root .minecraft folder
    game_directory: PathBuf,
    /// the assets directory, this is the root of the assets folder
    assets_directory: PathBuf,
    /// the libraries directory, this is the root of the libraries folder
    libraries_directory: PathBuf,
    /// the path to <version>.json
    version_manifest_path: PathBuf,
    /// is this version a snapshot
    is_snapshot: bool,
    /// the version name
    version_name: String,
    /// the client brand
    client_branding: String,
    /// the min/max amount of ram to use
    ram_size: RamSize,
    /// the path to javaw.exe
    java_path: PathBuf,
    /// the launcher name (e.g glowsquid)
    launcher_name: String,
    /// If you want to launch with quickplay
    quickplay: Option<Quickplay>,
    /// The reqwest client
    http_client: reqwest::Client,
}

impl Launcher {
    pub fn new(
        authentication_details: AuthenticationDetails,
        custom_resolution: Option<CustomResolution>,
        jar_path: PathBuf,
        game_directory: PathBuf,
        assets_directory: PathBuf,
        libraries_directory: PathBuf,
        version_manifest_path: PathBuf,
        is_snapshot: bool,
        version_name: String,
        client_branding: String,
        ram_size: RamSize,
        java_path: PathBuf,
        launcher_name: String,
        quickplay: Option<Quickplay>,
        http_client: Option<reqwest::Client>,
    ) -> Self {
        Self {
            authentication_details,
            custom_resolution,
            jar_path,
            game_directory,
            assets_directory,
            libraries_directory,
            version_manifest_path,
            is_snapshot,
            version_name,
            client_branding,
            ram_size,
            java_path,
            launcher_name,
            quickplay,
            http_client: http_client.unwrap_or(reqwest::Client::new()),
        }
    }

    pub fn parse_minecraft_args(&self, manifest: &client::Manifest) -> String {
        let args = manifest.get_arguments();

        match args {
            client::Args::MinecraftArguments(minecraft_args) => {
                self.parse_minecraft_arg_str(minecraft_args)
            }
            client::Args::Arguments(args) => {
                let game_args = args.game();

                game_args
                    .iter()
                    .map(|arg| match arg {
                        client::Game::GameClass(class) => {
                            let passes = class
                                .rules()
                                .iter()
                                .all(|rule| self.minecraft_rule_passes(rule));

                            if !passes {
                                return "".to_string();
                            };

                            match class.value() {
                                client::Value::String(s) => self.parse_minecraft_arg_str(s),
                                client::Value::StringArray(a) => {
                                    a.iter().map(|v| self.parse_minecraft_arg_str(v)).join(" ")
                                }
                            }
                        }
                        client::Game::String(arg) => arg.to_string(),
                    })
                    .join(" ")
            }
        }
    }

    pub fn parse_jvm_args(&self, manifest: &client::Manifest) -> String {
        let Args::Arguments(args) = manifest.get_arguments() else {
            return "".to_string();
        };

        let jvm = args.jvm();
        jvm.iter()
            .map(|arg| match arg {
                client::Jvm::String(arg) => self.parse_java_arg_str(arg),
                client::Jvm::Class(class) => {
                    let passes = class.rules().iter().all(|rule| self.java_rule_passes(rule));

                    if !passes {
                        return "".to_string();
                    };

                    match class.value() {
                        client::Value::String(s) => self.parse_java_arg_str(&s),
                        client::Value::StringArray(a) => {
                            a.iter().map(|v| self.parse_java_arg_str(v)).join(" ")
                        }
                    }
                }
            })
            .join(" ")
    }

    fn parse_java_arg_str(&self, arg: &str) -> String {
        arg.replace(
            "${natives_directory}",
            self.libraries_directory.to_str().unwrap_or_default(),
        )
    }

    fn java_rule_passes(&self, rule: &client::JvmRule) -> bool {
        match rule.action() {
            client::Action::Allow => {
                let Some(os) = rule.os() else {
                    return true;
                };

                match os.arch().map(String::as_str) {
                    Some("x86") => {
                        if !cfg!(target_arch = "x86") {
                            return false;
                        }
                    }
                    Some(_) => todo!("Unknown arch"),
                    None => (),
                }

                match os.name().map(String::as_str) {
                    // windows users pls test
                    #[cfg(target_os = "windows")]
                    Some("windows") => {
                        if let Some(ver) = &rule.os.version {
                            if ver != "^10\\." {
                                panic!("unrecognised windows version: {:?}, please report to https://github.com/glowsquid-launcher/copper/issues with the version you are using", ver);
                            }

                            return IsWindows10OrGreater().unwrap_or(false);
                        } else {
                            return true;
                        }
                    }
                    #[cfg(not(target_os = "windows"))]
                    Some("windows") => return false,
                    Some("osx") => {
                        if !cfg!(target_os = "macos") {
                            return false;
                        }
                    }
                    Some("linux") => {
                        if !cfg!(target_os = "linux") {
                            return false;
                        }
                    }
                    Some(_) => todo!("Unknown os"),
                    None => (),
                }

                true
            }
            client::Action::Disallow => todo!("No disallow rules for jvm args"),
        }
    }

    fn parse_minecraft_arg_str(&self, minecraft_arg: &str) -> String {
        minecraft_arg
            .replace(
                "${auth_player_name}",
                &self.authentication_details.auth_details.username,
            )
            .replace(
                "${version_name}",
                &self.version_name.replace(" ", "_").replace(":", "_"),
            )
            .replace(
                "${game_directory}",
                self.game_directory.to_str().unwrap_or_default(),
            )
            .replace(
                "${assets_root}",
                self.assets_directory.to_str().unwrap_or_default(),
            )
            .replace(
                "${assets_index_name}",
                &self.version_name.replace(" ", "_").replace(":", "_"),
            )
            .replace(
                "${auth_uuid}",
                &self.authentication_details.minecraft_profile.id(),
            )
            .replace(
                "${auth_access_token}",
                &self.authentication_details.auth_details.access_token,
            )
            .replace("${user_type}", "msa") // copper only supports MSA
            .replace(
                "${version_type}",
                if self.is_snapshot {
                    "snapshot"
                } else {
                    "release"
                },
            )
            .replace(
                "${resolution_width}",
                &self
                    .custom_resolution
                    .as_ref()
                    .map(|r| r.width.to_string())
                    .unwrap_or_default(),
            )
            .replace(
                "${resolution_height}",
                &self
                    .custom_resolution
                    .as_ref()
                    .map(|r| r.height.to_string())
                    .unwrap_or_default(),
            )
    }

    fn minecraft_rule_passes(&self, rule: &client::GameRule) -> bool {
        match rule.action() {
            client::Action::Allow => {
                let features = rule.features();

                if let Some(demo_user) = features.demo_user() {
                    if demo_user != self.authentication_details.is_demo_user {
                        return false;
                    }
                }

                if let Some(quick_plays_support) = features.quick_plays_support() {
                    if quick_plays_support != self.quickplay.is_some() {
                        return false;
                    }
                }

                if let Some(quick_play_singleplayer) = features.quick_play_singleplayer() {
                    if quick_play_singleplayer
                        != self
                            .quickplay
                            .as_ref()
                            .map(|q| q.is_singleplayer())
                            .unwrap_or_default()
                    {
                        return false;
                    }
                }

                if let Some(quick_play_multiplayer) = features.quick_play_multiplayer() {
                    if quick_play_multiplayer
                        != self
                            .quickplay
                            .as_ref()
                            .map(|q| q.is_multiplayer())
                            .unwrap_or_default()
                    {
                        return false;
                    }
                }

                if let Some(quick_play_realms) = features.quick_play_realms() {
                    if quick_play_realms
                        != self
                            .quickplay
                            .as_ref()
                            .map(|q| q.is_realms())
                            .unwrap_or_default()
                    {
                        return false;
                    }
                }

                true
            }
            client::Action::Disallow => {
                todo!("disallow rules are not supported yet, as none exist")
            }
        }
    }
}
