//! Windows platform implementation.
//!
//! This module provides the Windows-specific implementation using Win32 API
//! through the windows crate.

#![cfg(target_os = "windows")]

use std::ffi::c_void;
use std::mem;
use std::ptr;

use windows::core::{PCWSTR, w};
use windows::Win32::Foundation::{HWND, LPARAM, LRESULT, WPARAM, RECT, POINT};
use windows::Win32::Graphics::Gdi::{
    BeginPaint, EndPaint, InvalidateRect, PAINTSTRUCT, GetDC, ReleaseDC,
};
use windows::Win32::System::LibraryLoader::GetModuleHandleW;
use windows::Win32::UI::WindowsAndMessaging::{
    CreateWindowExW, DefWindowProcW, DispatchMessageW, GetMessageW,
    LoadCursorW, PostQuitMessage, RegisterClassW, ShowWindow, TranslateMessage,
    UpdateWindow, CS_HREDRAW, CS_VREDRAW, CW_USEDEFAULT, IDC_ARROW,
    MSG, SW_SHOW, WM_DESTROY, WM_PAINT, WM_SIZE, WM_LBUTTONDOWN,
    WM_LBUTTONUP, WM_RBUTTONDOWN, WM_RBUTTONUP, WM_MBUTTONDOWN, WM_MBUTTONUP,
    WM_MOUSEMOVE, WM_MOUSEWHEEL, WM_KEYDOWN, WM_KEYUP, WM_CHAR,
    WNDCLASSW, WS_OVERLAPPEDWINDOW, GetWindowRect, SetWindowPos,
    SWP_NOZORDER, SWP_NOMOVE, WINDOW_EX_STYLE, SetCursor,
    IDC_IBEAM, IDC_CROSS, IDC_HAND, IDC_SIZEWE, IDC_SIZENS,
};
use windows::Win32::UI::Input::KeyboardAndMouse::{
    GetKeyState, VK_SHIFT, VK_CONTROL, VK_MENU, VK_LWIN, VK_CAPITAL,
};

use crate::support::point::{Point, Extent};
use crate::view::{
    View, BaseView, MouseButton, MouseButtonKind, KeyCode, KeyAction, KeyInfo,
    TextInfo, CursorTracking, CursorType, DropInfo,
};

/// Translates a Windows virtual key code to our KeyCode enum.
pub fn translate_key(vk: i32) -> KeyCode {
    match vk {
        0x41 => KeyCode::A,
        0x42 => KeyCode::B,
        0x43 => KeyCode::C,
        0x44 => KeyCode::D,
        0x45 => KeyCode::E,
        0x46 => KeyCode::F,
        0x47 => KeyCode::G,
        0x48 => KeyCode::H,
        0x49 => KeyCode::I,
        0x4A => KeyCode::J,
        0x4B => KeyCode::K,
        0x4C => KeyCode::L,
        0x4D => KeyCode::M,
        0x4E => KeyCode::N,
        0x4F => KeyCode::O,
        0x50 => KeyCode::P,
        0x51 => KeyCode::Q,
        0x52 => KeyCode::R,
        0x53 => KeyCode::S,
        0x54 => KeyCode::T,
        0x55 => KeyCode::U,
        0x56 => KeyCode::V,
        0x57 => KeyCode::W,
        0x58 => KeyCode::X,
        0x59 => KeyCode::Y,
        0x5A => KeyCode::Z,
        0x30 => KeyCode::Key0,
        0x31 => KeyCode::Key1,
        0x32 => KeyCode::Key2,
        0x33 => KeyCode::Key3,
        0x34 => KeyCode::Key4,
        0x35 => KeyCode::Key5,
        0x36 => KeyCode::Key6,
        0x37 => KeyCode::Key7,
        0x38 => KeyCode::Key8,
        0x39 => KeyCode::Key9,
        0x70 => KeyCode::F1,
        0x71 => KeyCode::F2,
        0x72 => KeyCode::F3,
        0x73 => KeyCode::F4,
        0x74 => KeyCode::F5,
        0x75 => KeyCode::F6,
        0x76 => KeyCode::F7,
        0x77 => KeyCode::F8,
        0x78 => KeyCode::F9,
        0x79 => KeyCode::F10,
        0x7A => KeyCode::F11,
        0x7B => KeyCode::F12,
        0x26 => KeyCode::Up,
        0x28 => KeyCode::Down,
        0x25 => KeyCode::Left,
        0x27 => KeyCode::Right,
        0x24 => KeyCode::Home,
        0x23 => KeyCode::End,
        0x21 => KeyCode::PageUp,
        0x22 => KeyCode::PageDown,
        0x2D => KeyCode::Insert,
        0x2E => KeyCode::Delete,
        0x08 => KeyCode::Backspace,
        0x09 => KeyCode::Tab,
        0x0D => KeyCode::Enter,
        0x1B => KeyCode::Escape,
        0x20 => KeyCode::Space,
        0x10 => KeyCode::Shift,
        0x11 => KeyCode::Control,
        0x12 => KeyCode::Alt,
        0x5B => KeyCode::LeftSuper,
        0x5C => KeyCode::RightSuper,
        0x14 => KeyCode::CapsLock,
        0x90 => KeyCode::NumLock,
        0x91 => KeyCode::ScrollLock,
        _ => KeyCode::Unknown,
    }
}

