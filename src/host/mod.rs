//! Host module for platform-specific implementations.
//!
//! This module provides the platform abstraction layer for creating windows
//! and running the application event loop.

#[cfg(target_os = "macos")]
mod macos;

#[cfg(target_os = "windows")]
mod windows;

#[cfg(target_os = "linux")]
mod linux;

#[cfg(target_os = "macos")]
pub use macos::{MacOSApp, MacOSWindow};

#[cfg(target_os = "windows")]
pub use self::windows::{WindowsApp, WindowsTimer, WindowsWindow};

#[cfg(target_os = "linux")]
pub use self::linux::{LinuxApp, LinuxTimer, LinuxWindow};

use crate::element::ElementPtr;
use crate::support::point::Extent;
use crate::view::View;

#[cfg(target_os = "macos")]
use objc2_foundation::MainThreadMarker;

/// Window position.
#[derive(Debug, Clone, Copy)]
pub struct WindowPosition {
    pub x: i32,
    pub y: i32,
}

impl WindowPosition {
    /// Creates a new window position.
    pub fn new(x: i32, y: i32) -> Self {
        Self { x, y }
    }

    /// Center the window on screen.
    pub fn center() -> Self {
        Self { x: -1, y: -1 } // Sentinel value for centering
    }
}

impl Default for WindowPosition {
    fn default() -> Self {
        Self::center()
    }
}

/// Window style flags.
#[derive(Debug, Clone, Copy)]
pub struct WindowStyle {
    pub closable: bool,
    pub miniaturizable: bool,
    pub resizable: bool,
    pub borderless: bool,
}

impl Default for WindowStyle {
    fn default() -> Self {
        Self {
            closable: true,
            miniaturizable: true,
            resizable: true,
            borderless: false,
        }
    }
}

impl WindowStyle {
    /// Creates a borderless window style.
    pub fn borderless() -> Self {
        Self {
            closable: false,
            miniaturizable: false,
            resizable: false,
            borderless: true,
        }
    }
}

/// Window handle type (platform-specific).
pub type WindowHandle = *mut std::ffi::c_void;

/// View handle type (platform-specific).
pub type ViewHandle = *mut std::ffi::c_void;

/// Window builder for creating windows with various options.
pub struct WindowBuilder {
    title: String,
    size: Extent,
    position: WindowPosition,
    style: WindowStyle,
    min_size: Option<Extent>,
    max_size: Option<Extent>,
}

impl WindowBuilder {
    /// Creates a new window builder with the given title and size.
    pub fn new(title: impl Into<String>, size: Extent) -> Self {
        Self {
            title: title.into(),
            size,
            position: WindowPosition::default(),
            style: WindowStyle::default(),
            min_size: None,
            max_size: None,
        }
    }

    /// Sets the window position.
    pub fn position(mut self, pos: WindowPosition) -> Self {
        self.position = pos;
        self
    }

    /// Sets the window style.
    pub fn style(mut self, style: WindowStyle) -> Self {
        self.style = style;
        self
    }

    /// Sets the minimum size.
    pub fn min_size(mut self, size: Extent) -> Self {
        self.min_size = Some(size);
        self
    }

    /// Sets the maximum size.
    pub fn max_size(mut self, size: Extent) -> Self {
        self.max_size = Some(size);
        self
    }

    /// Builds the window.
    pub fn build(self) -> Window {
        Window::new_with_options(self)
    }
}

/// A platform window.
pub struct Window {
    title: String,
    size: Extent,
    position: WindowPosition,
    style: WindowStyle,
    view: View,
    handle: Option<WindowHandle>,
    #[cfg(target_os = "macos")]
    macos_window: Option<MacOSWindow>,
    #[cfg(target_os = "windows")]
    windows_window: Option<WindowsWindow>,
    #[cfg(target_os = "linux")]
    linux_window: Option<LinuxWindow>,
}

impl Window {
    /// Creates a new window with the given title and size.
    pub fn new(title: impl Into<String>, size: Extent) -> Self {
        let title_str = title.into();

        #[cfg(target_os = "macos")]
        let macos_window =
            { MainThreadMarker::new().map(|mtm| MacOSWindow::new(&title_str, size, mtm)) };
        #[cfg(target_os = "windows")]
        let windows_window = WindowsWindow::new(&title_str, size);
        #[cfg(target_os = "linux")]
        let linux_window = LinuxWindow::new(&title_str, size);

        Self {
            title: title_str,
            size,
            position: WindowPosition::default(),
            style: WindowStyle::default(),
            view: View::new(size),
            handle: None,
            #[cfg(target_os = "macos")]
            macos_window,
            #[cfg(target_os = "windows")]
            windows_window,
            #[cfg(target_os = "linux")]
            linux_window,
        }
    }

