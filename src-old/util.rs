use std::path::PathBuf;
use std::str::FromStr;
use std::time::Duration;

use futures::stream::FuturesUnordered;
use futures::StreamExt;
use reqwest::{Client, ClientBuilder};
use tokio::fs::create_dir_all;
use tokio::io::AsyncWriteExt;
use tokio::sync::watch::Receiver;
use tokio::task::{self, JoinHandle};
use tokio_retry::{strategy::FixedInterval, Retry};
use tracing::{debug, trace};

use crate::assets::structs::version::{LibraryDownloads, MappingsClass};
use crate::errors::{CreateLibraryDownloadError, DownloadError, MavenIdentifierParseError};

#[tracing::instrument]
pub fn create_download_task(
    url: String,
    path: PathBuf,
    client: Option<Client>,
) -> JoinHandle<Result<(), DownloadError>> {
    trace!("Creating download task for {}", url);
    tokio::spawn(async move {
        let client = client.clone().unwrap_or_else(create_client);

        create_dir_all(&path.parent().ok_or(DownloadError::NoPathParent)?).await?;

        // idk how to get rid of clone
        // hours wasted: 2
        let action = || {
            debug!("Attempting to download {}", url);
            client.get(url.clone()).send()
        };

        let retry_strategy = FixedInterval::from_millis(100).take(3);

        let response = Retry::spawn(retry_strategy, action).await?;

        trace!("Creating file at {}", &path.display());
        let mut file = tokio::fs::File::create(&path).await?;

        trace!("Writing response to file");
        let mut stream = response.bytes_stream();

        while let Some(item) = stream.next().await {
            let chunk = item?;
            file.write_all(&chunk).await?;
        }
        trace!("Wrote response to file");

        debug!("Downloaded {}", url);
        Ok(())
    })
}

pub type ListOfResultHandles = FuturesUnordered<task::JoinHandle<Result<(), DownloadError>>>;

// net.fabricmc:tiny-mappings-parser:0.3.0+build.17
pub struct MavenIdentifier {
    pub group_id: String,
    pub artifact_id: String,
    pub version: String,
}

impl FromStr for MavenIdentifier {
    type Err = MavenIdentifierParseError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let mut parts = s.split(':');

        let [group_id, artifact_id, version] = parts
            .next()
            .zip(parts.next())
            .zip(parts.next())
            .map(|((group, artifact), version)| [group, artifact, version]) //Option::map
            .ok_or(MavenIdentifierParseError::NotEnoughArgs)?
            .map(ToString::to_string); //array::map

        Ok(Self {
            group_id,
            artifact_id,
            version,
        })
    }
}

pub async fn create_library_download(
    url: &str,
    name: &str,
    client: Client,
) -> Result<LibraryDownloads, CreateLibraryDownloadError> {
    let identifier = MavenIdentifier::from_str(name)?;

    let maven_url = format!(
        "{}/{}/{}/{}-{}.jar",
        identifier.group_id.replace('.', "/"),
        identifier.artifact_id,
        identifier.version,
        identifier.artifact_id,
        identifier.version,
    );

    let download_url = format!("{}{}", &url, &maven_url);

    let size = client
        .head(download_url.clone())
        .send()
        .await?
        .content_length()
        .ok_or(CreateLibraryDownloadError::NoContentLength)?;

    let sha1 = client
        .get(format!("{}.sha1", &download_url))
        .send()
        .await?
        .text()
        .await?;

    Ok(LibraryDownloads {
        artifact: MappingsClass {
            sha1,
            size,
            url: download_url,
            path: Some(maven_url),
        },
        classifiers: None,
    })
}

#[derive(Clone, Copy, Debug)]
pub struct DownloadProgress {
    pub total_size: u64,
    pub finished: u64,
}

pub struct DownloadWatcher {
    pub progress_watcher: Receiver<DownloadProgress>,
    pub download_task: JoinHandle<()>,
}

pub fn create_client() -> Client {
    ClientBuilder::new()
        .connection_verbose(true)
        .pool_idle_timeout(Some(Duration::from_secs(600)))
        .tcp_keepalive(Some(Duration::from_secs(30)))
        .build()
        .unwrap()
}
