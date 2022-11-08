use std::{fs::create_dir_all, io::Write, path::PathBuf};

use futures::{stream::FuturesUnordered, StreamExt};
use serde::{Deserialize, Serialize};
use tokio::{
    sync::watch::{self, Sender},
    task,
};
use tracing::{debug, trace};

use crate::{
    errors::{DownloadError, VersionError},
    util::{
        create_client, create_download_task, create_library_download, DownloadProgress,
        DownloadWatcher, ListOfResultHandles,
    },
};

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Version {
    pub inherits_from: Option<String>,
    pub arguments: Option<Arguments>,
    #[serde(rename = "assetIndex")]
    pub asset_index: Option<AssetIndex>,
    pub assets: Option<String>,
    #[serde(rename = "complianceLevel")]
    pub compliance_level: Option<i64>,
    pub downloads: Option<VersionInfoDownloads>,
    pub id: Option<String>,
    #[serde(rename = "javaVersion")]
    pub java_version: Option<JavaVersion>,
    pub libraries: Option<Vec<Library>>,
    pub logging: Option<Logging>,
    #[serde(rename = "mainClass")]
    pub main_class: Option<String>,
    #[serde(rename = "minimumLauncherVersion")]
    pub minimum_launcher_version: Option<i64>,
    #[serde(rename = "releaseTime")]
    pub release_time: Option<String>,
    pub time: Option<String>,
    #[serde(rename = "type")]
    pub version_info_type: Option<String>,
}

impl Version {
    #[tracing::instrument]
    pub fn merge(self, lower: Self) -> Self {
        let mut merged = Self {
            inherits_from: None,
            arguments: None,
            asset_index: None,
            assets: None,
            compliance_level: None,
            downloads: None,
            id: None,
            java_version: None,
            libraries: None,
            logging: None,
            main_class: None,
            minimum_launcher_version: None,
            release_time: None,
            time: None,
            version_info_type: None,
        };

        // arguments (vector merging)
        if let Some(arguments) = lower.arguments {
            let current_arguments = self.arguments.unwrap_or(Arguments {
                game: Some(vec![]),
                jvm: Some(vec![]),
            });

            let jvm = current_arguments
                .jvm
                .unwrap_or_default()
                .into_iter()
                .chain(arguments.jvm.unwrap_or_default().into_iter())
                .collect();

            let game = current_arguments
                .game
                .unwrap_or_default()
                .into_iter()
                .chain(arguments.game.unwrap_or_default())
                .collect();

            merged.arguments = Some(Arguments {
                game: Some(game),
                jvm: Some(jvm),
            })
        }

        // inheriting (overriding reverse)
        merged.inherits_from = lower.inherits_from.or(self.inherits_from);

        // asset (overriding)
        merged.asset_index = self.asset_index.or(lower.asset_index);

        // asset (overriding)
        merged.assets = self.assets.or(lower.assets);

        // compliance (overriding)
        merged.compliance_level = self.compliance_level.or(lower.compliance_level);

        // download (overriding)
        merged.downloads = self.downloads.or(lower.downloads);

        // id (overriding)
        merged.id = self.id.or(lower.id);

        // java version (overriding)
        merged.java_version = self.java_version.or(lower.java_version);

        // library (combining)
        merged.libraries = Some(
            self.libraries
                .unwrap_or_default()
                .into_iter()
                .chain(lower.libraries.unwrap_or_default().into_iter())
                .collect(),
        );

        // main class (overriding)
        merged.main_class = self.main_class.or(lower.main_class);

        // minimum launcher version (overriding)
        merged.minimum_launcher_version = self
            .minimum_launcher_version
            .or(lower.minimum_launcher_version);

        // release time (overriding)
        merged.release_time = self.release_time.or(lower.release_time);

        // time (overriding)
        merged.time = self.time.or(lower.time);

        // version info type (overriding)
        merged.version_info_type = self.version_info_type.or(lower.version_info_type);

        merged
    }

    #[tracing::instrument]
    pub fn save_json(&self, save_path: PathBuf) -> Result<(), VersionError> {
        debug!("Saving version to {}", save_path.display());
        // serialize the struct to a json string
        let json = serde_json::to_string(self)?;
        trace!("version JSON: {}", json);
        create_dir_all(save_path.parent().ok_or(VersionError::NoPathParent)?)?;

        debug!("Creating file at {}", save_path.display());
        let mut file = std::fs::File::create(&save_path)?;
        debug!("Writing version file to file");
        file.write_all(json.as_bytes())?;

        debug!("Saved version file to {}", &save_path.display());
        Ok(())
    }