    /// Creates a new window with the given options.
    fn new_with_options(builder: WindowBuilder) -> Self {
        #[cfg(target_os = "macos")]
        let macos_window = {
            MainThreadMarker::new().map(|mtm| MacOSWindow::new(&builder.title, builder.size, mtm))
        };
        #[cfg(target_os = "windows")]
        let windows_window = WindowsWindow::new(&builder.title, builder.size);
        #[cfg(target_os = "linux")]
        let linux_window = LinuxWindow::new(&builder.title, builder.size);

        Self {
            title: builder.title,
            size: builder.size,
            position: builder.position,
            style: builder.style,
            view: View::new(builder.size),
            handle: None,
            #[cfg(target_os = "macos")]
            macos_window,
            #[cfg(target_os = "windows")]
            windows_window,
            #[cfg(target_os = "linux")]
            linux_window,
        }
    }

    /// Returns the window title.
    pub fn title(&self) -> &str {
        &self.title
    }

    /// Sets the window title.
    pub fn set_title(&mut self, title: impl Into<String>) {
        self.title = title.into();
        #[cfg(target_os = "macos")]
        if let Some(ref win) = self.macos_window {
            win.set_title(&self.title);
        }
        #[cfg(target_os = "windows")]
        if let Some(ref win) = self.windows_window {
            win.set_title(&self.title);
        }
        #[cfg(target_os = "linux")]
        if let Some(ref win) = self.linux_window {
            win.set_title(&self.title);
        }
    }

    /// Returns the window size.
    pub fn size(&self) -> Extent {
        self.size
    }

    /// Sets the window size.
    pub fn set_size(&mut self, size: Extent) {
        self.size = size;
        self.view.set_size(size);
        #[cfg(target_os = "macos")]
        if let Some(ref win) = self.macos_window {
            win.set_size(size);
        }
        #[cfg(target_os = "windows")]
        if let Some(ref win) = self.windows_window {
            win.set_size(size);
        }
        #[cfg(target_os = "linux")]
        if let Some(ref win) = self.linux_window {
            win.set_size(size);
        }
    }

    /// Returns the window position.
    pub fn position(&self) -> WindowPosition {
        self.position
    }

    /// Sets the window position.
    pub fn set_position(&mut self, pos: WindowPosition) {
        self.position = pos;
    }

    /// Returns a reference to the view.
    pub fn view(&self) -> &View {
        &self.view
    }

    /// Returns a mutable reference to the view.
    pub fn view_mut(&mut self) -> &mut View {
        &mut self.view
    }

    /// Sets the window content.
    pub fn set_content(&mut self, content: ElementPtr) {
        self.view.set_content(content.clone());
        #[cfg(target_os = "macos")]
        if let Some(ref win) = self.macos_window {
            win.set_content(content.clone());
        }
        #[cfg(target_os = "windows")]
        if let Some(ref win) = self.windows_window {
            win.set_content(content.clone());
        }
        #[cfg(target_os = "linux")]
        if let Some(ref win) = self.linux_window {
            win.set_content(content);
        }
    }

    /// Shows the window.
    pub fn show(&mut self) {
        #[cfg(target_os = "macos")]
        if let Some(ref win) = self.macos_window {
            win.show();
        }
        #[cfg(target_os = "windows")]
        if let Some(ref win) = self.windows_window {
            win.show();
        }
        #[cfg(target_os = "linux")]
        if let Some(ref win) = self.linux_window {
            win.show();
        }
    }

    /// Hides the window.
    pub fn hide(&mut self) {
        #[cfg(target_os = "macos")]
        if let Some(ref win) = self.macos_window {
            win.hide();
        }
        #[cfg(target_os = "windows")]
        if let Some(ref win) = self.windows_window {
            win.hide();
        }
        #[cfg(target_os = "linux")]
        if let Some(ref win) = self.linux_window {
            win.hide();
        }
    }

    /// Closes the window.
    pub fn close(&mut self) {
        #[cfg(target_os = "macos")]
        if let Some(ref win) = self.macos_window {
            win.close();
        }
        #[cfg(target_os = "windows")]
        if let Some(ref win) = self.windows_window {
            win.close();
        }
        #[cfg(target_os = "linux")]
        if let Some(ref win) = self.linux_window {
            win.close();
        }
    }

    /// Returns whether the window is visible.
    pub fn is_visible(&self) -> bool {
        true // Placeholder
    }

