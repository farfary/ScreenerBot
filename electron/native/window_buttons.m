// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1
//
// Node-API helper that sets the opacity of a window's native macOS traffic-light buttons.

// Electron exposes the traffic lights' visibility and position but not their
// appearance. The buttons are AppKit views drawn above the web contents, so a
// page scrim can never darken them; this helper changes their alpha instead,
// which leaves hit-testing, hover glyphs, the zoom button's tiling menu and
// accessibility untouched.
//
// Built by electron/tools/macos-native.js; loaded by electron/src/window_controls.js.

#define NAPI_VERSION 8

#import <AppKit/AppKit.h>
#include <node_api.h>

/**
 * setAlpha(nativeWindowHandle: Buffer, alpha: number): void
 *
 * `nativeWindowHandle` is `BrowserWindow.getNativeWindowHandle()`: a buffer
 * holding the window's content NSView pointer. Must run on the main thread,
 * where Electron runs the main process's JavaScript.
 */
static napi_value SetAlpha(napi_env env, napi_callback_info info) {
  size_t argc = 2;
  napi_value args[2];
  if (napi_get_cb_info(env, info, &argc, args, NULL, NULL) != napi_ok || argc < 2) {
    napi_throw_type_error(env, NULL, "setAlpha expects (nativeWindowHandle, alpha)");
    return NULL;
  }

  void *data = NULL;
  size_t length = 0;
  if (napi_get_buffer_info(env, args[0], &data, &length) != napi_ok || length < sizeof(void *)) {
    napi_throw_type_error(env, NULL, "nativeWindowHandle must be a native window handle buffer");
    return NULL;
  }

  double alpha = 1.0;
  if (napi_get_value_double(env, args[1], &alpha) != napi_ok || !(alpha >= 0.0 && alpha <= 1.0)) {
    napi_throw_range_error(env, NULL, "alpha must be a number from 0 to 1");
    return NULL;
  }

  if (![NSThread isMainThread]) {
    napi_throw_error(env, NULL, "setAlpha must run on the main thread");
    return NULL;
  }

  NSView *view = (__bridge NSView *)(*(void **)data);
  NSWindow *window = view.window;
  if (window == nil) return NULL;

  for (NSInteger type = NSWindowCloseButton; type <= NSWindowZoomButton; type++) {
    [[window standardWindowButton:(NSWindowButton)type] setAlphaValue:alpha];
  }
  return NULL;
}

NAPI_MODULE_INIT() {
  napi_value setAlpha;
  if (napi_create_function(env, "setAlpha", NAPI_AUTO_LENGTH, SetAlpha, NULL, &setAlpha) != napi_ok) return NULL;
  if (napi_set_named_property(env, exports, "setAlpha", setAlpha) != napi_ok) return NULL;
  return exports;
}
