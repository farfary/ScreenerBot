//! The GitHub release record: asset digests and the per-release update manifest.
//!
//! screenerbot.io decides *which* version is offered; this module reads what
//! that version actually consists of. Both halves have to agree before a byte is
//! written to disk, so a compromise of either one alone cannot ship a binary.

use super::types::*;
use super::{Error, Result, GITHUB_RELEASES_API_URL, MAX_UPDATE_BYTES, VERSION};
use serde::Deserialize;
use std::time::Duration;

/// Cap the manifest read. It is a few hundred bytes of JSON; anything larger is
/// a wrong or hostile asset, not a manifest.
const MAX_MANIFEST_BYTES: u64 = 256 * 1024;
const MANIFEST_SUFFIX: &str = "-update-manifest.json";

#[derive(Debug, Clone, Deserialize)]
pub(super) struct GithubRelease {
    #[serde(default)]
    pub assets: Vec<GithubAsset>,
}

#[derive(Debug, Clone, Deserialize)]
pub(super) struct GithubAsset {
    pub name: String,
    pub digest: Option<String>,
    pub size: u64,
    #[serde(default)]
    pub browser_download_url: String,
}

impl GithubRelease {
    pub(super) fn asset(&self, name: &str) -> Option<&GithubAsset> {
        self.assets.iter().find(|asset| asset.name == name)
    }

    /// The release's update manifest asset, located by suffix so the version
    /// prefix never has to be reconstructed.
    pub(super) fn manifest_asset(&self) -> Option<&GithubAsset> {
        self.assets
            .iter()
            .find(|asset| asset.name.ends_with(MANIFEST_SUFFIX))
    }
}

impl GithubAsset {
    /// The SHA-256 GitHub computed for this asset, lowercased, when it reports one.
    pub(super) fn sha256(&self) -> Option<String> {
        self.digest
            .as_deref()
            .and_then(|digest| digest.strip_prefix("sha256:"))
            .map(str::to_ascii_lowercase)
    }
}

/// Fetch the release record for a version from the GitHub API.
pub(super) async fn fetch_release(
    client: &reqwest::Client,
    github_api: &str,
    version: &str,
) -> Result<GithubRelease> {
    let url = format!("{github_api}/releases/tags/v{version}");
    let response = client
        .get(&url)
        .header("Accept", "application/vnd.github+json")
        .header("X-GitHub-Api-Version", "2022-11-28")
        .header("User-Agent", format!("ScreenerBot/{VERSION}"))
        .timeout(Duration::from_secs(15))
        .send()
        .await
        .map_err(|error| {
            Error::Network(crate::errors::NetworkError::RequestFailed {
                endpoint: url.clone(),
                detail: error.to_string(),
            })
        })?;
    if !response.status().is_success() {
        return Err(Error::Network(crate::errors::NetworkError::HttpStatus {
            endpoint: url,
            status: response.status().as_u16(),
            body: None,
        }));
    }
    response.json().await.map_err(|error| {
        Error::Data(crate::errors::DataError::ParseError {
            data_type: "GitHub release metadata".to_owned(),
            error: error.to_string(),
        })
    })
}

pub(super) async fn fetch_release_for(
    client: &reqwest::Client,
    version: &str,
) -> Result<GithubRelease> {
    fetch_release(client, GITHUB_RELEASES_API_URL, version).await
}

/// Verify that an installer asset the website advertised is byte-identical to
/// the asset GitHub published for the same release.
pub(super) fn verify_release_asset(
    release: &GithubRelease,
    filename: &str,
    file_size: u64,
    checksum: &str,
) -> Result<()> {
    let asset = release
        .asset(filename)
        .ok_or_else(|| Error::DigestMismatch {
            detail: format!("GitHub release does not contain {filename}"),
        })?;
    if asset.size != file_size {
        return Err(Error::DigestMismatch {
            detail: format!(
                "GitHub asset size {} differs from website size {file_size}",
                asset.size
            ),
        });
    }
    if asset.sha256().as_deref() != Some(checksum.to_ascii_lowercase().as_str()) {
        return Err(Error::DigestMismatch {
            detail: "GitHub asset checksum differs from website checksum".to_owned(),
        });
    }
    Ok(())
}

