//! macOS platform implementation.
//!
//! This module provides the macOS-specific implementation using Cocoa/AppKit
//! through the objc2 crate.

#![cfg(target_os = "macos")]

use objc2::rc::Retained;
use objc2_foundation::{
    NSString, MainThreadMarker, NSPoint, NSRect, NSSize,
};
use objc2_app_kit::{
    NSApplication, NSApplicationActivationPolicy, NSBackingStoreType,
    NSWindow, NSWindowStyleMask, NSCursor, NSPasteboard,
};

use crate::support::point::{Point, Extent};
use crate::view::{View, KeyCode, CursorType, modifiers};

/// Converts NSPoint to our Point type.
fn ns_point_to_point(p: NSPoint) -> Point {
    Point::new(p.x as f32, p.y as f32)
}

/// Converts our Point type to NSPoint.
fn point_to_ns_point(p: Point) -> NSPoint {
    NSPoint::new(p.x as f64, p.y as f64)
}

/// Converts NSSize to our Extent type.
fn ns_size_to_extent(s: NSSize) -> Extent {
    Extent::new(s.width as f32, s.height as f32)
}

/// Converts our Extent type to NSSize.
fn extent_to_ns_size(e: Extent) -> NSSize {
    NSSize::new(e.x as f64, e.y as f64)
}

/// Translates a macOS key code to our KeyCode enum.
pub fn translate_key(keycode: u16) -> KeyCode {
    match keycode {
        0x00 => KeyCode::A,
        0x01 => KeyCode::S,
        0x02 => KeyCode::D,
        0x03 => KeyCode::F,
        0x04 => KeyCode::H,
        0x05 => KeyCode::G,
        0x06 => KeyCode::Z,
        0x07 => KeyCode::X,
        0x08 => KeyCode::C,
        0x09 => KeyCode::V,
        0x0B => KeyCode::B,
        0x0C => KeyCode::Q,
        0x0D => KeyCode::W,
        0x0E => KeyCode::E,
        0x0F => KeyCode::R,
        0x10 => KeyCode::Y,
        0x11 => KeyCode::T,
        0x12 => KeyCode::Key1,
        0x13 => KeyCode::Key2,
        0x14 => KeyCode::Key3,
        0x15 => KeyCode::Key4,
        0x16 => KeyCode::Key6,
        0x17 => KeyCode::Key5,
        0x19 => KeyCode::Key9,
        0x1A => KeyCode::Key7,
        0x1C => KeyCode::Key8,
        0x1D => KeyCode::Key0,
        0x1F => KeyCode::O,
        0x20 => KeyCode::U,
        0x22 => KeyCode::I,
        0x23 => KeyCode::P,
        0x25 => KeyCode::L,
        0x26 => KeyCode::J,
        0x28 => KeyCode::K,
        0x2D => KeyCode::N,
        0x2E => KeyCode::M,
        0x24 => KeyCode::Enter,
        0x30 => KeyCode::Tab,
        0x31 => KeyCode::Space,
        0x33 => KeyCode::Backspace,
        0x35 => KeyCode::Escape,
        0x37 => KeyCode::LeftSuper,
        0x38 => KeyCode::LeftShift,
        0x39 => KeyCode::CapsLock,
        0x3A => KeyCode::LeftAlt,
        0x3B => KeyCode::LeftControl,
        0x3C => KeyCode::RightShift,
        0x3D => KeyCode::RightAlt,
        0x3E => KeyCode::RightControl,
        0x60 => KeyCode::F5,
        0x61 => KeyCode::F6,
        0x62 => KeyCode::F7,
        0x63 => KeyCode::F3,
        0x64 => KeyCode::F8,
        0x65 => KeyCode::F9,
        0x67 => KeyCode::F11,
        0x6D => KeyCode::F10,
        0x6F => KeyCode::F12,
        0x72 => KeyCode::Insert,
        0x73 => KeyCode::Home,
        0x74 => KeyCode::PageUp,
        0x75 => KeyCode::Delete,
        0x76 => KeyCode::F4,
        0x77 => KeyCode::End,
        0x78 => KeyCode::F2,
        0x79 => KeyCode::PageDown,
        0x7A => KeyCode::F1,
        0x7B => KeyCode::Left,
        0x7C => KeyCode::Right,
        0x7D => KeyCode::Down,
        0x7E => KeyCode::Up,
        _ => KeyCode::Unknown,
    }
}

