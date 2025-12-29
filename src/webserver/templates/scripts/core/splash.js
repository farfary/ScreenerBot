// Splash Screen Controller
// Shows on every app start, handles initialization check and routing

const SPLASH_MIN_DURATION = 3000; // Minimum splash display time in ms

// Status messages shown sequentially (not rotating)
const SPLASH_PHASES = [
  { message: "Starting ScreenerBot...", duration: 600 },
  { message: "Loading configuration...", duration: 500 },
  { message: "Connecting to Solana RPC...", duration: 700 },
  { message: "Initializing services...", duration: 600 },
  { message: "Preparing dashboard...", duration: 500 },
];

class SplashController {
  constructor() {
    this.splashEl = null;
    this.statusEl = null;
    this.startTime = Date.now();
    this.phaseIndex = 0;
    this.phaseTimeout = null;
    this.forceOnboarding = false;
    this.initializationRequired = false;
    this.readyToTransition = false;
    this.transitionTarget = null;
  }

  init() {
    // Skip splash screen when running in Electron - Electron has its own splash
    // Check multiple ways to detect Electron environment
    const isElectron = (
      (window.electronAPI && window.electronAPI.isElectron) ||
      (typeof navigator !== 'undefined' && navigator.userAgent.includes('Electron')) ||
      (typeof window !== 'undefined' && window.process && window.process.type)
    );
    
    if (isElectron) {
      // Immediately hide splash and proceed to dashboard
      const splashEl = document.getElementById("splashScreen");
      if (splashEl) {
        splashEl.style.display = "none";
      }
      return;
    }

    this.splashEl = document.getElementById("splashScreen");
    this.statusEl = document.getElementById("splashStatus");

    if (!this.splashEl) {
      console.warn("[Splash] Splash screen element not found");
      return;
    }

    // Load and display version
    this.loadVersion();

    // Start sequential status messages
    this.startPhaseSequence();

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

  startPhaseSequence() {
    if (!this.statusEl) return;

    // Show first phase immediately
    this.statusEl.textContent = SPLASH_PHASES[0].message;
    this.phaseIndex = 0;

    this.advancePhase();
  }

  advancePhase() {
    const currentPhase = SPLASH_PHASES[this.phaseIndex];

    this.phaseTimeout = setTimeout(() => {
      this.phaseIndex++;

      if (this.phaseIndex < SPLASH_PHASES.length) {
        // Show next phase message
        this.statusEl.textContent = SPLASH_PHASES[this.phaseIndex].message;
        this.advancePhase();
      } else {
        // All phases complete - show ready or wait
        this.statusEl.textContent = "Ready";
        this.checkReadyToTransition();
      }
    }, currentPhase.duration);
  }

  stopPhaseSequence() {
    if (this.phaseTimeout) {
      clearTimeout(this.phaseTimeout);
      this.phaseTimeout = null;
    }
  }

  checkReadyToTransition() {
    if (this.readyToTransition && this.transitionTarget) {
      this.transitionTo(this.transitionTarget);
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

      // Determine transition target
      if (this.forceOnboarding) {
        this.transitionTarget = "onboarding";
      } else if (result.required) {
        this.transitionTarget = result.onboarding_complete ? "setup" : "onboarding";
      } else {
        this.transitionTarget = "dashboard";
      }

      // Wait for minimum duration
      const elapsed = Date.now() - this.startTime;
      const remainingTime = Math.max(0, SPLASH_MIN_DURATION - elapsed);
      await new Promise((resolve) => setTimeout(resolve, remainingTime));

      this.readyToTransition = true;
      this.stopPhaseSequence();
      this.checkReadyToTransition();
    } catch (error) {
      console.error("[Splash] Failed to check initialization:", error);

      // On error, proceed to dashboard
      this.transitionTarget = "dashboard";

      const elapsed = Date.now() - this.startTime;
      const remainingTime = Math.max(0, SPLASH_MIN_DURATION - elapsed);
      await new Promise((resolve) => setTimeout(resolve, remainingTime));

      this.readyToTransition = true;
      this.stopPhaseSequence();
      this.checkReadyToTransition();
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
