use dunce::canonicalize;
use tracing::{debug, trace};

use crate::assets::structs::version::{Action, GameRule, JvmRule, Value, Version};
use crate::assets::structs::version::{GameClass, JvmClass};
use crate::errors::JavaArgumentsError;
use crate::launcher::Launcher;
use crate::util::create_library_download;

#[cfg(target_os = "windows")]
use winsafe::IsWindows10OrGreater;
// Windows users, please test this ^

pub struct GameArguments;
pub struct JavaArguments;

impl GameArguments {
    #[tracing::instrument]
    pub fn parse_class_argument(
        launcher_arguments: &Launcher,
        argument: &GameClass,
    ) -> Result<Option<String>, JavaArgumentsError> {
        debug!("Parsing class argument: {:?}", argument);

        let checks = argument
            .rules
            .iter()
            .map(|rule| Self::check_rule(rule, launcher_arguments));

        let rules_passed = itertools::process_results(checks, |mut iter| {
            iter.any(|rule| {
                debug!("Checking rule: {:?}", rule);
                rule
            })
        })?;

        if !rules_passed {
            Ok(None)
        } else {
            Ok(Some(Self::parse_string_argument(
                launcher_arguments,
                match &argument.value {
                    Value::String(str) => str.to_string(),
                    Value::StringArray(array) => array.join(" "),
                },
            )?))
        }
    }

    #[tracing::instrument]
    pub fn parse_string_argument(
        launcher_arguments: &Launcher,
        argument: String,
    ) -> Result<String, JavaArgumentsError> {
        trace!("Parsing string argument: {:?}", &argument);

        if argument.starts_with("${") && argument.ends_with('}') {
            let dynamic_argument = &argument[2..argument.len() - 1].to_string();
            Ok(Self::match_dynamic_argument(
                launcher_arguments,
                dynamic_argument,
            )?)
        } else if argument == "--clientId" {
            if launcher_arguments
                .authentication_details
                .client_id
                .is_some()
            {
                Ok(argument)
            } else {
                Ok("".to_string()) // dont put in argument if there is no client id
            }
        } else {
            Ok(argument)
        }
    }

    #[tracing::instrument]
    fn match_dynamic_argument(
        launcher_arguments: &Launcher,
        dynamic_argument: &str,
    ) -> Result<String, JavaArgumentsError> {
        // This is based of the 1.18 JSON. This assumes that all accounts are microsoft accounts
        // (As Mojang accounts are being deprecated and soon erased from existence).

        trace!("Matching dynamic argument: {:?}", &dynamic_argument);
        let client_id = launcher_arguments
            .authentication_details
            .client_id
            .as_ref()
            .unwrap_or(&"".to_string())
            .clone();

        Ok(match dynamic_argument {
            "auth_player_name" => launcher_arguments
                .authentication_details
                .username
                .to_owned(),
            "version_name" => launcher_arguments.version_name.to_owned(),
            "game_directory" => canonicalize(&launcher_arguments.game_directory)?
                .to_str()
                .ok_or(JavaArgumentsError::NotValidUtf8Path)?
                .to_owned(),
            "assets_root" => canonicalize(&launcher_arguments.assets_directory)?
                .to_str()
                .ok_or(JavaArgumentsError::NotValidUtf8Path)?
                .to_owned(),
            "assets_index_name" => launcher_arguments.version_name.to_owned(),
            "auth_uuid" => launcher_arguments.authentication_details.uuid.to_owned(),
            "auth_access_token" => launcher_arguments
                .authentication_details
                .access_token
                .to_owned(),
            "clientid" => client_id,
            "auth_xuid" => launcher_arguments
                .authentication_details
                .xbox_uid
                .to_owned(),
            // we assume that the user is a microsoft account
            "user_type" => "msa".to_string(),
            "version_type" => {
                if launcher_arguments.is_snapshot {
                    "snapshot".to_string()
                } else {
                    "release".to_string()
                }
            }
            "resolution_width" if launcher_arguments.custom_resolution.is_some() => {
                launcher_arguments
                    .custom_resolution
                    .as_ref()
                    .ok_or(JavaArgumentsError::NoCustomResolutionProvided)?
                    .width
                    .to_string()
            }
            "resolution_height" if launcher_arguments.custom_resolution.is_some() => {
                launcher_arguments
                    .custom_resolution
                    .as_ref()
                    .ok_or(JavaArgumentsError::NoCustomResolutionProvided)?
                    .height
                    .to_string()
            }
            _ => {
                return Err(JavaArgumentsError::UnrecognisedGameArgument(
                    dynamic_argument.to_string(),
                ))
            }
        })
    }

    #[tracing::instrument]
    fn check_rule(
        rule: &GameRule,
        launcher_arguments: &Launcher,
    ) -> Result<bool, JavaArgumentsError> {
        // based of the 1.18 json
        match rule.action {
            Action::Allow => {
                if rule.features.is_demo_user.is_some() {
                    Ok(launcher_arguments.authentication_details.is_demo_user)
                } else if rule.features.has_custom_resolution.is_some() {
                    Ok(launcher_arguments.custom_resolution.is_some())
                } else {
                    Err(JavaArgumentsError::UnrecognisedAllowRule)
                }
            }
            // no disallows yet
            Action::Disallow => Err(JavaArgumentsError::UnrecognisedDisallowRule),
        }
    }
}

