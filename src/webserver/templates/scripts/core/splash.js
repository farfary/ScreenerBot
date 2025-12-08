// Splash Screen Controller
// Shows on every app start, handles initialization check and routing

const SPLASH_MIN_DURATION = 1500; // Minimum splash display time in ms
const SPLASH_STATUS_MESSAGES = [
  "Initializing...",
  "Checking configuration...",
  "Connecting to services...",
  "Almost ready...",
];

class SplashController {
  constructor() {
    this.splashEl = null;
    this.statusEl = null;
    this.startTime = Date.now();
    this.statusIndex = 0;
    this.statusInterval = null;
    this.forceOnboarding = false;
    this.initializationRequired = false;
  }

  init() {
    this.splashEl = document.getElementById("splashScreen");
    this.statusEl = document.getElementById("splashStatus");

    if (!this.splashEl) {
      console.warn("[Splash] Splash screen element not found");
      return;
    }

    // Load and display version
    this.loadVersion();

    // Start rotating status messages
    this.startStatusRotation();

    // Check initialization status
    this.checkInitialization();
  }

  async loadVersion() {
    try {
      const response = await fetch('/api/version');
      if (response.ok) {
        const data = await response.json();
        const versionEl = document.getElementById('splashVersion');
        if (versionEl && data.version) {
          versionEl.textContent = `v${data.version}`;
        }
      }
    } catch (err) {
      console.warn("[Splash] Failed to load version:", err);
    }
  }

  startStatusRotation() {
    if (!this.statusEl) return;

    this.statusInterval = setInterval(() => {
      this.statusIndex = (this.statusIndex + 1) % SPLASH_STATUS_MESSAGES.length;
      this.statusEl.textContent = SPLASH_STATUS_MESSAGES[this.statusIndex];
    }, 800);
  }

  stopStatusRotation() {
    if (this.statusInterval) {
      clearInterval(this.statusInterval);
      this.statusInterval = null;
    }
  }

  async checkInitialization() {
    try {
      const response = await fetch("/api/initialization/status");
      if (!response.ok) {
        throw new Error(`HTTP ${response.status}`);
      }

      const result = await response.json();
      this.forceOnboarding = !!result.force_onboarding;
      this.initializationRequired = !!result.required;
      const elapsed = Date.now() - this.startTime;
      const remainingTime = Math.max(0, SPLASH_MIN_DURATION - elapsed);

      // Wait for minimum splash duration
      await new Promise((resolve) => setTimeout(resolve, remainingTime));

      this.stopStatusRotation();

      if (this.forceOnboarding) {
        this.transitionTo("onboarding");
        return;
      }

      // Determine next destination
      if (result.required) {
        // Check if onboarding needed (backend controls this)
        if (!result.onboarding_complete) {
          this.transitionTo("onboarding");
        } else {
          this.transitionTo("setup");
        }
      } else {
        // Fully initialized - go to dashboard
        this.transitionTo("dashboard");
      }
    } catch (error) {
      console.error("[Splash] Failed to check initialization:", error);
      this.stopStatusRotation();

      // On error, try to proceed to dashboard
      const elapsed = Date.now() - this.startTime;
      const remainingTime = Math.max(0, SPLASH_MIN_DURATION - elapsed);
      await new Promise((resolve) => setTimeout(resolve, remainingTime));
      this.transitionTo("dashboard");
    }
  }

  transitionTo(destination) {
    if (!this.splashEl) return;

    // Add fade-out class
    this.splashEl.classList.add("fade-out");

    // Wait for animation then handle transition
    setTimeout(() => {
      this.splashEl.style.display = "none";

      switch (destination) {
        case "onboarding":
          this.showOnboarding();
          break;
        case "setup":
          this.showSetup();
          break;
        case "dashboard":
          // Dashboard is already rendered, just show it
          break;
      }
    }, 500);
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

  needsSetupAfterOnboarding() {
    return this.initializationRequired;
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
