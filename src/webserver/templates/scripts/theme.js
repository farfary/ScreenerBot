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

    // Enable native window dragging via Rust command
    // This is a workaround for Tauri bug #9503 where TitleBarStyle::Overlay
    // breaks window dragging on macOS
    if (window.__TAURI__ && window.__TAURI__.core) {
      try {
        await window.__TAURI__.core.invoke("enable_window_drag");
        console.log("[Theme] Enabled native window dragging via Rust command");
      } catch (error) {
        console.warn("[Theme] Failed to enable native window dragging:", error);
      }
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
  // WINDOW DRAG HANDLING - Manual startDragging() for Tauri v2
  // Based on official Tauri v2 docs: https://v2.tauri.app/learn/window-customization/
  // The data-tauri-drag-region attribute requires manual JS implementation
  // when there are interactive child elements.
  // ============================================================================

  if (isTauri) {
    setupWindowDragging();
  }

  function setupWindowDragging() {
    console.log("[Theme] Setting up manual window drag handler");

    // Get the window API - must use correct API for Tauri v2
    let appWindow = null;
    
    if (window.__TAURI__?.window?.getCurrentWindow) {
      appWindow = window.__TAURI__.window.getCurrentWindow();
      console.log("[Theme] Got window from __TAURI__.window.getCurrentWindow()");
    } else if (window.__TAURI__?.webviewWindow?.getCurrentWebviewWindow) {
      appWindow = window.__TAURI__.webviewWindow.getCurrentWebviewWindow();
      console.log("[Theme] Got window from __TAURI__.webviewWindow.getCurrentWebviewWindow()");
    } else if (tauriWindow) {
      appWindow = tauriWindow;
      console.log("[Theme] Using cached tauriWindow");
    }

    if (!appWindow) {
      console.warn("[Theme] Could not get Tauri window API - dragging disabled");
      return;
    }

    // Elements that should NOT trigger drag (interactive elements)
    const interactiveSelectors = [
      "button",
      "a",
      "input",
      "select",
      "textarea",
      "[role='button']",
      ".header-card",
      ".header-brand",
      ".header-action-btn",
      ".header-actions",
      ".nav-tabs",
      ".nav-tab",
      ".ticker-segment",
    ].join(",");

    // Setup drag on title bar background (28px at top)
    const titleBarBg = document.querySelector(".tauri-titlebar-bg");
    if (titleBarBg) {
      setupDragOnElement(titleBarBg, appWindow, interactiveSelectors);
      console.log("[Theme] Drag handler installed on .tauri-titlebar-bg");
    }

    // Setup drag on header row 1 (main header with logo, cards, buttons)
    const headerRow1 = document.querySelector(".header-row-1");
    if (headerRow1) {
      setupDragOnElement(headerRow1, appWindow, interactiveSelectors);
      console.log("[Theme] Drag handler installed on .header-row-1");
    }

    // Also setup on any element with data-tauri-drag-region attribute
    const dragRegions = document.querySelectorAll("[data-tauri-drag-region]");
    dragRegions.forEach((region) => {
      if (region !== titleBarBg && region !== headerRow1) {
        setupDragOnElement(region, appWindow, interactiveSelectors);
      }
    });

    console.log("[Theme] Window drag handler setup complete");
  }

  function setupDragOnElement(element, appWindow, interactiveSelectors) {
    element.addEventListener("mousedown", (e) => {
      // Only handle primary (left) mouse button
      // Use e.buttons (bitmask) as per Tauri docs
      if (e.buttons !== 1) return;

      // Check if click is on an interactive element - allow normal behavior
      const isInteractive = e.target.closest(interactiveSelectors);
      if (isInteractive) {
        return;
      }

      // Prevent default to avoid text selection
      e.preventDefault();

      // e.detail contains click count - 2 means double click
      if (e.detail === 2) {
        // Double click - toggle maximize
        console.log("[Theme] Double-click detected - toggling maximize");
        appWindow.toggleMaximize().catch((err) => {
          console.warn("[Theme] toggleMaximize failed:", err);
          // Fallback to smart_maximize via invoke
          if (window.__TAURI__?.core?.invoke) {
            window.__TAURI__.core.invoke("smart_maximize").catch(() => {});
          }
        });
      } else {
        // Single click - start dragging
        appWindow.startDragging().catch((err) => {
          console.warn("[Theme] startDragging failed:", err);
        });
      }
    });
  }
})();
