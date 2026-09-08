//! The published release history shown inside the app.
//!
//! screenerbot.io is the only source: release notes live in the Website
//! database, are edited there, and survive installer cleanup. GitHub carries
//! artifacts, not the editorial record, so it is never consulted here.

use std::time::{Duration, Instant};

use serde::Deserialize;
use tokio::sync::{OnceCell, RwLock};

use super::types::{ApiResponse, ReleaseSummary};
use super::{Error, Result, UPDATE_SERVER_URL};
use crate::logger::{self, LogTag};

/// Releases requested in one page. The Website caps this at 100; the app only
/// ever shows a scrollable list, so a shorter page keeps the response small.
const HISTORY_LIMIT: u32 = 50;
const MAX_HISTORY_BYTES: u64 = 512 * 1024;
const HISTORY_TIMEOUT: Duration = Duration::from_secs(10);
/// Notes change only when the owner edits them, so a long window is right; the
/// dashboard re-reads on demand and any successful update check outdates it.
const HISTORY_TTL: Duration = Duration::from_secs(30 * 60);

struct CachedHistory {
    releases: Vec<ReleaseSummary>,
    fetched_at: Instant,
}

static HISTORY_CACHE: OnceCell<RwLock<Option<CachedHistory>>> = OnceCell::const_new();

async fn cache() -> &'static RwLock<Option<CachedHistory>> {
    HISTORY_CACHE
        .get_or_init(|| async { RwLock::new(None) })
        .await
}

/// Every published release, newest first.
///
/// A fetch failure falls back to the last good answer rather than emptying the
/// list — an installation that has read the history once keeps showing it while
/// the machine is offline.
pub async fn release_history() -> Result<Vec<ReleaseSummary>> {
    if let Some(fresh) = cached(HISTORY_TTL).await {
        return Ok(fresh);
    }

    if crate::connectivity::is_network_offline() {
        return cached_or(Error::Network(crate::errors::NetworkError::RequestFailed {
            endpoint: format!("{UPDATE_SERVER_URL}/releases/history"),
            detail: "the network is offline".to_owned(),
        }))
        .await;
    }

    match fetch_history().await {
        Ok(releases) => {
            *cache().await.write().await = Some(CachedHistory {
                releases: releases.clone(),
                fetched_at: Instant::now(),
            });
            Ok(releases)
        }
        Err(error) => {
            logger::warning(
                LogTag::Webserver,
                &format!("Release history could not be refreshed: {error}"),
            );
            cached_or(error).await
        }
    }
}

async fn cached(max_age: Duration) -> Option<Vec<ReleaseSummary>> {
    let guard = cache().await.read().await;
    guard
        .as_ref()
        .filter(|entry| entry.fetched_at.elapsed() < max_age)
        .map(|entry| entry.releases.clone())
}

async fn cached_or(error: Error) -> Result<Vec<ReleaseSummary>> {
    match cached(Duration::MAX).await {
        Some(releases) => Ok(releases),
        None => Err(error),
    }
}

async fn fetch_history() -> Result<Vec<ReleaseSummary>> {
    let url =
        format!("{UPDATE_SERVER_URL}/releases/history?limit={HISTORY_LIMIT}&includeBuilds=false");
    let response = crate::net::client()
        .get(&url)
        .header(
            "User-Agent",
            format!("ScreenerBot/{version}", version = super::VERSION),
        )
        .timeout(HISTORY_TIMEOUT)
        .send()
        .await
        .map_err(|error| {
            Error::Network(crate::errors::NetworkError::RequestFailed {
                endpoint: url.clone(),
                detail: error.to_string(),
            })
        })?;

    if !response.status().is_success() {
        return Err(Error::UpdateCheckFailed {
            status: response.status().as_u16(),
        });
    }

    let body = super::download::read_limited_body(response, MAX_HISTORY_BYTES, &url).await?;
    parse_history(&body)
}

/// Release history as published by the Website. Build metadata and download
/// counts are deliberately not modelled: the app shows notes, not downloads.
#[derive(Debug, Clone, Deserialize)]
struct ReleaseHistoryData {
    releases: Vec<ReleaseHistoryItem>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
struct ReleaseHistoryItem {
    version: String,
    release_notes: Option<String>,
    published_at: Option<String>,
}

fn parse_history(body: &[u8]) -> Result<Vec<ReleaseSummary>> {
    let response: ApiResponse<ReleaseHistoryData> =
        serde_json::from_slice(body).map_err(|error| {
            Error::Data(crate::errors::DataError::ParseError {
                data_type: "release history".to_owned(),
                error: error.to_string(),
            })
        })?;

    if !response.success {
        return Err(Error::Data(crate::errors::DataError::InvalidFormat {
            expected: "successful release history response".to_owned(),
            received: response
                .error
                .unwrap_or_else(|| "an unspecified server error".to_owned()),
        }));
    }

    let data = response.data.ok_or_else(|| {
        Error::Data(crate::errors::DataError::InvalidFormat {
            expected: "release history data".to_owned(),
            received: "no data".to_owned(),
        })
    })?;

    Ok(data
        .releases
        .into_iter()
        // A release with no notes has nothing to show, and a nameless one could
        // only render as an empty version heading.
        .filter(|release| {
            !release.version.trim().is_empty()
                && release
                    .release_notes
                    .as_deref()
                    .is_some_and(|notes| !notes.trim().is_empty())
        })
        .map(|release| ReleaseSummary {
            version: release.version,
            release_notes: release.release_notes,
            release_date: release.published_at.unwrap_or_default(),
        })
        .collect())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn keeps_only_releases_that_can_be_shown() {
        let body = serde_json::to_vec(&serde_json::json!({
            "success": true,
            "data": {
                "releases": [
                    {
                        "version": "0.2.5",
                        "releaseNotes": "## What's New in v0.2.5",
                        "publishedAt": "2026-09-01T00:00:00Z",
                        "isLatest": true,
                        "totalDownloads": 12
                    },
                    { "version": "0.2.4", "releaseNotes": "   ", "publishedAt": null },
                    { "version": "  ", "releaseNotes": "notes", "publishedAt": null }
                ],
                "total": 3
            }
        }))
        .unwrap();

        let releases = parse_history(&body).unwrap();
        assert_eq!(releases.len(), 1);
        assert_eq!(releases[0].version, "0.2.5");
        assert_eq!(releases[0].release_date, "2026-09-01T00:00:00Z");
    }

    #[test]
    fn rejects_an_unsuccessful_or_shapeless_response() {
        let failed =
            serde_json::to_vec(&serde_json::json!({ "success": false, "error": "boom" })).unwrap();
        assert!(parse_history(&failed).is_err());

        let shapeless = serde_json::to_vec(&serde_json::json!({ "success": true })).unwrap();
        assert!(parse_history(&shapeless).is_err());
    }
}
