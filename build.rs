fn main() {
  // macOS 26 Tahoe icon support: Track Assets.car for rebuild triggers
  #[cfg(target_os = "macos")]
  {
    let assets_car = std::path::Path::new("icons/macos-tahoe/Assets.car");
    if assets_car.exists() {
      println!("cargo:rerun-if-changed=icons/macos-tahoe/Assets.car");
      // println!(
      //   "cargo:warning= macOS 26 Tahoe Assets.car found - will be included in bundle"
      // );

      // Tell Tauri to copy Assets.car to the bundle Resources folder
      // This will be picked up by macOS 26+ automatically
      println!("cargo:rustc-env=TAURI_MACOS_ASSETS_CAR=icons/macos-tahoe/Assets.car");
    } else {
      println!("cargo:warning= macOS 26 Tahoe Assets.car not found");
      println!("cargo:warning= Icons may not display correctly on macOS 26 Tahoe");
      println!("cargo:warning= Steps:");
      println!("cargo:warning= 1. Create screenerbot.icon using Icon Composer");
      println!("cargo:warning= 2. Run: ./generate-tahoe-icons.sh");
      println!("cargo:warning= 3. Rebuild: cargo tauri build");
      println!("cargo:warning= See: icons/TAHOE_ICON_GUIDE.md");
    }
  }

  // Always compile Tauri resources (GUI is always available, use --gui flag at runtime)
  tauri_build::build()
}
