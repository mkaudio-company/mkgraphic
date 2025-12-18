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

use crate::support::point::Extent;
use crate::view::View;
use crate::element::ElementPtr;

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
}

impl Window {
    /// Creates a new window with the given title and size.
    pub fn new(title: impl Into<String>, size: Extent) -> Self {
        let title_str = title.into();

        #[cfg(target_os = "macos")]
        let macos_window = {
            MainThreadMarker::new().map(|mtm| MacOSWindow::new(&title_str, size, mtm))
        };

        Self {
            title: title_str,
            size,
            position: WindowPosition::default(),
            style: WindowStyle::default(),
            view: View::new(size),
            handle: None,
            #[cfg(target_os = "macos")]
            macos_window,
        }
    }

    /// Creates a new window with the given options.
    fn new_with_options(builder: WindowBuilder) -> Self {
        #[cfg(target_os = "macos")]
        let macos_window = {
            MainThreadMarker::new().map(|mtm| MacOSWindow::new(&builder.title, builder.size, mtm))
        };

        Self {
            title: builder.title,
            size: builder.size,
            position: builder.position,
            style: builder.style,
            view: View::new(builder.size),
            handle: None,
            #[cfg(target_os = "macos")]
            macos_window,
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
            win.set_content(content);
        }
    }

    /// Shows the window.
    pub fn show(&mut self) {
        #[cfg(target_os = "macos")]
        if let Some(ref win) = self.macos_window {
            win.show();
        }
    }

    /// Hides the window.
    pub fn hide(&mut self) {
        #[cfg(target_os = "macos")]
        if let Some(ref win) = self.macos_window {
            win.hide();
        }
    }

    /// Closes the window.
    pub fn close(&mut self) {
        #[cfg(target_os = "macos")]
        if let Some(ref win) = self.macos_window {
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
    }

    /// Returns the platform window handle.
    pub fn handle(&self) -> Option<WindowHandle> {
        self.handle
    }
}

/// The application.
pub struct App {
    running: bool,
    #[cfg(target_os = "macos")]
    macos_app: Option<MacOSApp>,
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
        #[cfg(not(target_os = "macos"))]
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
