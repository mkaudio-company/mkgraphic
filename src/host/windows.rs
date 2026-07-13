//! Windows platform implementation.
//!
//! This module provides the Windows-specific implementation using Win32 API
//! through the windows crate.

#![cfg(target_os = "windows")]

use std::cell::{Cell, RefCell};
use std::collections::HashMap;

use windows::core::{w, PCWSTR};
use windows::Win32::Foundation::{HWND, LPARAM, LRESULT, POINT, RECT, WPARAM};
use windows::Win32::Graphics::Gdi::{
    BeginPaint, EndPaint, InvalidateRect, ScreenToClient, StretchDIBits, UpdateWindow, BITMAPINFO,
    BITMAPINFOHEADER, BI_RGB, DIB_RGB_COLORS, PAINTSTRUCT, SRCCOPY,
};
use windows::Win32::System::LibraryLoader::GetModuleHandleW;
use windows::Win32::System::SystemServices::{MK_LBUTTON, MK_MBUTTON, MK_RBUTTON};
use windows::Win32::UI::HiDpi::{
    GetDpiForSystem, GetDpiForWindow, SetProcessDpiAwarenessContext,
    DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2,
};
use windows::Win32::UI::Input::KeyboardAndMouse::{
    GetKeyState, VK_CAPITAL, VK_CONTROL, VK_LWIN, VK_MENU, VK_SHIFT,
};
use windows::Win32::UI::WindowsAndMessaging::{
    CreateWindowExW, DefWindowProcW, DispatchMessageW, GetClientRect, GetMessageW,
    GetWindowLongPtrW, GetWindowRect, KillTimer, LoadCursorW, PostQuitMessage, RegisterClassW,
    SetCursor, SetTimer, SetWindowLongPtrW, SetWindowPos, ShowWindow, TranslateMessage, CS_HREDRAW,
    CS_VREDRAW, CW_USEDEFAULT, GWLP_USERDATA, IDC_ARROW, IDC_CROSS, IDC_HAND, IDC_IBEAM,
    IDC_SIZENS, IDC_SIZEWE, MSG, SWP_NOMOVE, SWP_NOZORDER, SW_SHOW, WINDOW_EX_STYLE, WM_CHAR,
    WM_DESTROY, WM_KEYDOWN, WM_KEYUP, WM_LBUTTONDOWN, WM_LBUTTONUP, WM_MBUTTONDOWN, WM_MBUTTONUP,
    WM_MOUSEMOVE, WM_MOUSEWHEEL, WM_PAINT, WM_RBUTTONDOWN, WM_RBUTTONUP, WM_SIZE, WM_TIMER,
    WNDCLASSW, WS_OVERLAPPEDWINDOW,
};

use super::CloseBehavior;
use crate::element::context::Context;
use crate::element::ElementPtr;
use crate::support::canvas::Canvas;
use crate::support::color::Color;
use crate::support::point::{Extent, Point};
use crate::support::rect::Rect;
use crate::view::{
    CursorType, KeyAction, KeyCode, KeyInfo, MouseButton, MouseButtonKind, TextInfo, View,
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

/// Extracts a client-coordinate mouse position from a mouse-message LPARAM.
fn get_mouse_pos(lparam: LPARAM) -> POINT {
    let x = (lparam.0 & 0xFFFF) as i16 as i32;
    let y = ((lparam.0 >> 16) & 0xFFFF) as i16 as i32;
    POINT { x, y }
}

/// Per-window state, addressed from `window_proc` via `GWLP_USERDATA` since
/// Win32 has no equivalent of Cocoa's per-object ivars. Owned by the
/// `WindowsWindow` that created it; freed on `WM_DESTROY` (see
/// `window_proc`).
#[derive(Default)]
struct WindowState {
    canvas: RefCell<Option<Canvas>>,
    content: RefCell<Option<ElementPtr>>,
    /// Logical (DPI-independent) size, matching the `Extent` semantics used
    /// everywhere else in the element tree - not the raw pixel size Win32
    /// APIs like `GetClientRect` report.
    size: RefCell<Extent>,
}

/// Retrieves the state a `WindowsWindow` stashed on its `HWND`, if any.
///
/// # Safety
/// `hwnd` must be a window created by [`WindowsWindow::new`] (or null/
/// foreign, in which case this returns `None`); the returned reference's
/// lifetime is tied to that window's lifetime, not actually `'static` -
/// callers must not retain it past the window's destruction.
unsafe fn window_state(hwnd: HWND) -> Option<&'static WindowState> {
    let ptr = GetWindowLongPtrW(hwnd, GWLP_USERDATA) as *const WindowState;
    ptr.as_ref()
}