/// Gets the current modifier key state.
pub fn get_modifiers() -> i32 {
    use crate::view::modifiers;

    let mut mods = 0i32;

    unsafe {
        if GetKeyState(VK_SHIFT.0 as i32) < 0 {
            mods |= modifiers::SHIFT;
        }
        if GetKeyState(VK_CONTROL.0 as i32) < 0 {
            mods |= modifiers::CONTROL;
        }
        if GetKeyState(VK_MENU.0 as i32) < 0 {
            mods |= modifiers::ALT;
        }
        if GetKeyState(VK_LWIN.0 as i32) < 0 {
            mods |= modifiers::SUPER;
        }
        if GetKeyState(VK_CAPITAL.0 as i32) & 1 != 0 {
            mods |= modifiers::CAPS_LOCK;
        }
    }

    mods
}

/// Sets the cursor type.
pub fn set_cursor(cursor: CursorType) {
    unsafe {
        let cursor_id = match cursor {
            CursorType::Arrow => IDC_ARROW,
            CursorType::IBeam => IDC_IBEAM,
            CursorType::CrossHair => IDC_CROSS,
            CursorType::Hand => IDC_HAND,
            CursorType::HResize => IDC_SIZEWE,
            CursorType::VResize => IDC_SIZENS,
        };

        if let Ok(cursor) = LoadCursorW(None, cursor_id) {
            SetCursor(cursor);
        }
    }
}

/// Extracts mouse position from LPARAM.
fn get_mouse_pos(lparam: LPARAM) -> Point {
    let x = (lparam.0 & 0xFFFF) as i16 as f32;
    let y = ((lparam.0 >> 16) & 0xFFFF) as i16 as f32;
    Point::new(x, y)
}

/// Window procedure callback.
unsafe extern "system" fn window_proc(
    hwnd: HWND,
    msg: u32,
    wparam: WPARAM,
    lparam: LPARAM,
) -> LRESULT {
    match msg {
        WM_DESTROY => {
            PostQuitMessage(0);
            LRESULT(0)
        }
        WM_PAINT => {
            let mut ps = PAINTSTRUCT::default();
            let _hdc = BeginPaint(hwnd, &mut ps);
            // Would do drawing here using the view
            EndPaint(hwnd, &ps);
            LRESULT(0)
        }
        WM_SIZE => {
            // Handle resize
            LRESULT(0)
        }
        WM_LBUTTONDOWN | WM_LBUTTONUP |
        WM_RBUTTONDOWN | WM_RBUTTONUP |
        WM_MBUTTONDOWN | WM_MBUTTONUP => {
            // Handle mouse clicks
            LRESULT(0)
        }
        WM_MOUSEMOVE => {
            // Handle mouse movement
            LRESULT(0)
        }
        WM_MOUSEWHEEL => {
            // Handle scroll
            LRESULT(0)
        }
        WM_KEYDOWN | WM_KEYUP => {
            // Handle keyboard
            LRESULT(0)
        }
        WM_CHAR => {
            // Handle text input
            LRESULT(0)
        }
        _ => DefWindowProcW(hwnd, msg, wparam, lparam),
    }
}

