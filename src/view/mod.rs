//! View module for window and view management.
//!
//! This module provides the View abstraction which represents a drawable surface
//! and handles user input events.

use std::collections::HashMap;
use crate::support::point::{Point, Extent};
use crate::support::rect::Rect;
use crate::support::canvas::Canvas;
use crate::element::{ElementPtr, ViewLimits};

/// Mouse button kind.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum MouseButtonKind {
    Left,
    Middle,
    Right,
}

/// Mouse button state.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MouseButtonState {
    Pressed,
    Released,
}

/// Mouse button event information.
#[derive(Debug, Clone, Copy)]
pub struct MouseButton {
    pub down: bool,
    pub click_count: i32,
    pub button: MouseButtonKind,
    pub modifiers: i32,
    pub pos: Point,
}

impl MouseButton {
    /// Creates a new mouse button event.
    pub fn new(down: bool, button: MouseButtonKind, pos: Point) -> Self {
        Self {
            down,
            click_count: 1,
            button,
            modifiers: 0,
            pos,
        }
    }
}

/// Key codes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum KeyCode {
    // Letters
    A, B, C, D, E, F, G, H, I, J, K, L, M,
    N, O, P, Q, R, S, T, U, V, W, X, Y, Z,

    // Numbers
    Key0, Key1, Key2, Key3, Key4,
    Key5, Key6, Key7, Key8, Key9,

    // Function keys
    F1, F2, F3, F4, F5, F6, F7, F8, F9, F10, F11, F12,

    // Navigation
    Up, Down, Left, Right,
    Home, End, PageUp, PageDown,

    // Editing
    Backspace, Delete, Insert,
    Enter, Tab, Escape,
    Space,

    // Modifiers
    Shift, Control, Alt, Super,
    LeftShift, RightShift,
    LeftControl, RightControl,
    LeftAlt, RightAlt,
    LeftSuper, RightSuper,

    // Other
    CapsLock, NumLock, ScrollLock,
    PrintScreen, Pause,
    Menu,

    Unknown,
}

/// Key action.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum KeyAction {
    Press,
    Release,
    Repeat,
}

/// Key event information.
#[derive(Debug, Clone, Copy)]
pub struct KeyInfo {
    pub key: KeyCode,
    pub action: KeyAction,
    pub modifiers: i32,
}

/// Text input information.
#[derive(Debug, Clone, Copy)]
pub struct TextInfo {
    pub codepoint: char,
    pub modifiers: i32,
}

/// Cursor tracking status.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CursorTracking {
    Entering,
    Hovering,
    Leaving,
}

/// Cursor type.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum CursorType {
    #[default]
    Arrow,
    IBeam,
    CrossHair,
    Hand,
    HResize,
    VResize,
}

/// Drop event information.
#[derive(Debug, Clone)]
pub struct DropInfo {
    pub where_: Point,
    pub data: HashMap<String, String>,
}

impl DropInfo {
    /// Creates a new drop info.
    pub fn new(pos: Point) -> Self {
        Self {
            where_: pos,
            data: HashMap::new(),
        }
    }
}

/// Modifier key flags.
pub mod modifiers {
    pub const SHIFT: i32 = 1 << 0;
    pub const CONTROL: i32 = 1 << 1;
    pub const ALT: i32 = 1 << 2;
    pub const SUPER: i32 = 1 << 3;
    pub const CAPS_LOCK: i32 = 1 << 4;
    pub const NUM_LOCK: i32 = 1 << 5;

    /// Platform-specific action modifier (Cmd on macOS, Ctrl on Windows/Linux).
    #[cfg(target_os = "macos")]
    pub const ACTION: i32 = SUPER;
    #[cfg(not(target_os = "macos"))]
    pub const ACTION: i32 = CONTROL;
}

/// Base view trait for platform-specific implementations.
pub trait BaseView {
    /// Draws the view content.
    fn draw(&mut self, canvas: &mut Canvas);

    /// Handles mouse click events.
    fn click(&mut self, btn: MouseButton);

    /// Handles mouse drag events.
    fn drag(&mut self, btn: MouseButton);

    /// Handles cursor movement events.
    fn cursor(&mut self, p: Point, status: CursorTracking);

    /// Handles scroll events.
    fn scroll(&mut self, dir: Point, p: Point);

    /// Handles key events.
    fn key(&mut self, k: KeyInfo) -> bool;