impl JavaArguments {
    #[tracing::instrument]
    pub async fn parse_string_argument(
        launcher_arguments: &Launcher,
        version_manifest: &Version,
        argument: String,
        client: reqwest::Client,
    ) -> Result<String, JavaArgumentsError> {
        let classpath =
            Self::create_classpath(version_manifest, launcher_arguments, client).await?;

        Ok(argument
            .replace(
                "${natives_directory}",
                //TODO: Add compat with mc version <= 1.16.5 which uses <version>/natives
                canonicalize(&launcher_arguments.libraries_directory)?
                    .to_str()
                    .ok_or(JavaArgumentsError::NotValidUtf8Path)?,
            )
            .replace("${launcher_name}", &launcher_arguments.client_branding)
            .replace("${launcher_version}", &launcher_arguments.launcher_name)
            .replace(
                "${classpath}",
                classpath
                    .join(if cfg!(windows) { ";" } else { ":" })
                    .as_str(),
            ))
    }

    #[tracing::instrument]
    pub async fn parse_class_argument(
        launcher_arguments: &Launcher,
        version_manifest: &Version,
        argument: &JvmClass,
        client: reqwest::Client,
    ) -> Result<Option<String>, JavaArgumentsError> {
        for rule in &argument.rules {
            if !Self::check_rule(rule)? {
                return Ok(None);
            }
        }

        Ok(Some(
            Self::parse_string_argument(
                launcher_arguments,
                version_manifest,
                match &argument.value {
                    Value::String(str) => str.to_string(),
                    Value::StringArray(array) => array.join(" "),
                },
                client,
            )
            .await?,
        ))
    }

    #[tracing::instrument]
    fn check_rule(rule: &JvmRule) -> Result<bool, JavaArgumentsError> {
        let mut current_allow = false;

        match rule.action {
            Action::Allow => {
                if let Some(name) = &rule.os.name {
                    current_allow = match &*name.to_owned() {
                        "osx" => cfg!(target_os = "macos"),
                        #[cfg(target_os = "windows")]
                        "windows" => {
                            if let Some(ver) = &rule.os.version {
                                if ver != "^10\\." {
                                    panic!("unrecognised windows version: {:?}, please report to https://github.com/glowsquid-launcher/minecraft-rs/issues with the version you are using", ver);
                                }

                                IsWindows10OrGreater().unwrap_or(false)
                            } else {
                                true
                            }
                        }
                        #[cfg(not(target_os = "windows"))]
                        "windows" => false,
                        "linux" => cfg!(target_os = "linux"),
                        _ => return Err(JavaArgumentsError::UnrecognisedOs),
                    };
                }

                if !current_allow {
                    return Ok(false);
                }

                if let Some(arch) = &rule.os.arch {
                    match &*arch.to_owned() {
                        "x86" => current_allow = cfg!(target_arch = "x86"),
                        _ => return Err(JavaArgumentsError::UnrecognisedOsArch),
                    }
                }
            }
            Action::Disallow => return Err(JavaArgumentsError::NoDissalows),
        }
        Ok(current_allow)
    }

    #[tracing::instrument]
    async fn create_classpath(
        version_manifest: &Version,
        launcher_arguments: &Launcher,
        client: reqwest::Client,
    ) -> Result<Vec<String>, JavaArgumentsError> {
        let mut cp = vec![];

        for library in version_manifest
            .libraries
            .as_ref()
            .ok_or(JavaArgumentsError::NoLibrariesFound)?
        {
            if let Some(rules) = &library.rules {
                if !Version::check_library_rules(rules) {
                    continue;
                }
            }

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

            cp.push(
                canonicalize(
                    launcher_arguments.libraries_directory.join(
                        download
                            .artifact
                            .path
                            .as_ref()
                            .ok_or(JavaArgumentsError::NoDownloadArtifactPath)?,
                    ),
                )?
                .to_str()
                .ok_or(JavaArgumentsError::NotValidUtf8Path)?
                .to_owned(),
            );

            if let Some(classifiers) = &download.classifiers {
                match std::env::consts::OS {
                    "windows" => {
                        if let Some(windows) = &classifiers.natives_windows {
                            cp.push(
                                canonicalize(
                                    launcher_arguments.libraries_directory.join(
                                        windows
                                            .path
                                            .as_ref()
                                            .ok_or(JavaArgumentsError::NoLibsPath)?,
                                    ),
                                )?
                                .to_str()
                                .ok_or(JavaArgumentsError::NotValidUtf8Path)?
                                .to_owned(),
                            );
                        } else {
                            continue;
                        }
                    }
                    "macos" => {
                        if let Some(macos) = &classifiers.natives_macos {
                            cp.push(
                                canonicalize(launcher_arguments.libraries_directory.join(
                                    macos.path.as_ref().ok_or(JavaArgumentsError::NoLibsPath)?,
                                ))?
                                .to_str()
                                .ok_or(JavaArgumentsError::NotValidUtf8Path)?
                                .to_owned(),
                            );
                        } else if let Some(osx) = &classifiers.natives_osx {
                            cp.push(
                                canonicalize(launcher_arguments.libraries_directory.join(
                                    osx.path.as_ref().ok_or(JavaArgumentsError::NoLibsPath)?,
                                ))?
                                .to_str()
                                .ok_or(JavaArgumentsError::NotValidUtf8Path)?
                                .to_owned(),
                            )
                        } else {
                            continue;
                        }
                    }
                    "linux" => {
                        if let Some(linux) = &classifiers.natives_linux {
                            cp.push(
                                canonicalize(launcher_arguments.libraries_directory.join(
                                    linux.path.as_ref().ok_or(JavaArgumentsError::NoLibsPath)?,
                                ))?
                                .to_str()
                                .ok_or(JavaArgumentsError::NotValidUtf8Path)?
                                .to_owned(),
                            );
                        } else {
                            continue;
                        }
                    }
                    _ => continue,
                };
            }
        }

        cp.push(
            canonicalize(&launcher_arguments.jar_path)?
                .to_str()
                .ok_or(JavaArgumentsError::NotValidUtf8Path)?
                .to_owned(),
        );

        Ok(cp)
    }
}
