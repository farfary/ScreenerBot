fn main() {
    // Always compile Tauri resources (GUI is always available, use --gui flag at runtime)
    tauri_build::build()
}
