// Splash Screen Controller
// Shows on every browser app start, handles the initialization check and routing.
//
// Electron paints its own launch window and hides this one, so everything below
// is the browser launch. The screen reports only what the backend has actually
// told us: it used to narrate five invented phases on timers and hold the
// dashboard for a fixed three seconds, which made a fast launch slower than a
// slow one and described work that was never happening.

// Shortest time the screen stays up. Long enough that a launch which answers
// instantly reads as a screen rather than a flash, short enough not to be a wait.
const SPLASH_MIN_DURATION = 500;
// How long the screen fades before it is taken out of the layout. Matches the
// `transition` on `.splash-screen` in splash.css.
const SPLASH_FADE_MS = 400;
// A backend that has not answered by now is worth explaining rather than
// leaving the user with a spinner and no reason for it.
const SPLASH_SLOW_AFTER_MS = 4000;

class SplashController {
  constructor() {
    this.splashEl = null;
    this.statusEl = null;
    this.detailEl = null;
    this.startTime = Date.now();
    this.retryTimeout = null;
    this.launchFailed = false;
  }

  /**
   * The bootstrap manager settles every launch. An unreachable backend must end
   * the splash in a stated failure rather than in the retry loop below, which
   * would otherwise keep waiting for as long as the window stays open.
   */
  watchLaunchOutcome() {
    window.addEventListener("screenerbot:bootstrap-settled", (event) => {
      if (event.detail?.outcome !== "unreachable") {
        return;
      }
      this.launchFailed = true;
      if (this.retryTimeout) {
        clearTimeout(this.retryTimeout);
        this.retryTimeout = null;
      }
      this.splashEl?.classList.add("settled");
      this.setState(
        "ScreenerBot could not start",
        "Check the log file, then restart the app."
      );
    });
  }

  init() {
    // Skip splash screen when running in Electron - Electron has its own splash
    // Check multiple ways to detect Electron environment
    const isElectron =
      (window.electronAPI && window.electronAPI.isElectron) ||
      (typeof navigator !== "undefined" && navigator.userAgent.includes("Electron")) ||
      (typeof window !== "undefined" && window.process && window.process.type);

    if (isElectron) {
      // Immediately hide splash - Electron has its own loading screen
      // But still check initialization status to decide what to show
      const splashEl = document.getElementById("splashScreen");
      if (splashEl) {
        splashEl.style.display = "none";
      }
      // The router.js will handle showing setup screen if needed
      return;
    }

    this.splashEl = document.getElementById("splashScreen");
    this.statusEl = document.getElementById("splashStatus");
    this.detailEl = document.getElementById("splashDetail");

    if (!this.splashEl) {
      console.warn("[Splash] Splash screen element not found");
      return;
    }

    this.watchLaunchOutcome();

    // Load and display version
    this.loadVersion();

    // Check initialization status
    this.checkInitialization();
  }

  async loadVersion() {
    try {
      const response = await fetch("/api/version");
      if (response.ok) {
        const data = await response.json();
        const versionEl = document.getElementById("splashVersion");
        if (versionEl && data.version) {
          versionEl.textContent = `v${data.version}`;
        }
      }
    } catch (err) {
      console.warn("[Splash] Failed to load version:", err);
    }
  }

  setState(status, detail = "") {
    if (this.statusEl && status) this.statusEl.textContent = status;
    if (this.detailEl) this.detailEl.textContent = detail;
  }

  async checkInitialization() {
    try {
      const response = await fetch("/api/initialization/status");
      if (!response.ok) {
        throw new Error(`HTTP ${response.status}`);
      }

      const result = await response.json();

      // Determine transition target
      let target;
      if (result.force_onboarding) {
        target = "onboarding";
      } else if (result.required) {
        target = result.onboarding_complete ? "setup" : "onboarding";
      } else {
        target = "dashboard";
      }

      const elapsed = Date.now() - this.startTime;
      const remainingTime = Math.max(0, SPLASH_MIN_DURATION - elapsed);
      await new Promise((resolve) => setTimeout(resolve, remainingTime));

      this.transitionTo(target);
    } catch (error) {
      console.error("[Splash] Failed to check initialization:", error);
      if (this.launchFailed) {
        // The launch already settled as failed; retrying only hides the reason.
        return;
      }
      // Initialization state is authoritative. Never guess "dashboard" on a
      // transient failure because that can bypass first-run onboarding.
      if (Date.now() - this.startTime >= SPLASH_SLOW_AFTER_MS) {
        this.setState("Starting ScreenerBot", "Waiting for the local core to answer.");
      }
      this.retryTimeout = setTimeout(() => this.checkInitialization(), 1500);
    }
  }

  transitionTo(destination) {
    if (!this.splashEl) return;

    // If going to onboarding/setup, add initialization-mode class to hide dashboard
    // This prevents dashboard elements from being visible behind overlays
    if (destination === "onboarding" || destination === "setup") {
      document.body.classList.add("initialization-mode");
    }

    // Show next screen BEFORE fading splash to prevent flash
    switch (destination) {
      case "onboarding":
        this.showOnboarding();
        break;
      case "setup":
        this.showSetup();
        break;
      case "dashboard":
        // Remove initialization-mode to show dashboard
        document.body.classList.remove("initialization-mode");
        break;
    }

    // Add fade-out class
    this.splashEl.classList.add("fade-out");

    // Wait for animation then hide splash
    setTimeout(() => {
      this.splashEl.style.display = "none";
    }, SPLASH_FADE_MS);
  }

  showOnboarding() {
    const onboardingEl = document.getElementById("onboardingScreen");
    if (onboardingEl) {
      onboardingEl.style.display = "grid";
      // Initialize onboarding controller if not already
      if (window.OnboardingController) {
        window.OnboardingController.init();
      }
    }
  }

  showSetup() {
    // Show the wrapper first (it's hidden by default in base.html)
    const wrapperEl = document.getElementById("setupScreenWrapper");
    if (wrapperEl) {
      wrapperEl.style.display = "block";
    }

    const setupEl = document.getElementById("setupScreen");
    if (setupEl) {
      setupEl.style.display = "grid";
      // Initialize setup controller if not already
      if (window.SetupController) {
        window.SetupController.init();
      }
    }
  }

}

// Export for use
window.SplashController = new SplashController();

// Auto-initialize when DOM is ready
if (document.readyState === "loading") {
  document.addEventListener("DOMContentLoaded", () => window.SplashController.init());
} else {
  window.SplashController.init();
}