/// Translates macOS modifier flags to our modifier bitmask.
pub fn translate_flags(flags: usize) -> i32 {
    let mut mods = 0i32;

    if flags & (1 << 17) != 0 {
        // NSEventModifierFlagShift
        mods |= modifiers::SHIFT;
    }
    if flags & (1 << 18) != 0 {
        // NSEventModifierFlagControl
        mods |= modifiers::CONTROL;
    }
    if flags & (1 << 19) != 0 {
        // NSEventModifierFlagOption (Alt)
        mods |= modifiers::ALT;
    }
    if flags & (1 << 20) != 0 {
        // NSEventModifierFlagCommand
        mods |= modifiers::SUPER;
    }
    if flags & (1 << 16) != 0 {
        // NSEventModifierFlagCapsLock
        mods |= modifiers::CAPS_LOCK;
    }

    mods
}

/// Sets the cursor type.
///
/// # Safety
/// This function calls Objective-C methods which require running on the main thread.
pub fn set_cursor(cursor: CursorType) {
    unsafe {
        match cursor {
            CursorType::Arrow => {
                let cursor = NSCursor::arrowCursor();
                cursor.set();
            }
            CursorType::IBeam => {
                let cursor = NSCursor::IBeamCursor();
                cursor.set();
            }
            CursorType::CrossHair => {
                let cursor = NSCursor::crosshairCursor();
                cursor.set();
            }
            CursorType::Hand => {
                let cursor = NSCursor::openHandCursor();
                cursor.set();
            }
            CursorType::HResize => {
                let cursor = NSCursor::resizeLeftRightCursor();
                cursor.set();
            }
            CursorType::VResize => {
                let cursor = NSCursor::resizeUpDownCursor();
                cursor.set();
            }
        }
    }
}

/// Gets the clipboard contents.
pub fn get_clipboard() -> String {
    unsafe {
        let _pasteboard = NSPasteboard::generalPasteboard();
        // Would need to read string from pasteboard
        String::new()
    }
}

/// Sets the clipboard contents.
pub fn set_clipboard(_text: &str) {
    unsafe {
        let _pasteboard = NSPasteboard::generalPasteboard();
        // Would need to write string to pasteboard
    }
}

/// macOS application wrapper.
pub struct MacOSApp {
    app: Retained<NSApplication>,
    mtm: MainThreadMarker,
}

impl MacOSApp {
    /// Creates a new macOS application.
    pub fn new() -> Option<Self> {
        let mtm = MainThreadMarker::new()?;

        let app = NSApplication::sharedApplication(mtm);
        app.setActivationPolicy(NSApplicationActivationPolicy::Regular);

        Some(Self { app, mtm })
    }

    /// Runs the application event loop.
    pub fn run(&self) {
        unsafe {
            self.app.run();
        }
    }

    /// Stops the application.
    pub fn stop(&self) {
        self.app.stop(None);
    }
}

/// macOS window wrapper.
pub struct MacOSWindow {
    window: Retained<NSWindow>,
    view: Option<View>,
}

impl MacOSWindow {
    /// Creates a new macOS window.
    pub fn new(title: &str, size: Extent, mtm: MainThreadMarker) -> Self {
        let frame = NSRect::new(
            NSPoint::new(0.0, 0.0),
            extent_to_ns_size(size),
        );

        let style = NSWindowStyleMask::Titled
            | NSWindowStyleMask::Closable
            | NSWindowStyleMask::Miniaturizable
            | NSWindowStyleMask::Resizable;

        let window = unsafe {
            NSWindow::initWithContentRect_styleMask_backing_defer(
                mtm.alloc(),
                frame,
                style,
                NSBackingStoreType::NSBackingStoreBuffered,
                false,
            )
        };

        let title_str = NSString::from_str(title);
        window.setTitle(&title_str);
        window.center();

        Self {
            window,
            view: Some(View::new(size)),
        }
    }

    /// Shows the window.
    pub fn show(&self) {
        self.window.makeKeyAndOrderFront(None);
    }

    /// Hides the window.
    pub fn hide(&self) {
        self.window.orderOut(None);
    }

    /// Closes the window.
    pub fn close(&self) {
        self.window.close();
    }

    /// Sets the window title.
    pub fn set_title(&self, title: &str) {
        let title_str = NSString::from_str(title);
        self.window.setTitle(&title_str);
    }

    /// Returns the window size.
    pub fn size(&self) -> Extent {
        let frame = self.window.frame();
        ns_size_to_extent(frame.size)
    }

    /// Sets the window size.
    pub fn set_size(&self, size: Extent) {
        let mut frame = self.window.frame();
        frame.size = extent_to_ns_size(size);
        self.window.setFrame_display(frame, true);
    }

    /// Returns a reference to the view.
    pub fn view(&self) -> Option<&View> {
        self.view.as_ref()
    }

    /// Returns a mutable reference to the view.
    pub fn view_mut(&mut self) -> Option<&mut View> {
        self.view.as_mut()
    }
}