/// This window's current DPI scale (96 DPI == 1.0), for converting between
/// the physical pixels Win32 APIs report and the logical/point units the
/// rest of mkgraphic works in - the same role `backingScaleFactor` plays on
/// macOS, just not implicit the way AppKit makes it.
fn window_scale(hwnd: HWND) -> f32 {
    unsafe { GetDpiForWindow(hwnd) as f32 / 96.0 }
}

/// Dispatches `f` with a `Context` bound to `state`'s current logical size
/// and content, if any content is set. Used by every input handler below,
/// which all need the same throwaway `View`/`Canvas`/`Context` scaffolding
/// that `Context` requires but that none of them actually render with.
fn with_content_context(state: &WindowState, f: impl FnOnce(&ElementPtr, &Context)) {
    let content_ref = state.content.borrow();
    let Some(ref content) = *content_ref else {
        return;
    };
    let size = *state.size.borrow();
    let bounds = Rect {
        left: 0.0,
        top: 0.0,
        right: size.x,
        bottom: size.y,
    };
    let Some(dummy_canvas) = Canvas::new(1, 1) else {
        return;
    };
    let canvas_cell = RefCell::new(dummy_canvas);
    let temp_view = View::new(size);
    let ctx = Context::new(&temp_view, &canvas_cell, bounds);
    f(content, &ctx);
}