    /// Handles text input events.
    fn text(&mut self, info: TextInfo) -> bool;

    /// Called when the view gains focus.
    fn begin_focus(&mut self);

    /// Called when the view loses focus.
    fn end_focus(&mut self);

    /// Handles drop tracking events.
    fn track_drop(&mut self, info: &DropInfo, status: CursorTracking);

    /// Handles drop events.
    fn drop(&mut self, info: &DropInfo) -> bool;

    /// Called periodically for idle processing.
    fn poll(&mut self);
}

/// The main view struct that manages the UI content.
pub struct View {
    bounds: Rect,
    cursor_pos: Point,
    scale: f32,
    content: Option<ElementPtr>,
    is_focus: bool,
}

impl View {
    /// Creates a new view with the given size.
    pub fn new(size: Extent) -> Self {
        Self {
            bounds: Rect::from_origin_size(Point::zero(), size),
            cursor_pos: Point::zero(),
            scale: 1.0,
            content: None,
            is_focus: false,
        }
    }

    /// Returns the view bounds.
    pub fn bounds(&self) -> Rect {
        self.bounds
    }

    /// Returns the view size.
    pub fn size(&self) -> Extent {
        self.bounds.size()
    }

    /// Sets the view size.
    pub fn set_size(&mut self, size: Extent) {
        self.bounds = Rect::from_origin_size(Point::zero(), size);
    }

    /// Returns the current cursor position.
    pub fn cursor_pos(&self) -> Point {
        self.cursor_pos
    }

    /// Returns the current scale factor.
    pub fn scale(&self) -> f32 {
        self.scale
    }

    /// Sets the scale factor.
    pub fn set_scale(&mut self, scale: f32) {
        self.scale = scale;
    }

    /// Sets the view content.
    pub fn set_content(&mut self, content: ElementPtr) {
        self.content = Some(content);
    }

    /// Returns the content element.
    pub fn content(&self) -> Option<&ElementPtr> {
        self.content.as_ref()
    }

    /// Returns the view limits based on content.
    pub fn limits(&self) -> ViewLimits {
        // Would need to query content limits
        ViewLimits::full()
    }

    /// Returns whether the view has focus.
    pub fn has_focus(&self) -> bool {
        self.is_focus
    }

    /// Triggers a refresh of the entire view.
    pub fn refresh(&self) {
        // Platform-specific implementation would trigger redraw
    }

    /// Triggers a refresh of a specific area.
    pub fn refresh_area(&self, area: Rect) {
        // Platform-specific implementation would trigger partial redraw
    }
}

impl BaseView for View {
    fn draw(&mut self, canvas: &mut Canvas) {
        // Draw content if present
        if let Some(content) = &self.content {
            // Would create context and draw
        }
    }

    fn click(&mut self, btn: MouseButton) {
        // Dispatch to content
    }

    fn drag(&mut self, btn: MouseButton) {
        // Dispatch to content
    }

    fn cursor(&mut self, p: Point, status: CursorTracking) {
        self.cursor_pos = p;
        // Dispatch to content
    }

    fn scroll(&mut self, dir: Point, p: Point) {
        // Dispatch to content
    }

    fn key(&mut self, k: KeyInfo) -> bool {
        // Dispatch to content
        false
    }

    fn text(&mut self, info: TextInfo) -> bool {
        // Dispatch to content
        false
    }

    fn begin_focus(&mut self) {
        self.is_focus = true;
    }

    fn end_focus(&mut self) {
        self.is_focus = false;
    }

    fn track_drop(&mut self, info: &DropInfo, status: CursorTracking) {
        // Dispatch to content
    }

    fn drop(&mut self, info: &DropInfo) -> bool {
        // Dispatch to content
        false
    }

    fn poll(&mut self) {
        // Process any pending async tasks
    }
}

/// Gets the clipboard contents.
pub fn clipboard() -> String {
    // Platform-specific implementation
    String::new()
}

/// Sets the clipboard contents.
pub fn set_clipboard(text: &str) {
    // Platform-specific implementation
}

/// Sets the cursor type.
pub fn set_cursor(cursor: CursorType) {
    // Platform-specific implementation
}

/// Returns the scroll direction preference (1.0 or -1.0).
pub fn scroll_direction() -> Point {
    Point::new(1.0, 1.0)
}
