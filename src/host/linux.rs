//! Linux platform implementation.
//!
//! This module provides the Linux-specific implementation using X11
//! through the x11rb crate.

#![cfg(target_os = "linux")]

use std::sync::Arc;

use x11rb::connection::Connection;
use x11rb::protocol::xproto::*;
use x11rb::protocol::Event;
use x11rb::rust_connection::RustConnection;
use x11rb::wrapper::ConnectionExt;
use x11rb::COPY_DEPTH_FROM_PARENT;

use crate::support::point::{Point, Extent};
use crate::view::{
    View, BaseView, MouseButton, MouseButtonKind, KeyCode, KeyAction, KeyInfo,
    TextInfo, CursorTracking, CursorType, DropInfo,
};

/// Translates an X11 keycode to our KeyCode enum.
pub fn translate_key(keycode: u8) -> KeyCode {
    // X11 keycodes are hardware-dependent, this is a simplified mapping
    // In practice, you'd use XKB for proper key translation
    match keycode {
        9 => KeyCode::Escape,
        10..=19 => {
            // 1-0 keys
            let idx = keycode - 10;
            match idx {
                0 => KeyCode::Key1,
                1 => KeyCode::Key2,
                2 => KeyCode::Key3,
                3 => KeyCode::Key4,
                4 => KeyCode::Key5,
                5 => KeyCode::Key6,
                6 => KeyCode::Key7,
                7 => KeyCode::Key8,
                8 => KeyCode::Key9,
                9 => KeyCode::Key0,
                _ => KeyCode::Unknown,
            }
        }
        22 => KeyCode::Backspace,
        23 => KeyCode::Tab,
        24 => KeyCode::Q,
        25 => KeyCode::W,
        26 => KeyCode::E,
        27 => KeyCode::R,
        28 => KeyCode::T,
        29 => KeyCode::Y,
        30 => KeyCode::U,
        31 => KeyCode::I,
        32 => KeyCode::O,
        33 => KeyCode::P,
        36 => KeyCode::Enter,
        37 => KeyCode::LeftControl,
        38 => KeyCode::A,
        39 => KeyCode::S,
        40 => KeyCode::D,
        41 => KeyCode::F,
        42 => KeyCode::G,
        43 => KeyCode::H,
        44 => KeyCode::J,
        45 => KeyCode::K,
        46 => KeyCode::L,
        50 => KeyCode::LeftShift,
        52 => KeyCode::Z,
        53 => KeyCode::X,
        54 => KeyCode::C,
        55 => KeyCode::V,
        56 => KeyCode::B,
        57 => KeyCode::N,
        58 => KeyCode::M,
        62 => KeyCode::RightShift,
        64 => KeyCode::LeftAlt,
        65 => KeyCode::Space,
        66 => KeyCode::CapsLock,
        67..=76 => {
            // F1-F10
            let idx = keycode - 67;
            match idx {
                0 => KeyCode::F1,
                1 => KeyCode::F2,
                2 => KeyCode::F3,
                3 => KeyCode::F4,
                4 => KeyCode::F5,
                5 => KeyCode::F6,
                6 => KeyCode::F7,
                7 => KeyCode::F8,
                8 => KeyCode::F9,
                9 => KeyCode::F10,
                _ => KeyCode::Unknown,
            }
        }
        95 => KeyCode::F11,
        96 => KeyCode::F12,
        105 => KeyCode::RightControl,
        108 => KeyCode::RightAlt,
        110 => KeyCode::Home,
        111 => KeyCode::Up,
        112 => KeyCode::PageUp,
        113 => KeyCode::Left,
        114 => KeyCode::Right,
        115 => KeyCode::End,
        116 => KeyCode::Down,
        117 => KeyCode::PageDown,
        118 => KeyCode::Insert,
        119 => KeyCode::Delete,
        133 => KeyCode::LeftSuper,
        134 => KeyCode::RightSuper,
        _ => KeyCode::Unknown,
    }
}

