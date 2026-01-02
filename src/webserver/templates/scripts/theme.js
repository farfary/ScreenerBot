// Theme Management
// Uses server-side state storage for persistence across app restarts
(async function () {
  const html = document.documentElement;
  const themeToggle = document.getElementById("themeToggle");
  const themeIcon = document.getElementById("themeIcon");
  const themeText = document.getElementById("themeText");

  // Check if running in Electron
  const isElectron = window.electronAPI !== undefined;

  console.log("[Theme] Initializing theme system...");
  console.log("[Theme] Running in Electron:", isElectron);

  // Detect platform - check user agent for macOS detection
  const isMacOS =
    navigator.userAgent.toUpperCase().indexOf("MAC") >= 0 ||
    navigator.platform.toUpperCase().indexOf("MAC") >= 0;

  // Apply macOS styling if running in Electron on macOS
  if (isElectron && isMacOS) {
    console.log("[Theme] Detected macOS in Electron - applying platform styles");
    document.body.classList.add("electron-macos");
    const header = document.querySelector(".modern-header");
    if (header) {
      header.classList.add("electron-macos");
      console.log("[Theme] Added macOS-specific styling classes");
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
  }
})();
