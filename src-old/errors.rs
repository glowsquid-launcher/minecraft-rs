use serde::{Deserialize, Serialize};
use serde_repr::{Deserialize_repr, Serialize_repr};
use std::error::Error as StdError;
use std::fmt::{Display, Formatter};
use thiserror::Error;

#[derive(Debug)]
pub struct InternalReqwestError(pub reqwest::Error);

impl Display for InternalReqwestError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl Serialize for InternalReqwestError {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_str(&self.0.to_string())
    }
}

impl From<reqwest::Error> for InternalReqwestError {
    fn from(err: reqwest::Error) -> Self {
        InternalReqwestError(err)
    }
}

impl StdError for InternalReqwestError {}

#[derive(Debug)]
pub struct InternalJWTError(pub jwt_simple::Error);

impl Display for InternalJWTError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl Serialize for InternalJWTError {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_str(&self.0.to_string())
    }
}

impl From<jwt_simple::Error> for InternalJWTError {
    fn from(err: jwt_simple::Error) -> Self {
        InternalJWTError(err)
    }
}

impl StdError for InternalJWTError {}

#[derive(Error, Debug)]
/// Errors relating to downloading and parsing a minecraft version manifest
pub enum VersionError {
    #[error("version.serde_error(error={0})")]
    /// serde_json failed to serialize/deserialize an error
    SerdeError(#[from] serde_json::Error),

    #[error("version.no_path_parent")]
    /// The save path doesn't have a parent this happens if you do not specify the file name
    /// usually
    NoPathParent,

    #[error("version.io_error(error={0})")]
    /// An error happened during an IO operation
    IoError(#[from] std::io::Error),

    #[error("version.no_asset_index")]
    /// an asset index was not provided by the version manifest
    ///
    /// This usually happens when you forget to merge e.g the fabric manifest with the base one
    NoAssetIndex,

    #[error("version.request_error(error={0})")]
    /// An error happened with reqwest.
    RequestError(#[from] reqwest::Error),

    #[error("version.no_libs")]
    /// No libraries were provided by the version manifest
    ///
    /// This usually happens when you forget to merge e.g the fabric manifest with the base one
    NoLibs,

    #[error("version.unsupported_os")]
    /// The OS the app is running on is unsupported. This shouldn't happen. If it does, please file
    /// a bug report
    UnsupportedOs,

    #[error("versions.no_downloads")]
    /// No downloads were provided by the version manifest
    ///
    /// This usually happens when you forget to merge e.g the fabric manifest with the base one
    NoDownloads,

    #[error("version.download_error(error={0})")]
    /// An error happened during a download
    DownloadErr(#[from] DownloadError),

    #[error("version.join_error")]
    /// An error happened when trying to join/wait for a threads output
    JoinError(#[from] tokio::task::JoinError),

    #[error("version.library_download_error(error={0})")]
    /// An error happened during creating a library download from a maven url
    LibraryDownloadError(#[from] CreateLibraryDownloadError),
}

#[derive(Error, Debug)]
pub enum DownloadError {
    #[error("download.no_path_parent")]
    /// The save path doesn't have a parent this happens if you do not specify the file name
    /// usually
    NoPathParent,

    #[error("download.io_error(error={0})")]
    /// An error happened during an IO operation
    IoError(#[from] std::io::Error),

    #[error("download.request_error(error={0})")]
    /// An error happened with reqwest.
    RequestError(#[from] reqwest::Error),
}

#[derive(Debug, Error)]
pub enum LauncherError {
    #[error("launcher.io_error(error={0})")]
    /// An error happened during an IO operation
    IoError(#[from] std::io::Error),

    #[error("launcher.serde_error(error={0})")]
    /// serde_json failed to serialize/deserialize an error
    SerdeError(#[from] serde_json::Error),

    #[error("launcher.argument_parse_error(error={0})")]
    /// An error happened when parsing arguments
    JavaArgumentParseError(#[from] JavaArgumentsError),

    #[error("launcher.cannot_get_stdout")]
    /// Cannot get the stdout stream from the minecraft process
    CannotGetStdout,

    #[error("launcher.cannot_get_stderr")]
    /// Cannot get the stderr stream from the minecraft process
    CannotGetStderr,

    #[error("Launcher.no_main")]
    /// a main class was not provided by the version manifest
    ///
    /// This usually happens when you forget to merge e.g A manifest that doesn't have a modified
    /// main class with the base one
    NoMainClass,

    #[error("Launcher.no_args")]
    /// arguments were not provided by the version manifest
    ///
    /// This usually happens when you forget to merge e.g A manifest that doesn't have any new args with the base one
    NoArgs,
}

#[derive(Debug, Error)]
pub enum JavaArgumentsError {
    #[error("java_arguments.request_error(error={0})")]
    /// An error happened with reqwest.
    RequestError(#[from] reqwest::Error),

    #[error("java_arguments.no_libs")]
    /// libs were not provided by the version manifest
    ///
    /// This usually happens when you forget to merge e.g A manifest that doesn't have any new libs with the base one
    NoLibrariesFound,

    #[error("java_arguments.not_valid_utf8_path")]
    /// A path is not valid UTF-8.
    NotValidUtf8Path,

    #[error("java_arguments.io_error(error={0})")]
    /// An error happened during an IO operation
    IoError(#[from] std::io::Error),

    #[error("java_arguments.no_download_artifact_path")]
    /// a download artifact path was not provided by the version manifest
    ///
    /// This usually happens when you forget to merge e.g A manifest that doesn't have a modified
    /// download manifest path with the base one
    NoDownloadArtifactPath,

    #[error("java_arguments.no_libs_path")]
    /// No lib path was found
    ///
    /// this _shouldn't_ happen, but in case it does, this exists
    NoLibsPath,

    #[error("java_arguments.unrecognised_os")]
    /// The OS in the arguments is not recognized. This shouldn't happen, if it does, file a bug
    UnrecognisedOs,

    #[error("java_arguments.unrecognised_os_arch")]
    /// The OS arch in the arguments is not recognized. This shouldn't happen, if it does, file a bug
    UnrecognisedOsArch,

    #[error("java_arguments.library_download_error(error={0})")]
    /// An error happened during creating a library download from a maven url
    LibraryDownloadError(#[from] CreateLibraryDownloadError),

    #[error("java_arguments.no_disallows")]
    /// No disallows are currently implemented. Please file a bug if this error happens
    NoDisallows,

    #[error("java_arguments.no_custom_resolution")]
    /// No custom resolution was provided
    ///
    /// this _should NEVER_ happen, but in case it does, this exists. Please file a bug report.
    NoCustomResolutionProvided,

    #[error("java_arguments.unrecognised_game_argument(arg={0})")]
    /// The launcher encountered a game argument it doesn't know about
    ///
    /// If this happens, report it as a bug
    UnrecognisedGameArgument(String),

    #[error("java_arguments.unrecognised_allow_rule")]
    /// The launcher encountered an allow rule it doesn't know about
    ///
    /// If this happens, report it as a bug
    UnrecognisedAllowRule,

    #[error("java_arguments.unrecognised_disallow_rule")]
    /// The launcher encountered a disallow rule it doesn't know about
    ///
    /// If this happens, report it as a bug
    UnrecognisedDisallowRule,
}

#[derive(Error, Debug)]
pub enum SaveError {
    #[error("save.io_error(error={0})")]
    /// An error happened during an IO operation
    IoError(#[from] std::io::Error),

    #[error("save.serde_error(error={0})")]
    /// serde_json failed to serialize/deserialize an error
    SerdeError(#[from] serde_json::Error),

    #[error("save.no_parent_path")]
    /// A path didn't have a parent
    ///
    /// This can happen if you forgot to include the file name
    NoParentPath,

    #[error("save.not_valid_utf8_path")]
    /// A path is not valid UTF-8.
    NotValidUtf8Path,
}

#[derive(Error, Debug)]
pub enum MavenIdentifierParseError {
    #[error("maven_parse.not_enough_args")]
    /// There were not enough `:` in the string to properly parse it
    NotEnoughArgs,
}

#[derive(Error, Debug)]
pub enum CreateLibraryDownloadError {
    #[error("library_download.reqwest_error")]
    /// An error happened with reqwest.
    RequestError(#[from] reqwest::Error),

    #[error("library_download.maven_parse_error(error={0})")]
    /// An error happened during a maven parse
    MavenParseError(#[from] MavenIdentifierParseError),

    #[error("library_download.no_content_length_header")]
    /// No content-length was provided from the HEAD request made to the maven server
    NoContentLength,

    #[error("library_download.cannot_parse_content_length")]
    /// content-length is not a valid number
    CannotParseContentLength,
}

#[derive(Error, Debug, Serialize)]
pub enum DeviceCodeError {
    #[error("auth_error.request_error(error={})", .0)]
    RequestError(#[from] InternalReqwestError),

    #[error("auth_error.microsoft_error(error={})", .0)]
    MicrosoftError(String),
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct InternalAuthTokenError {
    pub error: AuthTokenErrorType,
    pub error_description: String,
    pub error_uri: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "snake_case")]
pub enum AuthTokenErrorType {
    /// Continue polling
    AuthorizationPending,
    /// Stop polling
    AuthorizationDeclined,
    /// Stop polling
    ExpiredToken,
    /// Client should verify that the code is correct
    BadVerificationCode,
}

#[derive(Error, Debug, Serialize)]
pub enum XstsError {
    #[error("xsts_error.request_error(error={})", .0)]
    ReqwestError(#[from] InternalReqwestError),

    #[error("xsts_error.microsoft_error(error={})", .0)]
    MicrosoftError(XstsMsError),
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "PascalCase")]
pub struct XstsMsError {
    pub identity: String,
    pub x_err: XErr,
    pub message: String,
    /// This usually does not resolve to anywhere
    pub redirect: String,
}

impl Display for XstsMsError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}: {}", self.x_err, self.message)
    }
}

#[derive(Debug, Serialize_repr, Deserialize_repr, Copy, Clone)]
#[repr(u32)]
pub enum XErr {
    NoXboxAccount = 2148916233,
    /// Xbox live is banned in the users country
    XboxLiveCountryBanned = 2148916235,
    /// The user needs to be verified as an adult in South Korea
    SouthKoreaAdultVerificationRequired = 2148916236,
    /// The user needs to be verified as an adult in South Korea
    SouthKoreaAdultVerificationRequired2 = 2148916237,
}

impl Display for XErr {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", *self as u32)
    }
}

#[derive(Error, Debug, Serialize)]
pub enum JWTVerificationError {
    #[error("jwt_error.request_error(error={})", .0)]
    RequestError(#[from] InternalReqwestError),
    #[error("jwt_error.verify_error(error={})", .0)]
    VerificationError(#[from] InternalJWTError),
}

#[derive(Error, Debug)]
pub enum MinecraftProfileError {
    #[error("minecraft_profile_error.request_error(error={})", .0)]
    ReqwestError(#[from] reqwest::Error),
    #[error("minecraft_profile_error.profile_not_found")]
    /// Note that Xbox Game Pass users who haven't logged into the new Minecraft Launcher at least once will not return a profile, and will need to login once after activating Xbox Game Pass to setup their Minecraft username.
    NotFound,
}