/// Windows application wrapper.
pub struct WindowsApp {
    // Application state
}

impl WindowsApp {
    /// Creates a new Windows application.
    pub fn new() -> Option<Self> {
        Some(Self {})
    }

    /// Runs the application event loop.
    pub fn run(&self) {
        unsafe {
            let mut msg = MSG::default();
            while GetMessageW(&mut msg, None, 0, 0).into() {
                let _ = TranslateMessage(&msg);
                DispatchMessageW(&msg);
            }
        }
    }

    /// Stops the application.
    pub fn stop(&self) {
        unsafe {
            PostQuitMessage(0);
        }
    }
}

/// Windows window wrapper.
pub struct WindowsWindow {
    hwnd: HWND,
    view: Option<View>,
}

impl WindowsWindow {
    /// Creates a new Windows window.
    pub fn new(title: &str, size: Extent) -> Option<Self> {
        unsafe {
            let instance = GetModuleHandleW(None).ok()?;

            let class_name = w!("MKGraphicWindow");

            let wc = WNDCLASSW {
                style: CS_HREDRAW | CS_VREDRAW,
                lpfnWndProc: Some(window_proc),
                hInstance: instance.into(),
                lpszClassName: class_name,
                hCursor: LoadCursorW(None, IDC_ARROW).ok()?,
                ..Default::default()
            };

            RegisterClassW(&wc);

            // Convert title to wide string
            let title_wide: Vec<u16> = title.encode_utf16().chain(std::iter::once(0)).collect();

            let hwnd = CreateWindowExW(
                WINDOW_EX_STYLE::default(),
                class_name,
                PCWSTR(title_wide.as_ptr()),
                WS_OVERLAPPEDWINDOW,
                CW_USEDEFAULT,
                CW_USEDEFAULT,
                size.x as i32,
                size.y as i32,
                None,
                None,
                instance,
                None,
            )?;

            Some(Self {
                hwnd,
                view: Some(View::new(size)),
            })
        }
    }

    /// Shows the window.
    pub fn show(&self) {
        unsafe {
            let _ = ShowWindow(self.hwnd, SW_SHOW);
            let _ = UpdateWindow(self.hwnd);
        }
    }

    /// Returns the window size.
    pub fn size(&self) -> Extent {
        unsafe {
            let mut rect = RECT::default();
            let _ = GetWindowRect(self.hwnd, &mut rect);
            Extent::new(
                (rect.right - rect.left) as f32,
                (rect.bottom - rect.top) as f32,
            )
        }
    }

    /// Sets the window size.
    pub fn set_size(&self, size: Extent) {
        unsafe {
            let _ = SetWindowPos(
                self.hwnd,
                None,
                0,
                0,
                size.x as i32,
                size.y as i32,
                SWP_NOZORDER | SWP_NOMOVE,
            );
        }
    }

    /// Returns a reference to the view.
    pub fn view(&self) -> Option<&View> {
        self.view.as_ref()
    }

    /// Returns a mutable reference to the view.
    pub fn view_mut(&mut self) -> Option<&mut View> {
        self.view.as_mut()
    }

    /// Returns the window handle.
    pub fn handle(&self) -> HWND {
        self.hwnd
    }
}
