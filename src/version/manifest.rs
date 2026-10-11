// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1

//! The release update manifest: where it is read from and what it must prove.
//!
//! screenerbot.io decides *which* version is offered and attests the digest of
//! its manifest; the manifest names every artifact of that version. Release
//! assets are read from GitHub's download host, never from its REST API, whose
//! unauthenticated quota (60 requests an hour per address) is exhausted by
//! shared networks and failed updates outright.

use super::types::*;
use super::{
    Error, Result, GITHUB_RELEASE_DOWNLOAD_URL, MAX_UPDATE_BYTES, RELEASE_MIRROR_URL, VERSION,
};
use crate::errors::ErrorClass;
use crate::logger::{self, LogTag};
use std::time::Duration;

/// Cap the manifest read. It is a few kilobytes of JSON; anything larger is a
/// wrong or hostile asset, not a manifest.
const MAX_MANIFEST_BYTES: u64 = 256 * 1024;

/// Which host supplied a manifest whose bytes matched the website attestation.
///
/// Only GitHub's copy is a second party. The screenerbot.io mirror serves the
/// same bytes, but then the website alone vouches for every digest inside it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum ManifestOrigin {
    GitHub,
    Mirror,
}

#[derive(Debug, Clone)]
pub(super) struct AttestedManifest {
    pub manifest: UpdateManifest,
    pub origin: ManifestOrigin,
}

impl AttestedManifest {
    /// The installer digest GitHub's manifest publishes for `filename`, when the
    /// manifest was read from GitHub and names that file.
    pub(super) fn github_installer_digest(&self, filename: &str) -> Option<&ReleaseFile> {
        if self.origin != ManifestOrigin::GitHub {
            return None;
        }
        self.manifest.files.get(filename)
    }
}

fn manifest_filename(version: &str) -> String {
    format!("ScreenerBot-v{version}-update-manifest.json")
}

fn asset_url(directory: &str, version: &str, filename: &str) -> Result<reqwest::Url> {
    super::checker::validate_release_filename(filename, version)?;
    let url = format!("{directory}/{filename}");
    reqwest::Url::parse(&url).map_err(|error| Error::InvalidUpdateUrl {
        url,
        reason: error.to_string(),
    })
}

/// A release asset on GitHub's download host, which redirects to its CDN.
pub(super) fn github_asset_url(version: &str, filename: &str) -> Result<reqwest::Url> {
    asset_url(
        &format!("{GITHUB_RELEASE_DOWNLOAD_URL}/v{version}"),
        version,
        filename,
    )
}

/// The same asset as published under screenerbot.io's static release directory.
fn mirror_asset_url(version: &str, filename: &str) -> Result<reqwest::Url> {
    asset_url(
        &format!("{RELEASE_MIRROR_URL}/{version}"),
        version,
        filename,
    )
}

/// Read the release's update manifest, GitHub first and the screenerbot.io
/// mirror second, and accept only bytes that hash to the website attestation.
///
/// A host that cannot be reached hands over to the next one. A body that does
/// not match the attestation is terminal: GitHub and the website disagreeing
/// about a release is never resolved by asking a third time.
pub(super) async fn fetch_manifest(
    client: &reqwest::Client,
    version: &str,
    website_digest: &str,
) -> Result<AttestedManifest> {
    let filename = manifest_filename(version);
    let sources = [
        (
            ManifestOrigin::GitHub,
            github_asset_url(version, &filename)?,
        ),
        (
            ManifestOrigin::Mirror,
            mirror_asset_url(version, &filename)?,
        ),
    ];
    fetch_manifest_from(client, &sources, version, website_digest, true).await
}