/// Translates X11 modifier state to our modifier bitmask.
pub fn translate_modifiers(state: u16) -> i32 {
    use crate::view::modifiers;

    let mut mods = 0i32;

    if state & 0x01 != 0 {
        // Shift
        mods |= modifiers::SHIFT;
    }
    if state & 0x04 != 0 {
        // Control
        mods |= modifiers::CONTROL;
    }
    if state & 0x08 != 0 {
        // Mod1 (Alt)
        mods |= modifiers::ALT;
    }
    if state & 0x40 != 0 {
        // Mod4 (Super)
        mods |= modifiers::SUPER;
    }
    if state & 0x02 != 0 {
        // Lock (Caps Lock)
        mods |= modifiers::CAPS_LOCK;
    }

    mods
}

/// Linux/X11 application wrapper.
pub struct LinuxApp {
    conn: Arc<RustConnection>,
    screen_num: usize,
    running: bool,
}

impl LinuxApp {
    /// Creates a new Linux application.
    pub fn new() -> Option<Self> {
        let (conn, screen_num) = RustConnection::connect(None).ok()?;
        Some(Self {
            conn: Arc::new(conn),
            screen_num,
            running: false,
        })
    }

    /// Returns the connection.
    pub fn connection(&self) -> &Arc<RustConnection> {
        &self.conn
    }

    /// Returns the screen number.
    pub fn screen_num(&self) -> usize {
        self.screen_num
    }

    /// Runs the application event loop.
    pub fn run(&mut self) {
        self.running = true;

        while self.running {
            match self.conn.wait_for_event() {
                Ok(event) => {
                    self.handle_event(event);
                }
                Err(_) => {
                    self.running = false;
                }
            }
        }
    }

    /// Stops the application.
    pub fn stop(&mut self) {
        self.running = false;
    }

    fn handle_event(&mut self, event: Event) {
        match event {
            Event::Expose(_) => {
                // Handle expose (redraw)
            }
            Event::ConfigureNotify(_) => {
                // Handle resize
            }
            Event::ButtonPress(e) => {
                // Handle mouse press
            }
            Event::ButtonRelease(e) => {
                // Handle mouse release
            }
            Event::MotionNotify(e) => {
                // Handle mouse motion
            }
            Event::KeyPress(e) => {
                // Handle key press
            }
            Event::KeyRelease(e) => {
                // Handle key release
            }
            Event::DestroyNotify(_) => {
                self.running = false;
            }
            _ => {}
        }
    }
}

/// Linux/X11 window wrapper.
pub struct LinuxWindow {
    conn: Arc<RustConnection>,
    window: Window,
    view: Option<View>,
}

impl LinuxWindow {
    /// Creates a new Linux window.
    pub fn new(app: &LinuxApp, title: &str, size: Extent) -> Option<Self> {
        let conn = app.connection().clone();
        let screen = &conn.setup().roots[app.screen_num()];

        let window = conn.generate_id().ok()?;

        let values = CreateWindowAux::default()
            .background_pixel(screen.white_pixel)
            .event_mask(
                EventMask::EXPOSURE
                    | EventMask::STRUCTURE_NOTIFY
                    | EventMask::BUTTON_PRESS
                    | EventMask::BUTTON_RELEASE
                    | EventMask::POINTER_MOTION
                    | EventMask::KEY_PRESS
                    | EventMask::KEY_RELEASE
                    | EventMask::ENTER_WINDOW
                    | EventMask::LEAVE_WINDOW
                    | EventMask::FOCUS_CHANGE,
            );

        conn.create_window(
            COPY_DEPTH_FROM_PARENT,
            window,
            screen.root,
            0,
            0,
            size.x as u16,
            size.y as u16,
            0,
            WindowClass::INPUT_OUTPUT,
            0,
            &values,
        )
        .ok()?;

        // Set window title
        conn.change_property8(
            PropMode::REPLACE,
            window,
            AtomEnum::WM_NAME,
            AtomEnum::STRING,
            title.as_bytes(),
        )
        .ok()?;

        conn.flush().ok()?;

        Some(Self {
            conn,
            window,
            view: Some(View::new(size)),
        })
    }

    /// Shows the window.
    pub fn show(&self) {
        let _ = self.conn.map_window(self.window);
        let _ = self.conn.flush();
    }

    /// Hides the window.
    pub fn hide(&self) {
        let _ = self.conn.unmap_window(self.window);
        let _ = self.conn.flush();
    }

    /// Closes the window.
    pub fn close(&self) {
        let _ = self.conn.destroy_window(self.window);
        let _ = self.conn.flush();
    }

    /// Returns the window ID.
    pub fn window_id(&self) -> Window {
        self.window
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
