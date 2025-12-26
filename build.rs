//! Build script for ScreenerBot
//!
//! Generates build-time environment variables for cache busting.

fn main() {
    // Per-build asset version for cache busting of embedded HTML/CSS/JS
    let build_epoch = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    println!("cargo:rustc-env=ASSET_VERSION_TS={build_epoch}");

    // Track template files for rebuild triggers
    println!("cargo:rerun-if-changed=src/webserver/templates/");
}
