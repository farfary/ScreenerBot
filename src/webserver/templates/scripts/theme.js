// Theme Management
(async function () {
  const html = document.documentElement;
  const themeToggle = document.getElementById("themeToggle");
  const themeIcon = document.getElementById("themeIcon");
  const themeText = document.getElementById("themeText");

  // Check if running in Tauri
  const isTauri = window.__TAURI__ !== undefined;
  let tauriWindow = null;

  console.log("[Theme] Initializing theme system...");
  console.log("[Theme] Running in Tauri:", isTauri);
  console.log("[Theme] window.__TAURI__:", window.__TAURI__);

  // Initialize Tauri window API if available
  if (isTauri) {
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
      } else if (window.__TAURI_INTERNALS__) {
        console.log(
          "[Theme] Found __TAURI_INTERNALS__, checking structure:",
          Object.keys(window.__TAURI_INTERNALS__)
        );
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

    // Sync with Tauri window title bar theme
    if (tauriWindow) {
      try {
        console.log("[Theme] Calling tauriWindow.setTheme with:", theme);
        const result = await tauriWindow.setTheme(theme);
        console.log("[Theme] setTheme result:", result);
      } catch (error) {
        console.error("[Theme] Failed to set Tauri window theme:", error);
        console.error("[Theme] Error details:", error.message, error.stack);
      }
    } else {
      console.log("[Theme] Skipping Tauri window theme (not in Tauri environment)");
    }
  }
})();