fn mouse_button_kind(msg: u32) -> MouseButtonKind {
    match msg {
        WM_RBUTTONDOWN | WM_RBUTTONUP => MouseButtonKind::Right,
        WM_MBUTTONDOWN | WM_MBUTTONUP => MouseButtonKind::Middle,
        _ => MouseButtonKind::Left,
    }
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
            // The state was `Box::into_raw`'d in `WindowsWindow::new`; this
            // is the one place it's reclaimed, since `WM_DESTROY` is the
            // last message a window receives.
            let ptr = GetWindowLongPtrW(hwnd, GWLP_USERDATA) as *mut WindowState;
            if !ptr.is_null() {
                drop(Box::from_raw(ptr));
                SetWindowLongPtrW(hwnd, GWLP_USERDATA, 0);
            }
            // Only quit the message loop if this was the last live window
            // and nobody asked to keep running past that (see
            // `CloseBehavior`/`App::set_close_behavior`) - otherwise leave
            // the loop blocking in `GetMessageW`, alive with no window.
            let remaining = WINDOW_COUNT.with(|count| {
                let n = count.get().saturating_sub(1);
                count.set(n);
                n
            });
            if remaining == 0 && QUIT_ON_LAST_WINDOW_CLOSED.with(|q| q.get()) {
                PostQuitMessage(0);
            }
            LRESULT(0)
        }
        WM_PAINT => {
            if let Some(state) = window_state(hwnd) {
                paint(hwnd, state);
            }
            let mut ps = PAINTSTRUCT::default();
            let _ = BeginPaint(hwnd, &mut ps);
            let _ = EndPaint(hwnd, &ps);
            LRESULT(0)
        }
        WM_SIZE => {
            if let Some(state) = window_state(hwnd) {
                let width = (lparam.0 & 0xFFFF) as u32 as f32;
                let height = ((lparam.0 >> 16) & 0xFFFF) as u32 as f32;
                let scale = window_scale(hwnd);
                *state.size.borrow_mut() = Extent::new(width / scale, height / scale);
                let _ = InvalidateRect(hwnd, None, false);
            }
            LRESULT(0)
        }
        WM_LBUTTONDOWN | WM_LBUTTONUP | WM_RBUTTONDOWN | WM_RBUTTONUP | WM_MBUTTONDOWN
        | WM_MBUTTONUP => {
            if let Some(state) = window_state(hwnd) {
                let down = matches!(msg, WM_LBUTTONDOWN | WM_RBUTTONDOWN | WM_MBUTTONDOWN);
                let scale = window_scale(hwnd);
                let raw = get_mouse_pos(lparam);
                let pos = Point::new(raw.x as f32 / scale, raw.y as f32 / scale);

                let mouse_btn = MouseButton {
                    down,
                    click_count: 1,
                    button: mouse_button_kind(msg),
                    modifiers: get_modifiers(),
                    pos,
                };

                with_content_context(state, |content, ctx| {
                    // Clear focus before dispatching the click, same reasoning
                    // as the macOS backend: if the click lands on a focusable
                    // control, that control's own re-focus in handle_click
                    // must not be immediately wiped out afterward.
                    if down {
                        content.clear_focus();
                    }
                    let _ = content.handle_click(ctx, mouse_btn);
                });
                let _ = InvalidateRect(hwnd, None, false);
            }
            LRESULT(0)
        }
        WM_MOUSEMOVE => {
            if let Some(state) = window_state(hwnd) {
                // Only forward as a drag while a button is actually held,
                // matching the macOS backend (which only wires up
                // `mouseDragged:`, not plain `mouseMoved:`/hover tracking).
                let buttons_down =
                    (wparam.0 & (MK_LBUTTON.0 | MK_RBUTTON.0 | MK_MBUTTON.0) as usize) != 0;
                if buttons_down {
                    let scale = window_scale(hwnd);
                    let raw = get_mouse_pos(lparam);
                    let pos = Point::new(raw.x as f32 / scale, raw.y as f32 / scale);

                    let mouse_btn = MouseButton {
                        down: true,
                        click_count: 1,
                        button: MouseButtonKind::Left,
                        modifiers: get_modifiers(),
                        pos,
                    };

                    with_content_context(state, |content, ctx| {
                        content.handle_drag(ctx, mouse_btn);
                    });
                    let _ = InvalidateRect(hwnd, None, false);
                }
            }
            LRESULT(0)
        }
        WM_MOUSEWHEEL => {
            if let Some(state) = window_state(hwnd) {
                // Unlike the other mouse messages, WM_MOUSEWHEEL's lParam is
                // in *screen* coordinates - convert to client coordinates
                // before use.
                let mut pt = get_mouse_pos(lparam);
                let _ = ScreenToClient(hwnd, &mut pt);

                let scale = window_scale(hwnd);
                let pos = Point::new(pt.x as f32 / scale, pt.y as f32 / scale);

                let wheel_delta = ((wparam.0 >> 16) & 0xFFFF) as i16 as f32 / 120.0;
                let dir = Point::new(0.0, wheel_delta);

                with_content_context(state, |content, ctx| {
                    if content.handle_scroll(ctx, dir, pos) {
                        let _ = InvalidateRect(hwnd, None, false);
                    }
                });
            }
            LRESULT(0)
        }
        WM_KEYDOWN | WM_KEYUP => {
            if let Some(state) = window_state(hwnd) {
                let key = translate_key(wparam.0 as i32);
                let action = if msg == WM_KEYDOWN {
                    KeyAction::Press
                } else {
                    KeyAction::Release
                };
                let key_info = KeyInfo {
                    key,
                    action,
                    modifiers: get_modifiers(),
                };

                with_content_context(state, |content, ctx| {
                    if content.handle_key(ctx, key_info) {
                        let _ = InvalidateRect(hwnd, None, false);
                    }
                });
            }
            LRESULT(0)
        }
        WM_CHAR => {
            if let Some(state) = window_state(hwnd) {
                // WM_CHAR delivers one UTF-16 code unit per message; this
                // doesn't reassemble surrogate pairs (characters outside the
                // BMP), matching the scope of everything else here as a
                // first real implementation rather than a complete IME/
                // Unicode-input pipeline.
                if let Some(c) = char::from_u32(wparam.0 as u32) {
                    if !c.is_control() || c == '\n' || c == '\t' {
                        let text_info = TextInfo {
                            codepoint: c,
                            modifiers: get_modifiers(),
                        };
                        with_content_context(state, |content, ctx| {
                            if content.handle_text(ctx, text_info) {
                                let _ = InvalidateRect(hwnd, None, false);
                            }
                        });
                    }
                }
            }
            LRESULT(0)
        }
        WM_TIMER => {
            timer_proc(hwnd, msg, wparam.0, 0);
            LRESULT(0)
        }
        _ => DefWindowProcW(hwnd, msg, wparam, lparam),
    }
}