    #[tracing::instrument]
    pub async fn asset_index(&self) -> Result<super::asset_index::AssetIndex, VersionError> {
        trace!("Downloading asset index");
        // Get json and return it
        Ok(reqwest::get(
            &self
                .asset_index
                .as_ref()
                .ok_or(VersionError::NoAssetIndex)?
                .url,
        )
        .await?
        .json::<super::asset_index::AssetIndex>()
        .await?)
    }

    #[tracing::instrument]
    pub async fn download_libraries(
        &self,
        save_path: PathBuf,
        client: reqwest::Client,
    ) -> Result<ListOfResultHandles, VersionError> {
        debug!("Downloading libraries");
        let client = create_client();

        let tasks = FuturesUnordered::new();

        for library in self.libraries.as_ref().ok_or(VersionError::NoLibs)? {
            // Check rules for the library to see if it should be downloaded
            if let Some(rules) = &library.rules {
                debug!("Library {} has rules, checking them", library.name);
                // if the rules are not satisfied, skip the library
                if !Version::check_library_rules(rules) {
                    continue;
                }
            }

            debug!(
                "Library {} has no rules or the rules passed, downloading",
                library.name
            );

            // if we get here, then the library is allowed to be downloaded
            let download = if let Some(down) = &library.downloads {
                down.to_owned()
            } else {
                create_library_download(
                    library.url.as_ref().unwrap(),
                    &library.name,
                    client.clone(),
                )
                .await?
            };

            Self::create_save_task(&download.artifact, &save_path, library, &tasks, &client);

            if let Some(classifiers) = &download.classifiers {
                match std::env::consts::OS {
                    "windows" => {
                        if let Some(windows) = &classifiers.natives_windows {
                            Self::create_save_task(windows, &save_path, library, &tasks, &client);
                        } else {
                            continue;
                        }
                    }
                    "macos" => {
                        if let Some(macos) = &classifiers.natives_macos {
                            Self::create_save_task(macos, &save_path, library, &tasks, &client);
                        } else if let Some(osx) = &classifiers.natives_osx {
                            Self::create_save_task(osx, &save_path, library, &tasks, &client);
                        } else {
                            continue;
                        }
                    }
                    "linux" => {
                        if let Some(linux) = &classifiers.natives_linux {
                            Self::create_save_task(linux, &save_path, library, &tasks, &client);
                        } else {
                            continue;
                        }
                    }
                    _ => return Err(VersionError::UnsupportedOs),
                };
            }
        }

        debug!("Created {} library download tasks", tasks.len());
        Ok(tasks)
    }

    #[tracing::instrument]
    async fn run_downloads(
        mut tasks: ListOfResultHandles,
        progress_sender: Sender<DownloadProgress>,
    ) {
        trace!("Running library download tasks");
        let total = tasks.len();
        let mut finished = 0;

        while (tasks.next().await).is_some() {
            finished += 1;
            debug!("{}/{} library downloads finished", finished, total);
            let _ = progress_sender.send(DownloadProgress {
                total_size: total as u64,
                finished,
            });
        }
    }

    #[tracing::instrument]
    pub async fn start_download_libraries(
        &self,
        save_path: PathBuf,
        client: reqwest::Client,
    ) -> Result<DownloadWatcher, VersionError> {
        trace!("Starting download libraries");
        trace!("Creating progress watcher");
        let (progress_sender, progress_receiver) = watch::channel(DownloadProgress {
            finished: 0,
            total_size: 0,
        });

        trace!("Creating download tasks");
        let tasks = self.download_libraries(save_path, client).await?;
        trace!("Starting download tasks");
        let download_task = task::spawn(Self::run_downloads(tasks, progress_sender));

        Ok(DownloadWatcher {
            progress_watcher: progress_receiver,
            download_task,
        })
    }

    #[tracing::instrument]
    pub async fn download_client_jar(&self, save_path: PathBuf) -> Result<(), VersionError> {
        let url = self
            .downloads
            .as_ref()
            .ok_or(VersionError::NoDownloads)?
            .client
            .url
            .clone();

        let task = tokio::spawn(create_download_task(url, save_path, None));

        // the ultimate jank
        task.await???;

        Ok(())
    }

    #[tracing::instrument]
    pub async fn download_server_jar(&self, save_path: PathBuf) -> Result<(), VersionError> {
        let url = self
            .downloads
            .as_ref()
            .ok_or(VersionError::NoDownloads)?
            .server
            .url
            .clone();
        let task = tokio::spawn(create_download_task(url, save_path, None));

        // the ultimate jank
        task.await???;

        Ok(())
    }

