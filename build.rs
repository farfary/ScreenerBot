//! Build script for ScreenerBot
//!
//! Generates build-time environment variables for cache busting.
//! Watches all template files so the asset version timestamp changes
//! whenever any HTML/CSS/JS template is modified.

fn main() {
    // Per-build asset version for cache busting of embedded HTML/CSS/JS
    let build_epoch = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    println!("cargo:rustc-env=ASSET_VERSION_TS={build_epoch}");

    // Recursively watch all template files for rebuild triggers.
    // cargo:rerun-if-changed on a directory only watches the listing (add/remove),
    // not content changes inside files. We must list each file individually.
    watch_dir_recursive("src/webserver/templates");
}

fn watch_dir_recursive(dir: &str) {
    println!("cargo:rerun-if-changed={dir}");
    let Ok(entries) = std::fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        let path_str = path.display().to_string();
        if path.is_dir() {
            watch_dir_recursive(&path_str);
        } else {
            println!("cargo:rerun-if-changed={path_str}");
        }
    }
}