/// Renders `state`'s content into its canvas and blits it to `hwnd`. Called
/// from `WM_PAINT`, before the real `BeginPaint`/`EndPaint` pair (which
/// exist mainly to satisfy Windows' internal update-region bookkeeping;
/// GDI drawing itself happens against a plain `GetDC`, mirroring how the
/// macOS backend draws against whatever `NSGraphicsContext` is current
/// rather than something tied to `drawRect:`'s parameter).
fn paint(hwnd: HWND, state: &WindowState) {
    unsafe {
        let mut rect = RECT::default();
        if GetClientRect(hwnd, &mut rect).is_err() {
            return;
        }
        let pixel_width = (rect.right - rect.left).max(0) as u32;
        let pixel_height = (rect.bottom - rect.top).max(0) as u32;
        if pixel_width == 0 || pixel_height == 0 {
            return;
        }

        let scale = window_scale(hwnd);
        let logical_size = Extent::new(pixel_width as f32 / scale, pixel_height as f32 / scale);
        *state.size.borrow_mut() = logical_size;

        {
            let mut canvas_opt = state.canvas.borrow_mut();
            let needs_new = match &*canvas_opt {
                Some(c) => c.width() != pixel_width || c.height() != pixel_height,
                None => true,
            };
            if needs_new {
                *canvas_opt = Canvas::new(pixel_width, pixel_height);
            }
        }

        let mut canvas_opt = state.canvas.borrow_mut();
        let Some(ref mut canvas) = *canvas_opt else {
            return;
        };

        canvas.clear(Color::new(0.2, 0.2, 0.2, 1.0));

        // Establish the HiDPI base scale, same role as on macOS: element
        // drawing operates entirely in logical points, so the canvas'
        // transform must scale that up to fill its physical-pixel buffer.
        canvas.reset_transform();
        canvas.scale(scale, scale);

        let content_ref = state.content.borrow();
        if let Some(ref content) = *content_ref {
            let bounds = Rect {
                left: 0.0,
                top: 0.0,
                right: logical_size.x,
                bottom: logical_size.y,
            };

            let mut temp_view = View::new(logical_size);
            temp_view.set_scale(scale);

            let temp_canvas = std::mem::replace(canvas, Canvas::new(1, 1).unwrap());
            let canvas_cell = RefCell::new(temp_canvas);
            let ctx = Context::new(&temp_view, &canvas_cell, bounds);

            content.draw(&ctx);

            *canvas = canvas_cell.into_inner();
        }
        drop(content_ref);

        blit_to_window(hwnd, canvas, pixel_width, pixel_height);
    }
}

/// Blits `canvas` onto `hwnd`'s client area. Unlike the macOS backend's
/// blit (which maps a physical-pixel-resolution image onto a
/// logical-point-sized destination rect, since CoreGraphics separates the
/// two), Win32 GDI works in device pixels natively - `canvas`'s pixel
/// dimensions already match the client area exactly (see `paint` above),
/// so this is a plain 1:1 copy.
unsafe fn blit_to_window(hwnd: HWND, canvas: &Canvas, width: u32, height: u32) {
    let hdc = windows::Win32::Graphics::Gdi::GetDC(hwnd);
    if hdc.is_invalid() {
        return;
    }

    // tiny-skia stores premultiplied RGBA; GDI's `BI_RGB` DIB format is
    // BGRA (and bottom-up unless the height is negative) - swap R/B per
    // pixel into a scratch buffer rather than fight GDI's channel order.
    let src = canvas.pixmap().data();
    let mut bgra = vec![0u8; src.len()];
    for (chunk_in, chunk_out) in src.chunks_exact(4).zip(bgra.chunks_exact_mut(4)) {
        chunk_out[0] = chunk_in[2];
        chunk_out[1] = chunk_in[1];
        chunk_out[2] = chunk_in[0];
        chunk_out[3] = chunk_in[3];
    }

    let bmi = BITMAPINFO {
        bmiHeader: BITMAPINFOHEADER {
            biSize: std::mem::size_of::<BITMAPINFOHEADER>() as u32,
            biWidth: width as i32,
            // Negative height = top-down DIB, matching our top-left origin
            // (and avoiding a manual row-flip to GDI's default bottom-up).
            biHeight: -(height as i32),
            biPlanes: 1,
            biBitCount: 32,
            biCompression: BI_RGB.0,
            ..Default::default()
        },
        ..Default::default()
    };

    StretchDIBits(
        hdc,
        0,
        0,
        width as i32,
        height as i32,
        0,
        0,
        width as i32,
        height as i32,
        Some(bgra.as_ptr() as *const _),
        &bmi,
        DIB_RGB_COLORS,
        SRCCOPY,
    );

    windows::Win32::Graphics::Gdi::ReleaseDC(hwnd, hdc);
}

