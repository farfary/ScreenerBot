// Onboarding Flow Controller
// Handles the multi-page introduction for first-time users

class OnboardingControllerClass {
  constructor() {
    this.currentSlide = 0;
    this.totalSlides = 5;
    this.initialized = false;
  }

  init() {
    if (this.initialized) return;

    this.slides = document.querySelectorAll(".onboarding-slide");
    this.dots = document.querySelectorAll(".progress-dot");
    this.prevBtn = document.getElementById("onboardingPrev");
    this.nextBtn = document.getElementById("onboardingNext");
    this.skipBtn = document.getElementById("onboardingSkip");

    if (!this.slides.length) {
      console.warn("[Onboarding] Slides not found");
      return;
    }

    this.totalSlides = this.slides.length;
    this.bindEvents();
    this.goToSlide(0);
    this.loadVersion();
    this.initialized = true;
  }

  async loadVersion() {
    try {
      const response = await fetch("/api/version");
      if (response.ok) {
        const data = await response.json();
        const versionEl = document.getElementById("onboarding-version");
        if (versionEl && data.version) {
          versionEl.textContent = `v${data.version}`;
        }
      }
    } catch {
      console.warn("[Onboarding] Failed to load version");
    }
  }

  bindEvents() {
    // Navigation buttons
    if (this.prevBtn) {
      this.prevBtn.addEventListener("click", () => this.prev());
    }
    if (this.nextBtn) {
      this.nextBtn.addEventListener("click", () => this.next());
    }
    if (this.skipBtn) {
      this.skipBtn.addEventListener("click", () => this.complete());
    }

    // Dot navigation
    this.dots.forEach((dot) => {
      dot.addEventListener("click", () => {
        const slideIndex = parseInt(dot.dataset.dot, 10);
        this.goToSlide(slideIndex);
      });
    });

    // Feature card hover effect
    document.addEventListener("mousemove", (e) => {
      const onboardingScreen = document.getElementById("onboardingScreen");
      if (!onboardingScreen || onboardingScreen.style.display === "none") return;

      const cards = document.querySelectorAll(".onboarding-slide.active .slide-feature");
      cards.forEach((card) => {
        const rect = card.getBoundingClientRect();
        const x = e.clientX - rect.left;
        const y = e.clientY - rect.top;
        card.style.setProperty("--x", `${x}px`);
        card.style.setProperty("--y", `${y}px`);
      });
    });

    // Keyboard navigation
    document.addEventListener("keydown", (e) => {
      const onboardingScreen = document.getElementById("onboardingScreen");
      if (!onboardingScreen || onboardingScreen.style.display === "none") return;

      if (e.key === "ArrowRight" || e.key === "Enter") {
        this.next();
      } else if (e.key === "ArrowLeft") {
        this.prev();
      } else if (e.key === "Escape") {
        this.complete();
      }
    });
  }

  goToSlide(index) {
    if (index < 0 || index >= this.totalSlides) return;

    // Update theme
    const screen = document.getElementById("onboardingScreen");
    if (screen) {
      const themes = ["blue", "purple", "green", "amber", "cyan"];
      screen.setAttribute("data-theme", themes[index] || "blue");
    }

    // Update slide classes
    this.slides.forEach((slide, i) => {
      slide.classList.remove("active", "prev");
      if (i === index) {
        slide.classList.add("active");
        // Stagger feature card animations
        const cards = slide.querySelectorAll(".slide-feature");
        cards.forEach((card, ci) => {
          card.style.animationDelay = `${0.1 + ci * 0.08}s`;
          card.classList.remove("slide-feature-enter");
          void card.offsetWidth; // force reflow
          card.classList.add("slide-feature-enter");
        });
      } else if (i < index) {
        slide.classList.add("prev");
      }
    });

    this.currentSlide = index;
    this.updateUI();
  }

  prev() {
    if (this.currentSlide > 0) {
      this.goToSlide(this.currentSlide - 1);
    }
  }

  next() {
    if (this.currentSlide < this.totalSlides - 1) {
      this.goToSlide(this.currentSlide + 1);
    } else {
      this.complete();
    }
  }

  updateUI() {
    // Update dots
    this.dots.forEach((dot, i) => {
      dot.classList.toggle("active", i === this.currentSlide);
    });

    // Update buttons
    if (this.prevBtn) {
      this.prevBtn.disabled = this.currentSlide === 0;
    }
    if (this.nextBtn) {
      if (this.currentSlide === this.totalSlides - 1) {
        this.nextBtn.innerHTML = 'Get Started <i class="icon-arrow-right"></i>';
      } else {
        this.nextBtn.innerHTML = 'Next <i class="icon-chevron-right"></i>';
      }
    }
  }

  complete() {
    // Mark onboarding as complete in backend (in-memory only, not saved to disk)
    fetch("/api/initialization/onboarding/complete", { method: "POST" })
      .then((response) => {
        if (!response.ok) {
          console.error("[Onboarding] Failed to update completion state");
        }
      })
      .catch((err) => {
        console.error("[Onboarding] Error updating completion state:", err);
      });

    // Hide onboarding screen
    const onboardingScreen = document.getElementById("onboardingScreen");
    if (onboardingScreen) {
      onboardingScreen.style.display = "none";
    }

    // Always show setup after onboarding - we're in the initialization flow
    // so setup is always required (otherwise we wouldn't be in onboarding)
    const setupWrapper = document.getElementById("setupScreenWrapper");
    if (setupWrapper) {
      setupWrapper.style.display = "block";
      const setupScreen = document.getElementById("setupScreen");
      if (setupScreen) {
        setupScreen.style.display = "grid";
      }
      if (window.SetupController) {
        window.SetupController.init();
      }
    } else {
      console.error("[Onboarding] Setup screen wrapper not found!");
    }
  }
}

// Export for use
window.OnboardingController = new OnboardingControllerClass();
