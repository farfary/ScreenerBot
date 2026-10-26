// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1

//! Tests for the crash-safe whole-file writers.

use super::*;

fn has_temp_file(directory: &Path) -> bool {
    std::fs::read_dir(directory).unwrap().any(|entry| {
        entry
            .unwrap()
            .file_name()
            .to_string_lossy()
            .ends_with(".tmp")
    })
}

#[cfg(unix)]
fn mode_of(path: &Path) -> u32 {
    use std::os::unix::fs::PermissionsExt;
    std::fs::metadata(path).unwrap().permissions().mode() & 0o777
}

#[test]
fn replace_writes_and_overwrites_without_leaving_a_temp() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("state.json");

    replace(&path, b"first", 0o600).unwrap();
    replace(&path, b"second", 0o600).unwrap();

    assert_eq!(std::fs::read(&path).unwrap(), b"second");
    assert!(!has_temp_file(directory.path()));
}

#[cfg(unix)]
#[test]
fn replace_applies_the_exact_mode() {
    use std::os::unix::fs::PermissionsExt;
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("state.json");
    std::fs::write(&path, b"wide").unwrap();
    std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o644)).unwrap();

    replace(&path, b"narrow", 0o600).unwrap();

    assert_eq!(mode_of(&path), 0o600);
}

#[cfg(unix)]
#[test]
fn a_failed_replace_removes_its_temp() {
    let directory = tempfile::tempdir().unwrap();
    let clash = directory.path().join("state.json");
    std::fs::create_dir(&clash).unwrap();

    assert!(replace(&clash, b"bytes", 0o600).is_err());
    assert!(
        !has_temp_file(directory.path()),
        "temp file leaked on error path"
    );
}

#[test]
fn write_new_creates_a_missing_file() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("marker.json");

    write_new(&path, b"marker", 0o600).unwrap();

    assert_eq!(std::fs::read(&path).unwrap(), b"marker");
    assert!(!has_temp_file(directory.path()));
    #[cfg(unix)]
    assert_eq!(mode_of(&path), 0o600);
}

#[test]
fn write_new_refuses_an_existing_file_and_leaves_it_intact() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("marker.json");
    std::fs::write(&path, b"PRECIOUS").unwrap();

    let error = write_new(&path, b"other", 0o600).unwrap_err();

    assert_eq!(error.kind(), io::ErrorKind::AlreadyExists);
    assert_eq!(std::fs::read(&path).unwrap(), b"PRECIOUS");
    assert!(!has_temp_file(directory.path()));
}

#[test]
fn an_existing_temp_candidate_is_never_opened_or_truncated() {
    let directory = tempfile::tempdir().unwrap();
    let destination = directory.path().join("state.json");
    let victim = sibling_temp_path(&destination, 0);
    std::fs::write(&victim, b"PRECIOUS").unwrap();

    let mut options = OpenOptions::new();
    options.write(true).create_new(true);
    let error = options.open(&victim).unwrap_err();

    assert_eq!(error.kind(), io::ErrorKind::AlreadyExists);
    assert_eq!(std::fs::read(&victim).unwrap(), b"PRECIOUS");
}

#[test]
fn concurrent_replaces_never_share_a_temp() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("state.json");

    let handles: Vec<_> = (0..8)
        .map(|index| {
            let path = path.clone();
            std::thread::spawn(move || replace(&path, format!("writer {index}").as_bytes(), 0o600))
        })
        .collect();
    for handle in handles {
        handle
            .join()
            .unwrap()
            .expect("every racing replace succeeds");
    }

    assert!(std::fs::read_to_string(&path)
        .unwrap()
        .starts_with("writer "));
    assert!(!has_temp_file(directory.path()));
}

#[test]
fn remove_deletes_a_file_and_accepts_one_already_gone() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("marker.json");
    std::fs::write(&path, b"marker").unwrap();

    remove(&path).unwrap();
    assert!(!path.exists());
    remove(&path).unwrap();
}
