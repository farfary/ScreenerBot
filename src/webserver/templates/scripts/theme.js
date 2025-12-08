// Theme Management
// Uses server-side state storage for persistence across Tauri restarts
(async function () {
  const html = document.documentElement;
  const themeToggle = document.getElementById("themeToggle");
  const themeIcon = document.getElementById("themeIcon");
  const themeText = document.getElementById("themeText");

  // Check if running in Tauri
  const isTauri =
    window.__TAURI__ !== undefined || window.__TAURI_INTERNALS__ !== undefined;
  let tauriWindow = null;

  console.log("[Theme] Initializing theme system...");
  console.log("[Theme] Running in Tauri:", isTauri);

  // Detect platform - check user agent for macOS detection
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
        console.log("[Theme] Successfully initialized Tauri window");
      }

      // Detect platform via Tauri API if available
      if (window.__TAURI__.os) {
        const platform = await window.__TAURI__.os.platform();
        console.log("[Theme] Detected platform via Tauri API:", platform);
      }
    } catch (error) {
      console.warn("[Theme] Failed to initialize Tauri window API:", error);
    }
  }

  // Load saved theme from server or default to dark
  let savedTheme = "dark";
  try {
    const response = await fetch("/api/ui-state/load", {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify({ key: "theme" }),
    });
    if (response.ok) {
      const data = await response.json();
      if (data.value) {
        savedTheme = data.value;
      }
    }
  } catch (e) {
    console.warn("[Theme] Failed to load theme from server, using default:", e);
  }
  await setTheme(savedTheme);

  // Theme toggle click handler
  themeToggle.addEventListener("click", async () => {
    const currentTheme = html.getAttribute("data-theme");
    const newTheme = currentTheme === "light" ? "dark" : "light";
    console.log("[Theme] User toggled theme from", currentTheme, "to", newTheme);
    await setTheme(newTheme);

    // Save to server
    try {
      await fetch("/api/ui-state/save", {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({ key: "theme", value: newTheme }),
      });
    } catch (e) {
      console.warn("[Theme] Failed to save theme to server:", e);
    }
  });

  // Listen to system theme changes if in Tauri
  if (tauriWindow) {
    try {
      await tauriWindow.onThemeChanged(async (event) => {
        const systemTheme = event.payload;
        console.log("[Theme] System theme changed to:", systemTheme);

        // Check if user has set a preference
        try {
          const response = await fetch("/api/ui-state/load", {
            method: "POST",
            headers: { "Content-Type": "application/json" },
            body: JSON.stringify({ key: "theme" }),
          });
          if (response.ok) {
            const data = await response.json();
            // Only auto-sync if user hasn't explicitly set a theme preference
            if (!data.value) {
              await setTheme(systemTheme);
            }
          }
        } catch (e) {
          // On error, follow system theme
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

    // Sync with Tauri window theme
    if (tauriWindow) {
      try {
        console.log("[Theme] Syncing Tauri window theme to:", theme);
        await tauriWindow.setTheme(theme);
        console.log("[Theme] Successfully synced window theme");
      } catch (error) {
        console.log(
          "[Theme] Window theme sync not supported on this platform:",
          error.message
        );
      }
    }
  }

  // ============================================================================
  // CUSTOM TITLE BAR DOUBLE-CLICK HANDLER (macOS smart maximize)
  // ============================================================================

  if (isTauri && isMacOS) {
    setupSmartMaximizeHandler();
  }

  function setupSmartMaximizeHandler() {
    console.log("[Theme] Setting up smart maximize handler for macOS");

    const dragRegions = document.querySelectorAll("[data-tauri-drag-region]");

    if (dragRegions.length === 0) {
      console.warn("[Theme] No drag regions found for smart maximize handler");
      return;
    }

    console.log(`[Theme] Found ${dragRegions.length} drag regions`);

    let lastClickTime = 0;
    const DOUBLE_CLICK_THRESHOLD = 300;

    dragRegions.forEach((region) => {
      region.addEventListener(
        "mousedown",
        async (e) => {
          if (e.button !== 0) return;

          const now = Date.now();
          const timeSinceLastClick = now - lastClickTime;
          lastClickTime = now;

          if (timeSinceLastClick < DOUBLE_CLICK_THRESHOLD) {
            console.log("[Theme] Double-click detected on drag region");

            e.preventDefault();
            e.stopPropagation();
            e.stopImmediatePropagation();

            try {
              if (window.__TAURI__?.core?.invoke) {
                await window.__TAURI__.core.invoke("smart_maximize");
                console.log("[Theme] Smart maximize command executed");
              } else if (window.__TAURI_INTERNALS__?.invoke) {
                await window.__TAURI_INTERNALS__.invoke("smart_maximize");
                console.log("[Theme] Smart maximize command executed (via internals)");
              } else {
                console.warn("[Theme] Tauri invoke not available");
              }
            } catch (error) {
              console.error("[Theme] Smart maximize failed:", error);
            }

            lastClickTime = 0;
          }
        },
        { capture: true }
      );
    });

    console.log("[Theme] Smart maximize handler installed");
  }
})();
