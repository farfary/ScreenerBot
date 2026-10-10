// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1

//! Metadata-only measurement of the app's directories: family entries by `symlink_metadata`, stores by their file header, and the free space of the disk that holds them.

use std::collections::BTreeSet;
use std::fs;
use std::io::Read;
use std::path::{Path, PathBuf};
use std::time::SystemTime;

use crate::database::catalog::STORES;
use crate::errors::StorageError;

use super::families::{classify, FAMILIES};
use super::types::{Classification, Disk, FileEntry, StoreUsage};

/// Bytes of the SQLite file header this module reads.
const HEADER_BYTES: usize = 100;

/// The fields of a SQLite file header housekeeping plans from.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct StoreHeader {
    pub page_size: u64,
    pub page_count: u64,
    pub freelist_pages: u64,
    pub incremental: bool,
}

fn be_u32(header: &[u8], offset: usize) -> u64 {
    u64::from(u32::from_be_bytes([
        header[offset],
        header[offset + 1],
        header[offset + 2],
        header[offset + 3],
    ]))
}

/// Parse the SQLite header at the start of a store file. `None` when the bytes are
/// not a SQLite header.
pub fn parse_header(header: &[u8]) -> Option<StoreHeader> {
    if header.len() < HEADER_BYTES || !header.starts_with(b"SQLite format 3\0") {
        return None;
    }
    let page_size = match u16::from_be_bytes([header[16], header[17]]) {
        1 => 65_536,
        size => u64::from(size),
    };
    Some(StoreHeader {
        page_size,
        page_count: be_u32(header, 28),
        freelist_pages: be_u32(header, 36),
        // Auto-vacuum is on when the largest root b-tree page is set, and incremental
        // when the incremental-vacuum flag is set as well.
        incremental: be_u32(header, 52) != 0 && be_u32(header, 64) != 0,
    })
}

/// Bytes on disk under `path`, never following a symbolic link.
fn bytes_under(path: &Path, metadata: &fs::Metadata) -> u64 {
    if !metadata.is_dir() {
        return metadata.len();
    }
    let Ok(entries) = fs::read_dir(path) else {
        return 0;
    };
    entries
        .flatten()
        .filter_map(|entry| {
            let path = entry.path();
            fs::symlink_metadata(&path)
                .ok()
                .map(|metadata| bytes_under(&path, &metadata))
        })
        .sum()
}

fn is_empty(path: &Path, metadata: &fs::Metadata) -> bool {
    if metadata.is_dir() {
        fs::read_dir(path).is_ok_and(|mut entries| entries.next().is_none())
    } else {
        metadata.len() == 0
    }
}

/// Measure one path as an entry of the family that owns it.
pub fn measure_entry(path: &Path) -> Result<Option<FileEntry>, StorageError> {
    let Classification::Family(family) = classify(path) else {
        return Ok(None);
    };
    let metadata = match fs::symlink_metadata(path) {
        Ok(metadata) => metadata,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(error) => {
            return Err(StorageError::Measure {
                path: path.display().to_string(),
                message: error.to_string(),
            })
        }
    };
    let is_symlink = metadata.file_type().is_symlink();
    Ok(Some(FileEntry {
        path: path.to_path_buf(),
        family: family.id,
        bytes: if is_symlink {
            0
        } else {
            bytes_under(path, &metadata)
        },
        modified: metadata.modified().unwrap_or(SystemTime::UNIX_EPOCH),
        is_dir: metadata.is_dir(),
        is_symlink,
        empty: !is_symlink && is_empty(path, &metadata),
    }))
}

/// Every entry of every family on disk, each directory listed once.
pub fn measure_entries() -> Result<Vec<FileEntry>, StorageError> {
    let directories: BTreeSet<PathBuf> = FAMILIES
        .iter()
        .flat_map(|family| family.directories())
        .collect();
    let mut entries = Vec::new();
    for directory in directories {
        let listing = match fs::read_dir(&directory) {
            Ok(listing) => listing,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => continue,
            Err(error) => {
                return Err(StorageError::Measure {
                    path: directory.display().to_string(),
                    message: error.to_string(),
                })
            }
        };
        for item in listing {
            let item = item.map_err(|error| StorageError::Measure {
                path: directory.display().to_string(),
                message: error.to_string(),
            })?;
            if let Some(entry) = measure_entry(&item.path())? {
                entries.push(entry);
            }
        }
    }
    Ok(entries)
}

fn file_len(path: &Path) -> u64 {
    fs::symlink_metadata(path).map_or(0, |metadata| metadata.len())
}

fn sidecar(path: &Path, suffix: &str) -> PathBuf {
    let mut name = path.as_os_str().to_owned();
    name.push(suffix);
    PathBuf::from(name)
}

/// Every catalog store file on disk, described by its header and sidecar sizes.
/// Reading the header never opens the store, so a missing file stays missing.
pub fn measure_stores() -> Vec<StoreUsage> {
    let mut stores = Vec::new();
    for spec in STORES {
        for path in spec.paths() {
            let Ok(mut file) = fs::File::open(&path) else {
                continue;
            };
            let mut header = [0u8; HEADER_BYTES];
            let header = match file.read_exact(&mut header) {
                Ok(()) => parse_header(&header),
                Err(_) => None,
            };
            let wal_bytes = file_len(&sidecar(&path, "-wal"));
            let bytes = file_len(&path) + wal_bytes + file_len(&sidecar(&path, "-shm"));
            let header = header.unwrap_or(StoreHeader {
                page_size: 0,
                page_count: 0,
                freelist_pages: 0,
                incremental: true,
            });
            stores.push(StoreUsage {
                spec,
                path,
                bytes,
                wal_bytes,
                page_size: header.page_size,
                page_count: header.page_count,
                freelist_pages: header.freelist_pages,
                incremental: header.incremental,
            });
        }
    }
    stores
}

/// Free and total space of the disk holding `directory`: the mounted disk whose mount
/// point is the longest prefix of the directory's canonical path.
pub fn measure_disk(directory: &Path) -> Option<Disk> {
    let directory = fs::canonicalize(directory).ok()?;
    let disks = sysinfo::Disks::new_with_refreshed_list();
    disks
        .list()
        .iter()
        .filter(|disk| directory.starts_with(disk.mount_point()))
        .max_by_key(|disk| disk.mount_point().as_os_str().len())
        .map(|disk| Disk {
            free: disk.available_space(),
            total: disk.total_space(),
        })
}
