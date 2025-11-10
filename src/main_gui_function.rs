/// Run bot in GUI mode with Tauri window
async fn run_gui_mode() -> Result<(), String> {
    use std::sync::Arc;
    use std::time::Duration;
    use tauri::Manager;
    use tokio::sync::Mutex;

    logger::info(LogTag::System, "🖥️  Initializing Tauri desktop application");

    // State to hold server readiness
    struct ServerState {
        server_ready: Arc<Mutex<bool>>,
    }

    #[tauri::command]
    async fn is_server_ready(state: tauri::State<'_, ServerState>) -> Result<bool, String> {
        let ready = *state.server_ready.lock().await;
        Ok(ready)
    }

    // Create shared state for server readiness
    let server_ready = Arc::new(Mutex::new(false));
    let server_ready_clone = server_ready.clone();

    // Start the ScreenerBot backend in a background task
    tokio::spawn(async move {
        logger::info(
            LogTag::System,
            "Starting ScreenerBot backend services...",
        );

        // Wait a moment to ensure Tauri window is created first
        tokio::time::sleep(Duration::from_millis(500)).await;

        // Start the full ScreenerBot system (includes webserver on :8080)
        match screenerbot::run::run_bot().await {
            Ok(_) => {
                logger::info(
                    LogTag::System,
                    "✅ ScreenerBot backend started successfully",
                );
                *server_ready_clone.lock().await = true;
            }
            Err(e) => {
                logger::error(
                    LogTag::System,
                    &format!("❌ Failed to start ScreenerBot backend: {}", e),
                );
            }
        }
    });

    // Build and run Tauri application
    // The loading.html will automatically poll localhost:8080 and navigate when ready
    tauri::Builder::default()
        .setup(|app| {
            let _window = app.get_webview_window("main").unwrap();
            
            logger::info(
                LogTag::System,
                "✅ Tauri window created - loading.html will poll for server readiness",
            );

            Ok(())
        })
        .manage(ServerState { server_ready })
        .invoke_handler(tauri::generate_handler![is_server_ready])
        .run(tauri::generate_context!())
        .map_err(|e| format!("Tauri application error: {}", e))?;

    Ok(())
}
