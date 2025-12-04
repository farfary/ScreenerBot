// Theme Management
(async function () {
  const html = document.documentElement;
  const themeToggle = document.getElementById("themeToggle");
  const themeIcon = document.getElementById("themeIcon");
  const themeText = document.getElementById("themeText");

  // Check if running in Tauri (v2 uses __TAURI_INTERNALS__)
  const isTauri =
    window.__TAURI__ !== undefined ||
    window.__TAURI_INTERNALS__ !== undefined;
  let tauriWindow = null;
  let platform = null;

  console.log("[Theme] Initializing theme system...");
  console.log("[Theme] Running in Tauri:", isTauri);
  console.log("[Theme] window.__TAURI__:", window.__TAURI__);
  console.log("[Theme] window.__TAURI_INTERNALS__:", window.__TAURI_INTERNALS__);

  // Detect platform - check user agent for macOS detection (works without Tauri API)
  const isMacOS =
    navigator.platform.toUpperCase().indexOf("MAC") >= 0 ||
    navigator.userAgent.toUpperCase().indexOf("MAC") >= 0;

  // Apply macOS styling if running in Tauri on macOS
  if (isTauri && isMacOS) {
    console.log("[Theme] Detected macOS in Tauri - applying platform styles");
    document.body.classList.add("tauri-macos");
    const header = document.querySelector(".modern-header");
    if (header) {
      header.classList.add("tauri-macos");
      console.log("[Theme] Added macOS-specific styling classes");
    }
  }

  // Initialize Tauri window API if available
  if (isTauri && window.__TAURI__) {
    try {
      // Try multiple API access patterns
      if (window.__TAURI__.window) {
        const { getCurrentWindow } = window.__TAURI__.window;
        tauriWindow = getCurrentWindow();
        console.log("[Theme] Using window.__TAURI__.window API");
      } else if (window.__TAURI__.webviewWindow) {
        const { getCurrentWebviewWindow } = window.__TAURI__.webviewWindow;
        tauriWindow = getCurrentWebviewWindow();
        console.log("[Theme] Using window.__TAURI__.webviewWindow API");
      }

      if (tauriWindow) {
        console.log("[Theme] Successfully initialized Tauri window:", tauriWindow);
        console.log(
          "[Theme] Available methods:",
          Object.getOwnPropertyNames(Object.getPrototypeOf(tauriWindow))
        );
      } else {
        console.warn(
          "[Theme] Could not initialize Tauri window - API structure:",
          Object.keys(window.__TAURI__)
        );
      }

      // Detect platform via Tauri API if available
      if (window.__TAURI__.os) {
        platform = await window.__TAURI__.os.platform();
        console.log("[Theme] Detected platform via Tauri API:", platform);
      }
    } catch (error) {
      console.warn("[Theme] Failed to initialize Tauri window API:", error);
    }
  }

  // Load saved theme or default to dark
  const savedTheme = localStorage.getItem("theme") || "dark";
  await setTheme(savedTheme);

  // Theme toggle click handler
  themeToggle.addEventListener("click", async () => {
    const currentTheme = html.getAttribute("data-theme");
    const newTheme = currentTheme === "light" ? "dark" : "light";
    console.log("[Theme] User toggled theme from", currentTheme, "to", newTheme);
    await setTheme(newTheme);
    try {
      localStorage.setItem("theme", newTheme);
    } catch (e) {
      console.warn("[Theme] Failed to save theme to localStorage:", e);
    }
  });

  // Listen to system theme changes if in Tauri
  if (tauriWindow) {
    try {
      await tauriWindow.onThemeChanged(async (event) => {
        const systemTheme = event.payload; // 'light' or 'dark'
        const savedTheme = localStorage.getItem("theme");
        console.log("[Theme] System theme changed to:", systemTheme);

        // Only auto-sync if user hasn't explicitly set a theme preference
        if (!savedTheme) {
          await setTheme(systemTheme);
        }
      });
    } catch (error) {
      console.warn("[Theme] Failed to listen to system theme changes:", error);
    }
  }

  async function setTheme(theme) {
    console.log("[Theme] Setting theme to:", theme);
    html.setAttribute("data-theme", theme);

    // Update UI elements
    if (theme === "dark") {
      themeIcon.className = "action-icon icon-sun";
      if (themeText) themeText.textContent = "Light";
    } else {
      themeIcon.className = "action-icon icon-moon";
      if (themeText) themeText.textContent = "Dark";
    }

    // Sync with Tauri window theme (native title bar color on supported platforms)
    if (tauriWindow) {
      try {
        console.log("[Theme] Syncing Tauri window theme to:", theme);
        await tauriWindow.setTheme(theme);
        console.log("[Theme] Successfully synced window theme");
      } catch (error) {
        // setTheme may not be available on all platforms
        console.log("[Theme] Window theme sync not supported on this platform:", error.message);
      }
    }
  }
})();