/// Download the release's update manifest and prove it is the asset GitHub
/// published, then validate every field the updater will act on.
pub(super) async fn fetch_manifest(
    client: &reqwest::Client,
    release: &GithubRelease,
    version: &str,
) -> Result<UpdateManifest> {
    let asset = release
        .manifest_asset()
        .ok_or_else(|| Error::DigestMismatch {
            detail: format!("release v{version} publishes no update manifest"),
        })?;
    if asset.size == 0 || asset.size > MAX_MANIFEST_BYTES {
        return Err(Error::Data(crate::errors::DataError::ValidationError {
            field: "update manifest size".to_owned(),
            value: asset.size.to_string(),
            reason: format!("must be between 1 and {MAX_MANIFEST_BYTES} bytes"),
        }));
    }
    let expected_digest = asset.sha256().ok_or_else(|| Error::DigestMismatch {
        detail: "GitHub reports no digest for the update manifest".to_owned(),
    })?;

    let url = super::download::resolve_download_url(&asset.browser_download_url)?;
    let response = client
        .get(url)
        .header("User-Agent", format!("ScreenerBot/{VERSION}"))
        .timeout(Duration::from_secs(20))
        .send()
        .await
        .map_err(|error| {
            Error::Network(crate::errors::NetworkError::RequestFailed {
                endpoint: asset.browser_download_url.clone(),
                detail: error.to_string(),
            })
        })?;
    if !response.status().is_success() {
        return Err(Error::Network(crate::errors::NetworkError::HttpStatus {
            endpoint: response.url().to_string(),
            status: response.status().as_u16(),
            body: None,
        }));
    }
    super::download::validate_final_url(response.url())?;

    let body = response.bytes().await.map_err(|error| {
        Error::Network(crate::errors::NetworkError::RequestFailed {
            endpoint: asset.browser_download_url.clone(),
            detail: error.to_string(),
        })
    })?;
    if body.len() as u64 != asset.size {
        return Err(Error::DownloadSizeMismatch {
            expected: asset.size,
            actual: body.len() as u64,
        });
    }
    if super::download::sha256_bytes(&body) != expected_digest {
        return Err(Error::DigestMismatch {
            detail: "update manifest does not match its GitHub digest".to_owned(),
        });
    }

    parse_manifest(&body, version)
}

/// Parse and validate a manifest body against the version it must describe.
pub(super) fn parse_manifest(body: &[u8], version: &str) -> Result<UpdateManifest> {
    let manifest: UpdateManifest = serde_json::from_slice(body).map_err(|error| {
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
    fn release_assets_must_match_name_size_and_digest() {
        let release = GithubRelease {
            assets: vec![GithubAsset {
                name: "ScreenerBot-v0.2.2-macOS-arm64.dmg".to_owned(),
                digest: Some(format!("sha256:{}", "a".repeat(64))),
                size: 42,
                browser_download_url: String::new(),
            }],
        };
        assert!(verify_release_asset(
            &release,
            "ScreenerBot-v0.2.2-macOS-arm64.dmg",
            42,
            &"a".repeat(64)
        )
        .is_ok());
        assert!(verify_release_asset(
            &release,
            "ScreenerBot-v0.2.2-macOS-arm64.dmg",
            43,
            &"a".repeat(64)
        )
        .is_err());
        assert!(verify_release_asset(
            &release,
            "ScreenerBot-v0.2.2-macOS-arm64.dmg",
            42,
            &"b".repeat(64)
        )
        .is_err());
        assert!(verify_release_asset(&release, "other.dmg", 42, &"a".repeat(64)).is_err());
    }

    #[test]
    fn manifest_asset_is_found_by_suffix() {
        let release = GithubRelease {
            assets: vec![
                GithubAsset {
                    name: "ScreenerBot-v0.2.2-macOS-arm64.dmg".to_owned(),
                    digest: None,
                    size: 1,
                    browser_download_url: String::new(),
                },
                GithubAsset {
                    name: "ScreenerBot-v0.2.2-update-manifest.json".to_owned(),
                    digest: Some(format!("sha256:{}", "c".repeat(64))),
                    size: 300,
                    browser_download_url: String::new(),
                },
            ],
        };
        let asset = release.manifest_asset().unwrap();
        assert_eq!(asset.sha256().unwrap(), "c".repeat(64));
    }
}