async fn fetch_manifest_from(
    client: &reqwest::Client,
    sources: &[(ManifestOrigin, reqwest::Url)],
    version: &str,
    website_digest: &str,
    check_url_allowlist: bool,
) -> Result<AttestedManifest> {
    let mut last_error = None;
    for (origin, url) in sources {
        let body = super::download::retry_update_operation("Update manifest download", || {
            fetch_manifest_body(client, url.clone(), check_url_allowlist)
        })
        .await;
        match body {
            Ok(body) => {
                if super::download::sha256_bytes(&body) != website_digest {
                    return Err(Error::DigestMismatch {
                        detail: format!(
                            "update manifest from {url} differs from the website attestation"
                        ),
                    });
                }
                let manifest = parse_manifest(&body, version)?;
                return Ok(AttestedManifest {
                    manifest,
                    origin: *origin,
                });
            }
            Err(error) => {
                logger::warning(
                    LogTag::System,
                    &format!("Update manifest unavailable from {url}: {error}"),
                );
                // Report the failure a later attempt could overcome, if any host had one.
                if last_error
                    .as_ref()
                    .is_none_or(|kept| !is_transient_failure(kept))
                {
                    last_error = Some(error);
                }
            }
        }
    }
    Err(last_error.unwrap_or(Error::NoUpdateAvailable))
}

/// Whether a failure says nothing about the release itself, so the decision it
/// feeds should wait for a later attempt instead of settling now. A refused or
/// throttled request (GitHub answers 403 when its quota is spent) is transient;
/// an asset that does not exist is not.
pub(super) fn is_transient_failure(error: &Error) -> bool {
    match error {
        Error::Network(crate::errors::NetworkError::HttpStatus { status, .. }) => {
            !matches!(status, 404 | 410)
        }
        Error::Network(_) => true,
        other => other.is_retryable(),
    }
}

async fn fetch_manifest_body(
    client: &reqwest::Client,
    url: reqwest::Url,
    check_url_allowlist: bool,
) -> Result<Vec<u8>> {
    let endpoint = url.to_string();
    let response = client
        .get(url)
        .header("User-Agent", format!("ScreenerBot/{VERSION}"))
        .timeout(Duration::from_secs(20))
        .send()
        .await
        .map_err(|error| {
            Error::Network(crate::errors::NetworkError::RequestFailed {
                endpoint: endpoint.clone(),
                detail: error.to_string(),
            })
        })?;
    if !response.status().is_success() {
        return Err(Error::Network(crate::errors::NetworkError::http_status(
            response.url().to_string(),
            response.status().as_u16(),
            None,
        )));
    }
    if check_url_allowlist {
        super::download::validate_final_url(response.url())?;
    }
    super::download::read_limited_body(response, MAX_MANIFEST_BYTES, &endpoint).await
}

/// Parse and validate a manifest body against the version it must describe.
pub(super) fn parse_manifest(body: &[u8], version: &str) -> Result<UpdateManifest> {
    let mut manifest: UpdateManifest = serde_json::from_slice(body).map_err(|error| {
        Error::Data(crate::errors::DataError::ParseError {
            data_type: "update manifest".to_owned(),
            error: error.to_string(),
        })
    })?;

    if manifest.schema != 1 {
        return Err(Error::Data(crate::errors::DataError::InvalidFormat {
            expected: "update manifest schema 1".to_owned(),
            received: manifest.schema.to_string(),
        }));
    }
    if manifest.version != version {
        return Err(Error::Data(crate::errors::DataError::InvalidFormat {
            expected: version.to_owned(),
            received: manifest.version,
        }));
    }
    validate_shell_revision(&manifest.shell_revision)?;
    for (platform, artifact) in &manifest.core {
        validate_core_artifact(platform, artifact, version)?;
    }
    for (filename, file) in &manifest.files {
        validate_release_file(filename, file, version)?;
    }
    for artifact in manifest.core.values_mut() {
        artifact.sha256.make_ascii_lowercase();
        artifact.binary_sha256.make_ascii_lowercase();
    }
    for file in manifest.files.values_mut() {
        file.sha256.make_ascii_lowercase();
    }
    Ok(manifest)
}

fn validate_shell_revision(revision: &str) -> Result<()> {
    let valid = (8..=64).contains(&revision.len())
        && revision.bytes().all(|byte| byte.is_ascii_hexdigit())
        && revision.bytes().all(|byte| !byte.is_ascii_uppercase());
    if valid {
        Ok(())
    } else {
        Err(Error::Data(crate::errors::DataError::ValidationError {
            field: "manifest.shellRevision".to_owned(),
            value: revision.to_owned(),
            reason: "must be a lowercase hex digest of 8 to 64 characters".to_owned(),
        }))
    }
}

