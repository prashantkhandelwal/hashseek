//! Retrieve BitTorrent v1 metadata without downloading torrent content.

use std::{net::SocketAddr, path::PathBuf, time::Duration};

use anyhow::{Context, Result, bail};
use librqbit::{
    AddTorrent, AddTorrentOptions, AddTorrentResponse, DhtSessionConfig, Magnet, Session,
    SessionOptions,
};
use tokio::time::timeout;
use url::Url;

/// Public trackers used when the input does not already contain them.
pub const DEFAULT_TRACKERS: &[&str] = &[
    "udp://tracker.opentrackr.org:1337/announce",
    "udp://open.stealth.si:80/announce",
    "udp://tracker.torrent.eu.org:451/announce",
    "https://tracker2.ctix.cn:443/announce",
];

/// Configuration for a metadata lookup.
#[derive(Clone, Debug)]
pub struct LookupOptions {
    /// Maximum time to wait for peers to provide the torrent metadata.
    pub timeout: Duration,
    /// Trackers appended to the input magnet URI when they are not already present.
    pub fallback_trackers: Vec<String>,
    /// Temporary session folder. No torrent content is written in metadata-only mode.
    pub session_folder: PathBuf,
}

impl Default for LookupOptions {
    fn default() -> Self {
        Self {
            timeout: Duration::from_secs(120),
            fallback_trackers: DEFAULT_TRACKERS
                .iter()
                .map(|tracker| (*tracker).to_owned())
                .collect(),
            session_folder: std::env::temp_dir().join("hashseek-metadata"),
        }
    }
}

/// A file described by the torrent metadata.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TorrentFile {
    pub path: String,
    pub size: u64,
}

/// Metadata discovered for a torrent.
#[derive(Clone, Debug)]
pub struct TorrentInfo {
    pub name: Option<String>,
    pub info_hash: String,
    pub total_size: u64,
    pub total_pieces: u32,
    pub files: Vec<TorrentFile>,
    /// Peers encountered while resolving the magnet metadata.
    pub discovered_peers: Vec<SocketAddr>,
    /// A tracker scrape is required to populate this; DHT does not provide it.
    pub seeders: Option<u64>,
}

/// Retrieves metadata for a BitTorrent v1 infohash or magnet URI.
///
/// The torrent is added in `list_only` mode, so payload content is not downloaded.
pub async fn lookup(input: &str, options: LookupOptions) -> Result<TorrentInfo> {
    let magnet = magnet_with_fallback_trackers(input, &options.fallback_trackers)?;
    let session = Session::new_with_opts(
        options.session_folder,
        SessionOptions {
            dht: Some(DhtSessionConfig {
                persistence: None,
                ..Default::default()
            }),
            ..Default::default()
        },
    )
    .await
    .context("failed to start the BitTorrent session")?;

    let response = timeout(
        options.timeout,
        session.add_torrent(
            AddTorrent::from_url(magnet),
            Some(AddTorrentOptions {
                list_only: true,
                ..Default::default()
            }),
        ),
    )
    .await
    .with_context(|| {
        format!(
            "timed out after {} seconds; the torrent may have no reachable peers",
            options.timeout.as_secs()
        )
    })?
    .context("failed to retrieve torrent metadata")?;

    let AddTorrentResponse::ListOnly(metadata) = response else {
        bail!("torrent session unexpectedly started a download");
    };

    let files = metadata
        .info
        .iter_file_details()
        .map(|file| TorrentFile {
            path: file.filename.to_string(),
            size: file.len,
        })
        .collect();

    Ok(TorrentInfo {
        name: metadata.info.name().map(|name| name.into_owned()),
        info_hash: metadata.info_hash.as_string(),
        total_size: metadata.info.lengths().total_length(),
        total_pieces: metadata.info.lengths().total_pieces(),
        files,
        discovered_peers: metadata.seen_peers,
        seeders: None,
    })
}

fn magnet_with_fallback_trackers(input: &str, fallback_trackers: &[String]) -> Result<String> {
    let parsed = Magnet::parse(input).context("invalid infohash or magnet URI")?;
    if parsed.as_id20().is_none() {
        bail!("only BitTorrent v1 infohashes are currently supported");
    }

    let mut url = if input.len() == 40 {
        Url::parse(&format!("magnet:?xt=urn:btih:{input}"))?
    } else {
        Url::parse(input).context("invalid magnet URI")?
    };

    let existing_trackers = parsed.trackers;
    {
        let mut query = url.query_pairs_mut();
        for tracker in fallback_trackers {
            if !existing_trackers.contains(tracker) {
                query.append_pair("tr", tracker);
            }
        }
    }

    Ok(url.into())
}

#[cfg(test)]
mod tests {
    use super::*;

    const HASH: &str = "d2474e86c95b19b8bcfdb92bc12c9d44667cfa36";

    fn trackers() -> Vec<String> {
        DEFAULT_TRACKERS
            .iter()
            .map(|tracker| (*tracker).to_owned())
            .collect()
    }

    #[test]
    fn converts_infohash_to_magnet_with_trackers() {
        let magnet = magnet_with_fallback_trackers(HASH, &trackers()).unwrap();
        let parsed = Magnet::parse(&magnet).unwrap();

        assert_eq!(parsed.as_id20().unwrap().as_string(), HASH);
        assert_eq!(parsed.trackers.len(), DEFAULT_TRACKERS.len());
    }

    #[test]
    fn preserves_existing_tracker_without_duplicating_it() {
        let input = format!("magnet:?xt=urn:btih:{HASH}&tr={}", DEFAULT_TRACKERS[0]);
        let magnet = magnet_with_fallback_trackers(&input, &trackers()).unwrap();
        let parsed = Magnet::parse(&magnet).unwrap();

        assert_eq!(parsed.trackers.len(), DEFAULT_TRACKERS.len());
    }

    #[test]
    fn rejects_invalid_hash() {
        let error = magnet_with_fallback_trackers("not-an-infohash", &trackers()).unwrap_err();

        assert!(error.to_string().contains("invalid infohash or magnet URI"));
    }
}