    /// Triggers a refresh of the window.
    pub fn refresh(&self) {
        self.view.refresh();
        // `View::refresh` above is a platform-agnostic no-op stub; the
        // macOS backend's actual `setNeedsDisplay` call lives on
        // `MacOSWindow` instead, same as every other method here that
        // forwards to it. Without this, nothing calling `Window::refresh`
        // (e.g. a timer callback updating a widget's text outside of any
        // mouse/key event) ever produced a visible repaint -- confirmed by
        // running the `timer` example and observing the status bar's text
        // freeze after whatever the first incidental repaint happened to
        // catch.
        #[cfg(target_os = "macos")]
        if let Some(ref win) = self.macos_window {
            win.refresh();
        }
        #[cfg(target_os = "windows")]
        if let Some(ref win) = self.windows_window {
            win.refresh();
        }
        #[cfg(target_os = "linux")]
        if let Some(ref win) = self.linux_window {
            win.refresh();
        }
    }

    /// Returns the platform native window handle (macOS: `NSWindow*`,
    /// Windows: `HWND`, Linux: X11 window ID), for embedding
    /// externally-managed content into the window instead of using
    /// mkgraphic's own element tree for it.
    pub fn handle(&self) -> Option<WindowHandle> {
        #[cfg(target_os = "macos")]
        {
            self.macos_window
                .as_ref()
                .map(|window| window.native_window_handle())
        }
        #[cfg(target_os = "windows")]
        {
            self.windows_window
                .as_ref()
                .map(|window| window.native_window_handle())
        }
        #[cfg(target_os = "linux")]
        {
            self.linux_window
                .as_ref()
                .map(|window| window.native_window_handle())
        }
        #[cfg(not(any(target_os = "macos", target_os = "windows", target_os = "linux")))]
        {
            self.handle
        }
    }
}

/// The application.
pub struct App {
    running: bool,
    #[cfg(target_os = "macos")]
    macos_app: Option<MacOSApp>,
    #[cfg(target_os = "windows")]
    windows_app: Option<WindowsApp>,
    #[cfg(target_os = "linux")]
    linux_app: Option<LinuxApp>,
}

impl App {
    /// Creates a new application.
    pub fn new() -> Self {
        #[cfg(target_os = "macos")]
        {
            Self {
                running: false,
                macos_app: MacOSApp::new(),
            }
        }
        #[cfg(target_os = "windows")]
        {
            Self {
                running: false,
                windows_app: WindowsApp::new(),
            }
        }
        #[cfg(target_os = "linux")]
        {
            Self {
                running: false,
                linux_app: LinuxApp::new(),
            }
        }
        #[cfg(not(any(target_os = "macos", target_os = "windows", target_os = "linux")))]
        {
            Self { running: false }
        }
    }

    /// Runs the application event loop.
    pub fn run(&mut self) {
        self.running = true;
        #[cfg(target_os = "macos")]
        {
            if let Some(ref app) = self.macos_app {
                app.run();
            }
        }
        #[cfg(target_os = "windows")]
        {
            if let Some(ref app) = self.windows_app {
                app.run();
            }
        }
        #[cfg(target_os = "linux")]
        {
            if let Some(ref mut app) = self.linux_app {
                app.run();
            }
        }
    }

    /// Stops the application.
    pub fn stop(&mut self) {
        self.running = false;
        #[cfg(target_os = "macos")]
        {
            if let Some(ref app) = self.macos_app {
                app.stop();
            }
        }
        #[cfg(target_os = "windows")]
        {
            if let Some(ref app) = self.windows_app {
                app.stop();
            }
        }
        #[cfg(target_os = "linux")]
        {
            if let Some(ref mut app) = self.linux_app {
                app.stop();
            }
        }
    }

    /// Returns whether the application is running.
    pub fn is_running(&self) -> bool {
        self.running
    }

    /// Returns the main thread marker (macOS only).
    #[cfg(target_os = "macos")]
    pub fn main_thread_marker(&self) -> Option<MainThreadMarker> {
        MainThreadMarker::new()
    }