fn validate_release_file(filename: &str, file: &ReleaseFile, version: &str) -> Result<()> {
    super::checker::validate_release_filename(filename, version)?;
    super::checker::validate_checksum(&file.sha256)?;
    if file.size == 0 || file.size > MAX_UPDATE_BYTES {
        return Err(Error::Data(crate::errors::DataError::ValidationError {
            field: "manifest.files.size".to_owned(),
            value: file.size.to_string(),
            reason: format!("must be between 1 and {MAX_UPDATE_BYTES} bytes"),
        }));
    }
    Ok(())
}

fn validate_core_artifact(platform: &str, artifact: &CoreArtifact, version: &str) -> Result<()> {
    let platform_ok = !platform.is_empty()
        && platform.len() <= 32
        && platform
            .bytes()
            .all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit() || byte == b'-');
    if !platform_ok {
        return Err(Error::Data(crate::errors::DataError::ValidationError {
            field: "manifest.core key".to_owned(),
            value: platform.to_owned(),
            reason: "must be a lowercase platform key".to_owned(),
        }));
    }
    super::checker::validate_checksum(&artifact.sha256)?;
    super::checker::validate_checksum(&artifact.binary_sha256)?;
    super::checker::validate_release_filename(&artifact.filename, version)?;
    if !artifact.filename.ends_with("-core.gz") {
        return Err(Error::Data(crate::errors::DataError::ValidationError {
            field: "manifest.core.filename".to_owned(),
            value: artifact.filename.clone(),
            reason: "a core artifact must be a single gzip-compressed binary".to_owned(),
        }));
    }
    for (field, value) in [
        ("manifest.core.size", artifact.size),
        ("manifest.core.binarySize", artifact.binary_size),
    ] {
        if value == 0 || value > MAX_UPDATE_BYTES {
            return Err(Error::Data(crate::errors::DataError::ValidationError {
                field: field.to_owned(),
                value: value.to_string(),
                reason: format!("must be between 1 and {MAX_UPDATE_BYTES} bytes"),
            }));
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn manifest_json(version: &str, revision: &str) -> Vec<u8> {
        serde_json::to_vec(&serde_json::json!({
            "schema": 1,
            "version": version,
            "shellRevision": revision,
            "core": {
                "macos-arm64": {
                    "filename": format!("ScreenerBot-v{version}-macOS-arm64-core.gz"),
                    "size": 24_000_000,
                    "sha256": "a".repeat(64),
                    "binarySize": 80_000_000,
                    "binarySha256": "b".repeat(64),
                }
            }
        }))
        .unwrap()
    }

    #[test]
    fn accepts_a_complete_manifest() {
        let body = manifest_json("0.2.2", "a1b2c3d4e5f6");
        let manifest = parse_manifest(&body, "0.2.2").unwrap();
        assert_eq!(manifest.shell_revision, "a1b2c3d4e5f6");
        assert_eq!(manifest.core.len(), 1);
        assert_eq!(manifest.core["macos-arm64"].binary_size, 80_000_000);
    }

    #[test]
    fn canonicalizes_valid_uppercase_artifact_digests() {
        let mut value: serde_json::Value =
            serde_json::from_slice(&manifest_json("0.2.2", "a1b2c3d4e5f6")).unwrap();
        value["core"]["macos-arm64"]["sha256"] = serde_json::json!("A".repeat(64));
        value["core"]["macos-arm64"]["binarySha256"] = serde_json::json!("B".repeat(64));
        let body = serde_json::to_vec(&value).unwrap();

        let manifest = parse_manifest(&body, "0.2.2").unwrap();
        let artifact = &manifest.core["macos-arm64"];
        assert_eq!(artifact.sha256, "a".repeat(64));
        assert_eq!(artifact.binary_sha256, "b".repeat(64));
    }

    #[test]
    fn rejects_a_manifest_for_another_version_or_schema() {
        let body = manifest_json("0.2.2", "a1b2c3d4e5f6");
        assert!(parse_manifest(&body, "0.2.3").is_err());

        let wrong_schema = serde_json::to_vec(&serde_json::json!({
            "schema": 2, "version": "0.2.2", "shellRevision": "a1b2c3d4e5f6", "core": {}
        }))
        .unwrap();
        assert!(parse_manifest(&wrong_schema, "0.2.2").is_err());
    }

    #[test]
    fn rejects_unusable_shell_revisions_and_core_entries() {
        assert!(parse_manifest(&manifest_json("0.2.2", "nothex!!"), "0.2.2").is_err());
        assert!(parse_manifest(&manifest_json("0.2.2", "abc"), "0.2.2").is_err());
        assert!(parse_manifest(&manifest_json("0.2.2", "A1B2C3D4"), "0.2.2").is_err());

        let foreign_filename = serde_json::to_vec(&serde_json::json!({
            "schema": 1,
            "version": "0.2.2",
            "shellRevision": "a1b2c3d4e5f6",
            "core": { "macos-arm64": {
                "filename": "../evil.gz", "size": 1, "sha256": "a".repeat(64),
                "binarySize": 1, "binarySha256": "b".repeat(64) } }
        }))
        .unwrap();
        assert!(parse_manifest(&foreign_filename, "0.2.2").is_err());
    }

    #[test]
    fn release_file_digests_are_validated_and_canonicalized() {
        let mut value: serde_json::Value =
            serde_json::from_slice(&manifest_json("0.2.2", "a1b2c3d4e5f6")).unwrap();
        value["files"] = serde_json::json!({
            "ScreenerBot-v0.2.2-macOS-arm64.dmg": { "size": 42, "sha256": "C".repeat(64) }
        });
        let manifest = parse_manifest(&serde_json::to_vec(&value).unwrap(), "0.2.2").unwrap();
        assert_eq!(
            manifest.files["ScreenerBot-v0.2.2-macOS-arm64.dmg"],
            ReleaseFile {
                size: 42,
                sha256: "c".repeat(64)
            }
        );

        for files in [
            serde_json::json!({ "../evil.dmg": { "size": 42, "sha256": "c".repeat(64) } }),
            serde_json::json!({ "ScreenerBot-v0.2.3-macOS-arm64.dmg": { "size": 42, "sha256": "c".repeat(64) } }),
            serde_json::json!({ "ScreenerBot-v0.2.2-macOS-arm64.dmg": { "size": 0, "sha256": "c".repeat(64) } }),
            serde_json::json!({ "ScreenerBot-v0.2.2-macOS-arm64.dmg": { "size": 42, "sha256": "bad" } }),
        ] {
            value["files"] = files;
            assert!(parse_manifest(&serde_json::to_vec(&value).unwrap(), "0.2.2").is_err());
        }
    }

    #[test]
    fn a_manifest_without_release_files_still_parses() {
        // Releases before the manifest listed installers keep a usable core map.
        let manifest = parse_manifest(&manifest_json("0.2.2", "a1b2c3d4e5f6"), "0.2.2").unwrap();
        assert!(manifest.files.is_empty());
    }

    #[test]
    fn only_a_manifest_read_from_github_attests_an_installer() {
        let mut manifest =
            parse_manifest(&manifest_json("0.2.2", "a1b2c3d4e5f6"), "0.2.2").unwrap();
        let filename = "ScreenerBot-v0.2.2-macOS-arm64.dmg";
        manifest.files.insert(
            filename.to_owned(),
            ReleaseFile {
                size: 42,
                sha256: "c".repeat(64),
            },
        );
        let github = AttestedManifest {
            manifest: manifest.clone(),
            origin: ManifestOrigin::GitHub,
        };
        let mirror = AttestedManifest {
            manifest,
            origin: ManifestOrigin::Mirror,
        };
        assert_eq!(github.github_installer_digest(filename).unwrap().size, 42);
        assert!(github.github_installer_digest("other.dmg").is_none());
        assert!(mirror.github_installer_digest(filename).is_none());
    }

    #[test]
    fn release_assets_come_from_download_hosts_never_the_rest_api() {
        let filename = "ScreenerBot-v0.2.2-update-manifest.json";
        let github = github_asset_url("0.2.2", filename).unwrap();
        assert_eq!(
            github.as_str(),
            "https://github.com/farfary/ScreenerBot/releases/download/v0.2.2/ScreenerBot-v0.2.2-update-manifest.json"
        );
        let mirror = mirror_asset_url("0.2.2", filename).unwrap();
        assert_eq!(
            mirror.as_str(),
            "https://screenerbot.io/releases/0.2.2/ScreenerBot-v0.2.2-update-manifest.json"
        );
        for url in [&github, &mirror] {
            assert!(super::super::download::is_allowed_update_url(url));
        }
        assert!(github_asset_url("0.2.2", "../ScreenerBot-v0.2.2-x.dmg").is_err());
        assert!(github_asset_url("0.2.2", "ScreenerBot-v0.2.3-x.dmg").is_err());
        let api = reqwest::Url::parse("https://api.github.com/repos/farfary/ScreenerBot").unwrap();
        assert!(!super::super::download::is_allowed_update_url(&api));
    }

    #[test]
    fn refused_or_throttled_hosts_are_transient_and_missing_assets_are_not() {
        let status = |status| {
            Error::Network(crate::errors::NetworkError::http_status(
                "https://github.com/x".to_owned(),
                status,
                None,
            ))
        };
        assert!(is_transient_failure(&status(403)));
        assert!(is_transient_failure(&status(429)));
        assert!(is_transient_failure(&status(502)));
        assert!(!is_transient_failure(&status(404)));
        assert!(!is_transient_failure(&Error::DigestMismatch {
            detail: String::new()
        }));
        assert!(is_transient_failure(&Error::Network(
            crate::errors::NetworkError::RequestFailed {
                endpoint: "https://github.com/x".to_owned(),
                detail: "connection reset".to_owned(),
            }
        )));
    }

    /// Serve one canned HTTP response per connection, in order.
    async fn serve(responses: Vec<(u16, Vec<u8>)>) -> reqwest::Url {
        use tokio::io::{AsyncReadExt, AsyncWriteExt};
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let address = listener.local_addr().unwrap();
        tokio::spawn(async move {
            for (status, body) in responses {
                let (mut socket, _) = listener.accept().await.unwrap();
                let mut request = [0_u8; 2048];
                let _ = socket.read(&mut request).await.unwrap();
                let head = format!(
                    "HTTP/1.1 {status} X\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
                    body.len()
                );
                socket.write_all(head.as_bytes()).await.unwrap();
                socket.write_all(&body).await.unwrap();
            }
        });
        format!("http://{address}/manifest.json").parse().unwrap()
    }

    #[tokio::test]
    async fn a_refused_github_read_falls_back_to_the_mirror() {
        let body = manifest_json("0.2.2", "a1b2c3d4e5f6");
        let digest = super::super::download::sha256_bytes(&body);
        let github = serve(vec![(403, Vec::new())]).await;
        let mirror = serve(vec![(200, body)]).await;
        let attested = fetch_manifest_from(
            &reqwest::Client::new(),
            &[
                (ManifestOrigin::GitHub, github),
                (ManifestOrigin::Mirror, mirror),
            ],
            "0.2.2",
            &digest,
            false,
        )
        .await
        .unwrap();
        assert_eq!(attested.origin, ManifestOrigin::Mirror);
        assert_eq!(attested.manifest.shell_revision, "a1b2c3d4e5f6");
    }

    #[tokio::test]
    async fn a_manifest_that_contradicts_the_website_stops_the_update() {
        let body = manifest_json("0.2.2", "a1b2c3d4e5f6");
        let github = serve(vec![(200, body.clone())]).await;
        let mirror = serve(vec![(200, body)]).await;
        let error = fetch_manifest_from(
            &reqwest::Client::new(),
            &[
                (ManifestOrigin::GitHub, github),
                (ManifestOrigin::Mirror, mirror),
            ],
            "0.2.2",
            &"d".repeat(64),
            false,
        )
        .await
        .unwrap_err();
        assert!(matches!(error, Error::DigestMismatch { .. }));
    }

    #[tokio::test]
    async fn every_host_failing_reports_the_failure_worth_retrying() {
        let github = serve(vec![(403, Vec::new())]).await;
        let mirror = serve(vec![(404, Vec::new())]).await;
        let error = fetch_manifest_from(
            &reqwest::Client::new(),
            &[
                (ManifestOrigin::GitHub, github),
                (ManifestOrigin::Mirror, mirror),
            ],
            "0.2.2",
            &"d".repeat(64),
            false,
        )
        .await
        .unwrap_err();
        assert!(is_transient_failure(&error));
    }
}
