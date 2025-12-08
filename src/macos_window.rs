/// macOS-specific window management using Cocoa APIs
///
/// Provides smart window maximization that respects macOS system settings,
/// including "Tiled windows have margins" and proper zoom behavior.

use crate::logger::{self, LogTag};
use objc2::rc::Retained;
use objc2::runtime::AnyObject;
use objc2_app_kit::NSWindow;
use objc2_foundation::{NSNumber, NSRect, NSString};
use tauri::WebviewWindow;

/// The margin size used by macOS when "Tiled windows have margins" is enabled.
/// This matches the system's default tiling margin (8 points on each side).
const TILED_WINDOW_MARGIN: f64 = 8.0;

/// State tracking for toggle behavior
static PREVIOUS_FRAME: std::sync::Mutex<Option<NSRect>> = std::sync::Mutex::new(None);

/// Check if "Tiled windows have margins" is enabled in System Settings
///
/// Reads from com.apple.WindowManager's EnableTiledWindowMargins key
fn is_tiled_window_margins_enabled() -> bool {
  unsafe {
    // Create NSUserDefaults instance for the com.apple.WindowManager domain
    let domain = NSString::from_str("com.apple.WindowManager");
    let key = NSString::from_str("EnableTiledWindowMargins");

    // Allocate and init with suite name
    let defaults: *mut AnyObject = objc2::msg_send![objc2::class!(NSUserDefaults), alloc];
    let defaults: *mut AnyObject = objc2::msg_send![defaults, initWithSuiteName: &*domain];

    if defaults.is_null() {
      logger::debug(
        LogTag::System,
        "Could not create NSUserDefaults for WindowManager domain, defaulting to enabled",
      );
      return true;
    }

    // Get the object for key first to check if it exists
    let value: *const AnyObject = objc2::msg_send![defaults, objectForKey: &*key];
    
    if value.is_null() {
      // Key doesn't exist, default to true (macOS default has margins enabled)
      logger::debug(
        LogTag::System,
        "EnableTiledWindowMargins key not found, defaulting to enabled",
      );
      return true;
    }

    // Get boolean value
    let enabled: bool = objc2::msg_send![defaults, boolForKey: &*key];
    logger::info(
      LogTag::System,
      &format!("Tiled window margins setting: {}", enabled),
    );
    enabled
  }
}

/// Smart maximize for macOS - respects tiled window margins setting
///
/// This function:
/// - Reads the "Tiled windows have margins" setting from System Settings
/// - Applies appropriate margins when maximizing if enabled
/// - Toggles between maximized and previous state
pub fn smart_maximize_macos(window: &WebviewWindow) -> Result<(), String> {
  logger::info(
    LogTag::System,
    "Smart maximize: Calculating frame with margin support",
  );

  unsafe {
    // Get the NSWindow from the Tauri window
    let ns_window = get_ns_window(window)?;

    // Get the screen containing the window
    let screen = ns_window
      .screen()
      .ok_or_else(|| "Window has no associated screen".to_string())?;

    // Get the visible frame (excludes menu bar and dock)
    let visible_frame = screen.visibleFrame();
    let current_frame = ns_window.frame();

    // Check if margins should be applied
    let margins_enabled = is_tiled_window_margins_enabled();
    let margin = if margins_enabled {
      TILED_WINDOW_MARGIN
    } else {
      0.0
    };

    // Calculate the target frame with margins
    let target_frame = NSRect {
      origin: objc2_foundation::NSPoint {
        x: visible_frame.origin.x + margin,
        y: visible_frame.origin.y + margin,
      },
      size: objc2_foundation::NSSize {
        width: visible_frame.size.width - (margin * 2.0),
        height: visible_frame.size.height - (margin * 2.0),
      },
    };

    logger::debug(
      LogTag::System,
      &format!(
        "Smart maximize: margins_enabled={}, margin={}, visible_frame=({:.0}, {:.0}, {:.0}x{:.0}), target_frame=({:.0}, {:.0}, {:.0}x{:.0})",
        margins_enabled,
        margin,
        visible_frame.origin.x,
        visible_frame.origin.y,
        visible_frame.size.width,
        visible_frame.size.height,
        target_frame.origin.x,
        target_frame.origin.y,
        target_frame.size.width,
        target_frame.size.height,
      ),
    );

    // Check if window is already at target frame (with some tolerance)
    let is_maximized = is_frame_equal(&current_frame, &target_frame, 10.0);

    if is_maximized {
      // Restore previous frame if available
      let mut prev_guard = PREVIOUS_FRAME.lock().unwrap();
      if let Some(prev_frame) = prev_guard.take() {
        logger::info(
          LogTag::System,
          &format!(
            "Smart maximize: Restoring to ({:.0}, {:.0}, {:.0}x{:.0})",
            prev_frame.origin.x,
            prev_frame.origin.y,
            prev_frame.size.width,
            prev_frame.size.height,
          ),
        );
        ns_window.setFrame_display_animate(prev_frame, true, true);
      } else {
        // No previous frame, center window with default size
        logger::info(
          LogTag::System,
          "Smart maximize: No previous frame, centering window",
        );
        let default_width = 1200.0_f64.min(visible_frame.size.width * 0.8);
        let default_height = 800.0_f64.min(visible_frame.size.height * 0.8);
        let default_frame = NSRect {
          origin: objc2_foundation::NSPoint {
            x: visible_frame.origin.x + (visible_frame.size.width - default_width) / 2.0,
            y: visible_frame.origin.y + (visible_frame.size.height - default_height) / 2.0,
          },
          size: objc2_foundation::NSSize {
            width: default_width,
            height: default_height,
          },
        };
        ns_window.setFrame_display_animate(default_frame, true, true);
      }
    } else {
      // Save current frame and maximize
      {
        let mut prev_guard = PREVIOUS_FRAME.lock().unwrap();
        *prev_guard = Some(current_frame);
      }

      logger::info(
        LogTag::System,
        &format!(
          "Smart maximize: Maximizing to ({:.0}, {:.0}, {:.0}x{:.0}) with {}px margin",
          target_frame.origin.x,
          target_frame.origin.y,
          target_frame.size.width,
          target_frame.size.height,
          margin,
        ),
      );
      ns_window.setFrame_display_animate(target_frame, true, true);
    }

    Ok(())
  }
}