    #[tracing::instrument]
    pub fn check_library_rules(rules: &Vec<LibraryRule>) -> bool {
        for rule in rules {
            match rule.action {
                Action::Allow => {
                    if let Some(os) = &rule.os {
                        if match os.name {
                            Name::Linux => cfg!(target_os = "linux"),
                            Name::Osx => cfg!(target_os = "macos"),
                            Name::Windows => cfg!(target_os = "windows"),
                        } {
                            // continue going through the rules
                            continue;
                        } else {
                            // continue the loop because this library is not for this OS
                            return false;
                        }
                    } else {
                        // continue as this rule does not have an OS
                        continue;
                    }
                }
                Action::Disallow => {
                    if let Some(os) = &rule.os {
                        if match os.name {
                            Name::Linux => cfg!(target_os = "linux"),
                            Name::Osx => cfg!(target_os = "macos"),
                            Name::Windows => cfg!(target_os = "windows"),
                        } {
                            return false;
                        } else {
                            // continue going through the rules
                            continue;
                        }
                    } else {
                        // continue the loop because this library is not allowed for any OS
                        // (mojank moment)
                        return false;
                    }
                }
            }
        }

        true
    }

    #[tracing::instrument]
    fn create_save_task(
        mappings_class: &MappingsClass,
        save_path: &PathBuf,
        library: &Library,
        tasks: &FuturesUnordered<task::JoinHandle<Result<(), DownloadError>>>,
        client: &reqwest::Client,
    ) {
        let url = mappings_class.url.clone();
        let sub_path = mappings_class
        .path
        .as_ref()
        .expect("library doesnt have a path. Please report this bug to https://github.com/glowsquid-launcher/glowsquid/issues");

        let full_path = save_path.join(sub_path);
        debug!(
            "Creating download task for library {}, saving to {}",
            library.name,
            full_path.display()
        );
        tasks.push(create_download_task(url, full_path, Some(client.clone())));
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Arguments {
    pub game: Option<Vec<GameElement>>,
    pub jvm: Option<Vec<JvmElement>>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct GameClass {
    pub rules: Vec<GameRule>,
    pub value: Value,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct GameRule {
    pub action: Action,
    pub features: Features,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Features {
    pub is_demo_user: Option<bool>,
    pub has_custom_resolution: Option<bool>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct JvmClass {
    pub rules: Vec<JvmRule>,
    pub value: Value,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct JvmRule {
    pub action: Action,
    pub os: PurpleOs,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct PurpleOs {
    pub name: Option<String>,
    pub version: Option<String>,
    pub arch: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct AssetIndex {
    pub id: String,
    pub sha1: String,
    pub size: i64,
    #[serde(rename = "totalSize")]
    pub total_size: Option<i64>,
    pub url: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct VersionInfoDownloads {
    pub client: MappingsClass,
    pub client_mappings: MappingsClass,
    pub server: MappingsClass,
    pub server_mappings: MappingsClass,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct MappingsClass {
    pub sha1: String,
    pub size: u64,
    pub url: String,
    pub path: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct JavaVersion {
    pub component: String,
    #[serde(rename = "majorVersion")]
    pub major_version: i64,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Library {
    pub downloads: Option<LibraryDownloads>,
    pub url: Option<String>,
    pub name: String,
    pub rules: Option<Vec<LibraryRule>>,
    pub natives: Option<Natives>,
    pub extract: Option<Extract>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct LibraryDownloads {
    pub artifact: MappingsClass,
    pub classifiers: Option<Classifiers>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Classifiers {
    pub javadoc: Option<MappingsClass>,
    #[serde(rename = "natives-linux")]
    pub natives_linux: Option<MappingsClass>,
    #[serde(rename = "natives-macos")]
    pub natives_macos: Option<MappingsClass>,
    #[serde(rename = "natives-windows")]
    pub natives_windows: Option<MappingsClass>,
    pub sources: Option<MappingsClass>,
    #[serde(rename = "natives-osx")]
    pub natives_osx: Option<MappingsClass>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Extract {
    pub exclude: Vec<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Natives {
    pub osx: Option<String>,
    pub linux: Option<String>,
    pub windows: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct LibraryRule {
    pub action: Action,
    pub os: Option<FluffyOs>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct FluffyOs {
    pub name: Name,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Logging {
    pub client: LoggingClient,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct LoggingClient {
    pub argument: String,
    pub file: AssetIndex,
    #[serde(rename = "type")]
    pub client_type: String,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(untagged)]
pub enum GameElement {
    GameClass(GameClass),
    String(String),
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(untagged)]
pub enum Value {
    String(String),
    StringArray(Vec<String>),
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(untagged)]
pub enum JvmElement {
    JvmClass(JvmClass),
    String(String),
}

#[derive(Debug, Serialize, Deserialize)]
pub enum Action {
    #[serde(rename = "allow")]
    Allow,
    #[serde(rename = "disallow")]
    Disallow,
}

#[derive(Debug, Serialize, Deserialize)]
pub enum Name {
    #[serde(rename = "osx")]
    Osx,
    #[serde(rename = "linux")]
    Linux,
    #[serde(rename = "windows")]
    Windows,
}