/// One registered [`App::schedule_timer`]/[`App::schedule_once`] callback,
/// keyed by the OS-assigned timer id `SetTimer` returns.
struct TimerEntry {
    callback: Box<dyn FnMut()>,
    repeats: bool,
}

thread_local! {
    static TIMER_CALLBACKS: RefCell<HashMap<usize, TimerEntry>> = RefCell::new(HashMap::new());
    // Tracks live windows and what should happen when the last one closes
    // (see `CloseBehavior`/`App::set_close_behavior`). `window_proc`'s
    // `WM_DESTROY` arm is a plain `extern "system" fn` with no `&WindowsApp`
    // reachable from it, so this state has to live somewhere
    // process/thread-global rather than on a struct - same reasoning as
    // `TIMER_CALLBACKS` above.
    static WINDOW_COUNT: Cell<u32> = const { Cell::new(0) };
    static QUIT_ON_LAST_WINDOW_CLOSED: Cell<bool> = const { Cell::new(true) };
}

/// `TIMERPROC` for every app-level timer (`hwnd = None` in `SetTimer`, so
/// Windows calls this directly via `DispatchMessage` rather than routing a
/// `WM_TIMER` through any particular window's `window_proc`). Also invoked
/// directly from `window_proc`'s own `WM_TIMER` arm as a fallback, in case
/// a future caller ever schedules a window-associated timer instead.
unsafe extern "system" fn timer_proc(_hwnd: HWND, _msg: u32, id: usize, _dwtime: u32) {
    let entry = TIMER_CALLBACKS.with(|cbs| cbs.borrow_mut().remove(&id));
    if let Some(mut entry) = entry {
        (entry.callback)();
        if entry.repeats {
            TIMER_CALLBACKS.with(|cbs| {
                cbs.borrow_mut().insert(id, entry);
            });
        } else {
            let _ = KillTimer(None, id);
        }
    }
}

/// A handle to a Windows-backed timer. See `Timer` in `host/mod.rs`, which
/// wraps this the same way it wraps the macOS `NSTimer`.
pub struct WindowsTimer {
    id: usize,
}

impl WindowsTimer {
    pub fn cancel(&self) {
        unsafe {
            let _ = KillTimer(None, self.id);
        }
        TIMER_CALLBACKS.with(|cbs| {
            cbs.borrow_mut().remove(&self.id);
        });
    }
}

impl Drop for WindowsTimer {
    fn drop(&mut self) {
        self.cancel();
    }
}

/// Windows application wrapper.
pub struct WindowsApp {}

impl WindowsApp {
    /// Creates a new Windows application.
    pub fn new() -> Option<Self> {
        unsafe {
            // Without this, Windows treats the process as DPI-unaware and
            // bitmap-stretches the whole window on HiDPI displays (blurry,
            // same failure mode the macOS backend had before it queried
            // `backingScaleFactor`) rather than letting us render at the
            // real pixel resolution ourselves.
            let _ = SetProcessDpiAwarenessContext(DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2);
        }
        Some(Self {})
    }