/// Get the NSWindow from a Tauri WebviewWindow
///
/// This uses the raw window handle to access the underlying Cocoa NSWindow.
unsafe fn get_ns_window(window: &WebviewWindow) -> Result<Retained<NSWindow>, String> {
  use raw_window_handle::{HasWindowHandle, RawWindowHandle};

  let handle = window
    .window_handle()
    .map_err(|e| format!("Failed to get window handle: {}", e))?;

  match handle.as_raw() {
    RawWindowHandle::AppKit(app_kit_handle) => {
      // The ns_view is the NSView, we need to get its window
      let ns_view = app_kit_handle.ns_view.as_ptr() as *mut AnyObject;
      if ns_view.is_null() {
        return Err("NSView pointer is null".to_string());
      }

      // Get the NSWindow from the NSView
      let ns_window: *mut AnyObject = objc2::msg_send![ns_view, window];
      if ns_window.is_null() {
        return Err("NSWindow is null".to_string());
      }

      // Retain the window to ensure it lives long enough
      Ok(Retained::retain(ns_window as *mut NSWindow).ok_or("Failed to retain NSWindow")?)
    }
    _ => Err("Not an AppKit window handle".to_string()),
  }
}

/// Check if two frames are equal within a tolerance
fn is_frame_equal(a: &NSRect, b: &NSRect, tolerance: f64) -> bool {
  (a.origin.x - b.origin.x).abs() < tolerance
    && (a.origin.y - b.origin.y).abs() < tolerance
    && (a.size.width - b.size.width).abs() < tolerance
    && (a.size.height - b.size.height).abs() < tolerance
}

/// Start native window drag using Cocoa API
/// 
/// This is a workaround for the Tauri bug where TitleBarStyle::Overlay
/// breaks window dragging on macOS (issue #9503).
/// 
/// We use NSWindow's setMovableByWindowBackground: to enable dragging
/// from anywhere in the window.
pub fn set_window_draggable(window: &WebviewWindow, draggable: bool) -> Result<(), String> {
  unsafe {
    let ns_window = get_ns_window(window)?;
    
    // setMovableByWindowBackground: allows the window to be moved by
    // clicking and dragging anywhere on the window background
    let _: () = objc2::msg_send![&*ns_window, setMovableByWindowBackground: draggable];
    
    logger::debug(
      LogTag::System,
      &format!("Set window movableByWindowBackground to {}", draggable),
    );
    
    Ok(())
  }
}

/// Enable the window to accept first mouse click for dragging
/// This allows dragging the window even when it's not focused
pub fn set_accepts_first_mouse(window: &WebviewWindow, accepts: bool) -> Result<(), String> {
  unsafe {
    let ns_window = get_ns_window(window)?;
    
    // Get the content view
    let content_view: *mut AnyObject = objc2::msg_send![&*ns_window, contentView];
    if !content_view.is_null() {
      // Note: acceptsFirstMouse is actually a method on NSView that needs
      // to be overridden. We can't easily set it. Instead, we use
      // setAcceptsMouseMovedEvents to help with responsiveness.
      let _: () = objc2::msg_send![&*ns_window, setAcceptsMouseMovedEvents: accepts];
    }
    
    logger::debug(
      LogTag::System,
      &format!("Set window acceptsMouseMovedEvents to {}", accepts),
    );
    
    Ok(())
  }
}