    /// Schedules `callback` to run repeatedly on the main thread every
    /// `interval_secs` seconds, for as long as the returned [`Timer`] is
    /// kept alive (dropping it stops the callback -- see [`Timer`]'s own
    /// docs). This is mkgraphic's first timer/idle-callback primitive:
    /// previously there was no way for an app to update UI state on a
    /// schedule rather than in direct response to an event the platform
    /// backend was already dispatching (macOS's AppKit backend in
    /// particular only repaints from inside its own mouse/key handlers),
    /// which made e.g. streaming live subprocess output or auto-polling a
    /// language server's diagnostics impossible without blocking the UI
    /// thread until the work finished.
    ///
    /// Covers macOS, Windows, and Linux/X11.
    #[cfg(any(target_os = "macos", target_os = "windows", target_os = "linux"))]
    pub fn schedule_timer(&self, interval_secs: f64, callback: impl FnMut() + 'static) -> Timer {
        #[cfg(target_os = "macos")]
        let inner = self
            .macos_app
            .as_ref()
            .expect("App::new should have created a MacOSApp")
            .schedule_timer(interval_secs, true, callback);
        #[cfg(target_os = "windows")]
        let inner = self
            .windows_app
            .as_ref()
            .expect("App::new should have created a WindowsApp")
            .schedule_timer(interval_secs, true, callback);
        #[cfg(target_os = "linux")]
        let inner = self
            .linux_app
            .as_ref()
            .expect("App::new should have created a LinuxApp")
            .schedule_timer(interval_secs, true, callback);
        Timer { inner }
    }

    /// Runs `callback` once, the next time the main run loop turns (a
    /// zero-delay, non-repeating timer -- the "idle callback" half of this
    /// primitive). Unlike [`Self::schedule_timer`], the caller doesn't need
    /// to hold on to anything: the timer invalidates itself immediately
    /// after firing once.
    #[cfg(any(target_os = "macos", target_os = "windows", target_os = "linux"))]
    pub fn schedule_once(&self, callback: impl FnOnce() + 'static) {
        // `schedule_timer` takes `FnMut`; wrap the `FnOnce` in an `Option`
        // so it can be called through a `&mut self` closure while only
        // ever actually running the inner callback the one time it fires
        // (`repeats: false`, so there's no second call to worry about, but
        // `FnMut`'s type still requires something callable more than once
        // in principle).
        let mut callback = Some(callback);
        #[cfg(target_os = "macos")]
        let inner = self
            .macos_app
            .as_ref()
            .expect("App::new should have created a MacOSApp")
            .schedule_timer(0.0, false, move || {
                if let Some(callback) = callback.take() {
                    callback();
                }
            });
        #[cfg(target_os = "windows")]
        let inner = self
            .windows_app
            .as_ref()
            .expect("App::new should have created a WindowsApp")
            .schedule_timer(0.0, false, move || {
                if let Some(callback) = callback.take() {
                    callback();
                }
            });
        #[cfg(target_os = "linux")]
        let inner = self
            .linux_app
            .as_ref()
            .expect("App::new should have created a LinuxApp")
            .schedule_timer(0.0, false, move || {
                if let Some(callback) = callback.take() {
                    callback();
                }
            });
        // Intentionally leaked: a one-shot timer has no handle for the
        // caller to hold, and it invalidates (and the run loop drops its
        // reference to it) right after firing once on its own.
        std::mem::forget(inner);
    }
}

/// A handle to a [`App::schedule_timer`] callback. Dropping this (or
/// calling [`Self::cancel`]) stops future firings -- the timer is not kept
/// alive by anything else once this handle is gone, so letting it drop
/// (e.g. a local variable going out of scope) is a real, if easy to miss,
/// way to stop a timer.
#[cfg(target_os = "macos")]
pub struct Timer {
    inner: objc2::rc::Retained<objc2_foundation::NSTimer>,
}

#[cfg(target_os = "windows")]
pub struct Timer {
    inner: WindowsTimer,
}

#[cfg(target_os = "linux")]
pub struct Timer {
    inner: LinuxTimer,
}

#[cfg(target_os = "macos")]
impl Timer {
    /// Stops future firings. Also happens automatically on drop.
    pub fn cancel(&self) {
        unsafe {
            self.inner.invalidate();
        }
    }
}

#[cfg(target_os = "windows")]
impl Timer {
    /// Stops future firings. Also happens automatically on drop.
    pub fn cancel(&self) {
        self.inner.cancel();
    }
}

#[cfg(target_os = "linux")]
impl Timer {
    /// Stops future firings. Also happens automatically on drop.
    pub fn cancel(&self) {
        self.inner.cancel();
    }
}

#[cfg(any(target_os = "macos", target_os = "windows", target_os = "linux"))]
impl Drop for Timer {
    fn drop(&mut self) {
        self.cancel();
    }
}

impl Default for App {
    fn default() -> Self {
        Self::new()
    }
}

/// Error type for platform operations.
#[derive(Debug, thiserror::Error)]
pub enum PlatformError {
    #[error("Failed to create window: {0}")]
    WindowCreation(String),

    #[error("Failed to initialize application: {0}")]
    Initialization(String),

    #[error("Platform error: {0}")]
    Other(String),
}

/// Result type for platform operations.
pub type PlatformResult<T> = Result<T, PlatformError>;
