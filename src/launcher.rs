use std::path::PathBuf;
use std::process::{ExitStatus, Stdio};

use crate::assets::structs::version::Version;
use crate::errors::LauncherError;
use crate::parser::JavaArguments;
use crate::{assets, parser::GameArguments};
use tokio::fs;
use tokio::io::BufReader;
use tokio::process::{ChildStderr, ChildStdout, Command};
use tokio::task::JoinHandle;
use tracing::{debug, trace};

#[derive(Default, Debug, Clone)]
pub struct AuthenticationDetails {
    pub username: String,
    pub uuid: String,
    pub access_token: String,
    pub xbox_uid: String,
    pub client_id: Option<String>,
    pub is_demo_user: bool,
}

#[derive(Default, Debug, Clone)]
pub struct CustomResolution {
    pub width: i32,
    pub height: i32,
}

#[derive(Default, Clone, Debug)]
pub struct RamSize {
    pub min: String,
    pub max: String,
}

pub struct GameOutput {
    pub stdout: BufReader<ChildStdout>,
    pub stderr: BufReader<ChildStderr>,
    pub exit_handle: JoinHandle<Option<ExitStatus>>,
}

#[derive(Default, Clone, Debug)]
pub struct Launcher {
    /// the authentication details (username, uuid, access token, xbox uid, etc)
    pub authentication_details: AuthenticationDetails,
    /// a custom resolution to use instead of the default
    pub custom_resolution: Option<CustomResolution>,
    /// the minecraft jar file path
    pub jar_path: PathBuf,
    /// the root .minecraft folder
    pub game_directory: PathBuf,
    /// the assets directory, this is the root of the assets folder
    pub assets_directory: PathBuf,
    /// the libraries directory, this is the root of the libraries folder
    pub libraries_directory: PathBuf,
    /// the path to <version>.json
    pub version_manifest_path: PathBuf,
    /// is this version a snapshot
    pub is_snapshot: bool,
    /// the version name
    pub version_name: String,
    /// the client brand
    pub client_branding: String,
    /// the min/max amount of ram to use
    pub ram_size: RamSize,
    /// the path to javaw.exe
    pub java_path: PathBuf,
    /// the launcher name (e.g glowsquid)
    pub launcher_name: String,
    /// The reqwest client
    pub http_client: reqwest::Client,
}

impl Launcher {
    #[tracing::instrument]
    pub async fn launch(
        &self,
        version_manifest: Option<Version>,
    ) -> Result<GameOutput, LauncherError> {
        trace!("Launching minecraft");

        let version_manifest = match version_manifest {
            Some(manifest) => manifest,
            None => serde_json::from_str(
                &fs::read_to_string(self.version_manifest_path.clone()).await?,
            )?,
        };

        let game_args = self.parse_game_arguments(&version_manifest)?;
        debug!("Game arguments: {:?}", &game_args);

        let java_args = self.parse_java_arguments(&version_manifest).await?;

        let main_class = version_manifest
            .main_class
            .as_ref()
            .ok_or(LauncherError::NoMainClass)?;

        debug!("Java arguments: {:?}", &java_args);
        debug!("main class: {}", main_class);

        let mut process = Command::new(self.java_path.clone())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .args(&java_args)
            .arg(main_class)
            .args(&game_args)
            .spawn()?;

        let stdout = process
            .stdout
            .take()
            .ok_or(LauncherError::CannotGetStdout)?;

        let stderr = process
            .stderr
            .take()
            .ok_or(LauncherError::CannotGetStderr)?;

        let out_reader = BufReader::new(stdout);
        let err_reader = BufReader::new(stderr);

        let exit = tokio::spawn(async move { process.wait().await.ok() });

        Ok(GameOutput {
            stderr: err_reader,
            stdout: out_reader,
            exit_handle: exit,
        })
    }

    #[tracing::instrument]
    async fn parse_java_arguments(
        &self,
        version_manifest: &Version,
    ) -> Result<Vec<String>, LauncherError> {
        let mut args: Vec<String> = vec![];

        for arg in version_manifest
            .arguments
            .as_ref()
            .ok_or(LauncherError::NoArgs)?
            .jvm
            .as_ref()
            .ok_or(LauncherError::NoArgs)?
        {
            let formatted_arg = match arg {
                assets::structs::version::JvmElement::JvmClass(argument) => {
                    JavaArguments::parse_class_argument(
                        self,
                        version_manifest,
                        argument,
                        self.http_client.clone(),
                    )
                    .await?
                }
                assets::structs::version::JvmElement::String(argument) => Some(
                    JavaArguments::parse_string_argument(
                        self,
                        version_manifest,
                        argument.to_string(),
                        self.http_client.clone(),
                    )
                    .await?,
                ),
            };

            if let Some(arg) = formatted_arg {
                args.push(arg);
            }
        }

        Ok(args)
    }

    #[tracing::instrument]
    fn parse_game_arguments(
        &self,
        version_manifest: &Version,
    ) -> Result<Vec<String>, LauncherError> {
        let mut args: Vec<String> = vec![];

        for arg in version_manifest
            .arguments
            .as_ref()
            .ok_or(LauncherError::NoArgs)?
            .game
            .as_ref()
            .ok_or(LauncherError::NoArgs)?
        {
            let formatted_arg = match arg {
                assets::structs::version::GameElement::GameClass(argument) => {
                    GameArguments::parse_class_argument(self, argument)?
                }

                assets::structs::version::GameElement::String(argument) => Some(
                    GameArguments::parse_string_argument(self, argument.to_string())?,
                ),
            };

            if let Some(arg) = formatted_arg {
                args.push(arg);
            }
        }

        Ok(args)
    }
}