    /// Schedules `callback` to run on this thread's message loop: every
    /// `interval_secs` seconds if `repeats`, or once otherwise. Uses an
    /// `hwnd`-less (`SetTimer(None, ...)`) timer with an explicit
    /// `TIMERPROC`, so it fires via `DispatchMessage` independent of any
    /// particular window - mirroring the macOS backend's app-level
    /// `NSTimer`, which isn't tied to a window either.
    ///
    /// Win32 timers always repeat at the OS level; `KillTimer` from inside
    /// `timer_proc` after a non-repeating entry's one firing is what
    /// actually stops it (there's no "fire once" flag to pass to
    /// `SetTimer` itself).
    pub fn schedule_timer(
        &self,
        interval_secs: f64,
        repeats: bool,
        callback: impl FnMut() + 'static,
    ) -> WindowsTimer {
        let elapse_ms = ((interval_secs * 1000.0).round() as u32).max(1);
        // When hwnd is null, Win32 ignores the requested id and returns a
        // fresh one - that return value, not anything we pass in, is what
        // must be used to look the callback up and to `KillTimer` it later.
        let id = unsafe { SetTimer(None, 0, elapse_ms, Some(timer_proc)) };
        TIMER_CALLBACKS.with(|cbs| {
            cbs.borrow_mut().insert(
                id,
                TimerEntry {
                    callback: Box::new(callback),
                    repeats,
                },
            );
        });
        WindowsTimer { id }
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

    /// See [`super::CloseBehavior`] and [`super::App::set_close_behavior`].
    ///
    /// Windows has no equivalent of macOS's Dock-icon reopen gesture, so
    /// only the "don't quit when the last window closes" half of
    /// `CloseBehavior::KeepRunning` is honored here; the `rebuild` closure
    /// is intentionally never called.
    pub fn set_close_behavior(&self, behavior: CloseBehavior) {
        let quit_on_last_window_closed = match behavior {
            CloseBehavior::QuitApp => true,
            CloseBehavior::KeepRunning(_) => false,
        };
        QUIT_ON_LAST_WINDOW_CLOSED.with(|q| q.set(quit_on_last_window_closed));
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

            // `size` is logical (point) units, matching every other backend;
            // Win32 window creation wants physical pixels, so scale by the
            // system DPI (the best estimate available before the window -
            // and thus its own per-monitor DPI - exists).
            let scale = GetDpiForSystem() as f32 / 96.0;
            let pixel_width = (size.x * scale).round() as i32;
            let pixel_height = (size.y * scale).round() as i32;

            let hwnd = CreateWindowExW(
                WINDOW_EX_STYLE::default(),
                class_name,
                PCWSTR(title_wide.as_ptr()),
                WS_OVERLAPPEDWINDOW,
                CW_USEDEFAULT,
                CW_USEDEFAULT,
                pixel_width,
                pixel_height,
                None,
                None,
                instance,
                None,
            );
            if hwnd.0 == 0 {
                return None;
            }

            let state = Box::new(WindowState {
                canvas: RefCell::new(None),
                content: RefCell::new(None),
                size: RefCell::new(size),
            });
            SetWindowLongPtrW(hwnd, GWLP_USERDATA, Box::into_raw(state) as isize);
            WINDOW_COUNT.with(|count| count.set(count.get() + 1));

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
            let scale = window_scale(self.hwnd);
            let _ = SetWindowPos(
                self.hwnd,
                None,
                0,
                0,
                (size.x * scale).round() as i32,
                (size.y * scale).round() as i32,
                SWP_NOZORDER | SWP_NOMOVE,
            );
            if let Some(state) = window_state(self.hwnd) {
                *state.size.borrow_mut() = size;
            }
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

    /// Sets the window title.
    pub fn set_title(&self, title: &str) {
        let title_wide: Vec<u16> = title.encode_utf16().chain(std::iter::once(0)).collect();
        unsafe {
            let _ = windows::Win32::UI::WindowsAndMessaging::SetWindowTextW(
                self.hwnd,
                PCWSTR(title_wide.as_ptr()),
            );
        }
    }

    /// Sets the window content.
    pub fn set_content(&self, content: ElementPtr) {
        if let Some(state) = unsafe { window_state(self.hwnd) } {
            *state.content.borrow_mut() = Some(content);
            unsafe {
                let _ = InvalidateRect(self.hwnd, None, false);
            }
        }
    }

    /// Hides the window.
    pub fn hide(&self) {
        unsafe {
            let _ = ShowWindow(self.hwnd, windows::Win32::UI::WindowsAndMessaging::SW_HIDE);
        }
    }

    /// Closes the window.
    pub fn close(&self) {
        unsafe {
            let _ = windows::Win32::UI::WindowsAndMessaging::DestroyWindow(self.hwnd);
        }
    }

    /// Triggers a redraw.
    pub fn refresh(&self) {
        unsafe {
            let _ = InvalidateRect(self.hwnd, None, false);
        }
    }

    /// Returns the raw `HWND`, for embedding externally-managed native
    /// content into this window instead of using mkgraphic's own element
    /// tree for it - the Windows equivalent of `MacOSWindow::native_window_handle`.
    pub fn native_window_handle(&self) -> *mut std::ffi::c_void {
        self.hwnd.0 as *mut std::ffi::c_void
    }

    /// Returns the window handle.
    pub fn handle(&self) -> HWND {
        self.hwnd
    }
}
