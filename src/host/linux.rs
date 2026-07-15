//! Linux platform implementation.
//!
//! This module provides the Linux-specific implementation using X11
//! through the x11rb crate.
//!
//! Known limitation: HiDPI/per-monitor scaling isn't implemented here (scale
//! is always 1.0) - unlike macOS's `backingScaleFactor` or Windows'
//! `GetDpiForWindow`, X11 has no single unified per-window DPI API (it's
//! fragmented across desktop environments via XRandR/XSettings/`Xft.dpi`),
//! and covering that properly is a separate, environment-specific task from
//! getting real rendering/input/timer dispatch working at all.

#![cfg(target_os = "linux")]

use std::cell::{Cell, RefCell};
use std::collections::HashMap;
use std::os::unix::io::AsRawFd;
use std::rc::Rc;
use std::sync::Arc;
use std::time::{Duration, Instant};

use x11rb::connection::Connection;
use x11rb::protocol::xproto::*;
use x11rb::protocol::Event;
use x11rb::rust_connection::RustConnection;
use x11rb::wrapper::ConnectionExt as _;
use x11rb::COPY_DEPTH_FROM_PARENT;

use super::CloseBehavior;
use crate::element::context::Context;
use crate::element::ElementPtr;
use crate::support::canvas::Canvas;
use crate::support::color::Color;
use crate::support::point::{Extent, Point};
use crate::support::rect::Rect;
use crate::view::{KeyAction, KeyCode, KeyInfo, MouseButton, MouseButtonKind, TextInfo, View};

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

/// Per-window state. Unlike the Windows backend (which stashes this on the
/// `HWND` via `GWLP_USERDATA`), X11 has no per-window user-data slot, and
/// the event loop is centralized on `LinuxApp` rather than a per-window
/// callback - so `LinuxApp` keeps a registry of these keyed by X window ID,
/// and each `LinuxWindow` also holds its own `Rc` clone directly.
struct WindowState {
    canvas: RefCell<Option<Canvas>>,
    content: RefCell<Option<ElementPtr>>,
    /// Logical size. Equal to the physical pixel size for now since there's
    /// no HiDPI scaling on this backend yet (see the module-level doc).
    size: RefCell<Extent>,
    gc: Gcontext,
    depth: u8,
    bits_per_pixel: u8,
    msb_first: bool,
}

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

/// Renders `state`'s content and blits it to `xid`.
fn paint(conn: &RustConnection, xid: Window, state: &WindowState) {
    let Ok(cookie) = conn.get_geometry(xid) else {
        return;
    };
    let Ok(geom) = cookie.reply() else {
        return;
    };
    let width = (geom.width as u32).max(1);
    let height = (geom.height as u32).max(1);

    let logical_size = Extent::new(width as f32, height as f32);
    *state.size.borrow_mut() = logical_size;

    {
        let mut canvas_opt = state.canvas.borrow_mut();
        let needs_new = match &*canvas_opt {
            Some(c) => c.width() != width || c.height() != height,
            None => true,
        };
        if needs_new {
            *canvas_opt = Canvas::new(width, height);
        }
    }

    let mut canvas_opt = state.canvas.borrow_mut();
    let Some(ref mut canvas) = *canvas_opt else {
        return;
    };

    canvas.clear(Color::new(0.2, 0.2, 0.2, 1.0));
    canvas.reset_transform();

    let content_ref = state.content.borrow();
    if let Some(ref content) = *content_ref {
        let bounds = Rect {
            left: 0.0,
            top: 0.0,
            right: logical_size.x,
            bottom: logical_size.y,
        };

        let temp_view = View::new(logical_size);
        let temp_canvas = std::mem::replace(canvas, Canvas::new(1, 1).unwrap());
        let canvas_cell = RefCell::new(temp_canvas);
        let ctx = Context::new(&temp_view, &canvas_cell, bounds);

        // See `host::macos`'s identical fix: overlays need a genuinely
        // separate second pass over the whole tree, not one embedded in
        // `draw` (element::tile::VTile::draw_overlay's doc comment).
        content.draw(&ctx);
        content.draw_overlay(&ctx);

        *canvas = canvas_cell.into_inner();
    }
    drop(content_ref);

    blit_to_window(conn, xid, state, canvas, width, height);
}

/// Converts `canvas`'s premultiplied RGBA into the depth/byte-order the X
/// server expects and sends it via `PutImage`, in row-chunks sized to stay
/// under the server's `maximum_request_length` (a single request can only
/// carry so many bytes; a tall/large window's image may need several).
///
/// Assumes the common case of a standard TrueColor visual (red/green/blue
/// occupying the top three bytes in server byte order) rather than parsing
/// the root visual's actual channel masks - true on effectively every
/// modern Linux desktop, but not guaranteed by the X protocol itself.
fn blit_to_window(
    conn: &RustConnection,
    xid: Window,
    state: &WindowState,
    canvas: &Canvas,
    width: u32,
    height: u32,
) {
    let bytes_per_pixel = ((state.bits_per_pixel as usize) / 8).max(1);
    let src = canvas.pixmap().data();

    let mut packed = vec![0u8; width as usize * height as usize * bytes_per_pixel];
    for (i, px) in src.chunks_exact(4).enumerate() {
        let (r, g, b) = (px[0], px[1], px[2]);
        let out = &mut packed[i * bytes_per_pixel..(i + 1) * bytes_per_pixel];
        if state.msb_first {
            if bytes_per_pixel >= 4 {
                out[bytes_per_pixel - 3] = r;
                out[bytes_per_pixel - 2] = g;
                out[bytes_per_pixel - 1] = b;
            } else if bytes_per_pixel == 3 {
                out[0] = r;
                out[1] = g;
                out[2] = b;
            }
        } else if bytes_per_pixel >= 3 {
            out[0] = b;
            out[1] = g;
            out[2] = r;
        }
    }

    let row_bytes = width as usize * bytes_per_pixel;
    if row_bytes == 0 {
        return;
    }
    // Conservative allowance for the PutImage request header itself so
    // chunking stays comfortably under the limit rather than exactly at it.
    let max_request_bytes = (conn.setup().maximum_request_length as usize) * 4;
    let max_rows_per_request = (max_request_bytes.saturating_sub(64) / row_bytes).max(1);

    let mut y = 0u32;
    while y < height {
        let rows = (max_rows_per_request as u32).min(height - y);
        let start = y as usize * row_bytes;
        let end = start + rows as usize * row_bytes;
        let _ = conn.put_image(
            ImageFormat::Z_PIXMAP,
            xid,
            state.gc,
            width as u16,
            rows as u16,
            0,
            y as i16,
            0,
            state.depth,
            &packed[start..end],
        );
        y += rows;
    }
    let _ = conn.flush();
}

/// One registered [`LinuxApp::schedule_timer`]/`schedule_once` callback.
struct TimerEntry {
    next_fire: Instant,
    /// `Some(interval)` if repeating, `None` for a one-shot timer.
    interval: Option<Duration>,
    callback: Box<dyn FnMut()>,
    cancelled: Rc<Cell<bool>>,
}

/// A handle to a [`LinuxApp::schedule_timer`] callback. Dropping this (or
/// calling [`Self::cancel`]) stops future firings - mirrors `Timer` on
/// macOS/Windows, just without needing any OS-level timer object, since the
/// run loop below checks `cancelled` itself every iteration.
pub struct LinuxTimer {
    cancelled: Rc<Cell<bool>>,
}

impl LinuxTimer {
    pub fn cancel(&self) {
        self.cancelled.set(true);
    }
}

impl Drop for LinuxTimer {
    fn drop(&mut self) {
        self.cancel();
    }
}

/// The connection/registry state shared between `LinuxApp` and every
/// `LinuxWindow`. Held behind `Rc` and stashed in `CURRENT_APP` so that
/// `LinuxWindow::new(title, size)` can reach it without needing an `&App`
/// parameter - keeping the cross-platform `Window::new` signature the same
/// on Linux as on macOS/Windows (neither of which ties window creation to a
/// specific app instance either; macOS just needs a `MainThreadMarker`,
/// Windows needs nothing app-specific at all).
struct LinuxAppInner {
    conn: Arc<RustConnection>,
    screen_num: usize,
    windows: RefCell<HashMap<Window, Rc<WindowState>>>,
    timers: RefCell<Vec<TimerEntry>>,
    quit_on_last_window_closed: Cell<bool>,
}

thread_local! {
    static CURRENT_APP: RefCell<Option<Rc<LinuxAppInner>>> = const { RefCell::new(None) };
}

/// Linux/X11 application wrapper.
pub struct LinuxApp {
    inner: Rc<LinuxAppInner>,
    running: bool,
}

impl LinuxApp {
    /// Creates a new Linux application.
    pub fn new() -> Option<Self> {
        let (conn, screen_num) = RustConnection::connect(None).ok()?;
        let inner = Rc::new(LinuxAppInner {
            conn: Arc::new(conn),
            screen_num,
            windows: RefCell::new(HashMap::new()),
            timers: RefCell::new(Vec::new()),
            quit_on_last_window_closed: Cell::new(true),
        });
        CURRENT_APP.with(|cell| *cell.borrow_mut() = Some(inner.clone()));
        Some(Self {
            inner,
            running: false,
        })
    }

    /// Returns the connection.
    pub fn connection(&self) -> &Arc<RustConnection> {
        &self.inner.conn
    }

    /// Returns the screen number.
    pub fn screen_num(&self) -> usize {
        self.inner.screen_num
    }

    /// Schedules `callback` to run on this thread's event loop: every
    /// `interval_secs` seconds if `repeats`, or once otherwise. Since the
    /// event loop is our own code (unlike a native message pump), timers
    /// are just entries checked each iteration against a `poll()` timeout,
    /// with no OS-level timer object involved at all.
    pub fn schedule_timer(
        &self,
        interval_secs: f64,
        repeats: bool,
        callback: impl FnMut() + 'static,
    ) -> LinuxTimer {
        let interval = Duration::from_secs_f64(interval_secs.max(0.0));
        let cancelled = Rc::new(Cell::new(false));
        self.inner.timers.borrow_mut().push(TimerEntry {
            next_fire: Instant::now() + interval,
            interval: if repeats { Some(interval) } else { None },
            callback: Box::new(callback),
            cancelled: cancelled.clone(),
        });
        LinuxTimer { cancelled }
    }

    /// Runs the application event loop.
    pub fn run(&mut self) {
        self.running = true;

        while self.running {
            let timeout = self.next_timer_timeout();
            if self.wait_readable(timeout) {
                loop {
                    match self.inner.conn.poll_for_event() {
                        Ok(Some(event)) => {
                            self.handle_event(event);
                            if !self.running {
                                break;
                            }
                        }
                        Ok(None) => break,
                        Err(_) => {
                            self.running = false;
                            break;
                        }
                    }
                }
            }
            self.fire_due_timers();
        }
    }

    /// Stops the application.
    pub fn stop(&mut self) {
        self.running = false;
    }

    /// See [`super::CloseBehavior`] and [`super::App::set_close_behavior`].
    ///
    /// X11 has no equivalent of macOS's Dock-icon reopen gesture, so only
    /// the "don't quit when the last window closes" half of
    /// `CloseBehavior::KeepRunning` is honored here; the `rebuild` closure
    /// is intentionally never called.
    pub fn set_close_behavior(&self, behavior: CloseBehavior) {
        let quit_on_last_window_closed = match behavior {
            CloseBehavior::QuitApp => true,
            CloseBehavior::KeepRunning(_) => false,
        };
        self.inner
            .quit_on_last_window_closed
            .set(quit_on_last_window_closed);
    }

    /// Time until the earliest live timer should fire, or `None` if there
    /// are none - used as the `poll()` timeout so the loop wakes up in time
    /// to service timers even with no X activity at all.
    fn next_timer_timeout(&self) -> Option<Duration> {
        let now = Instant::now();
        self.inner
            .timers
            .borrow()
            .iter()
            .filter(|t| !t.cancelled.get())
            .map(|t| t.next_fire.saturating_duration_since(now))
            .min()
    }

    /// Blocks (via `poll(2)` on the X connection's socket) until either the
    /// connection has data to read or `timeout` elapses. Returns whether
    /// there's data to read. `None` timeout blocks indefinitely, matching
    /// the original purely-blocking `wait_for_event` loop's behavior when
    /// no timers are scheduled.
    fn wait_readable(&self, timeout: Option<Duration>) -> bool {
        let fd = self.inner.conn.stream().as_raw_fd();
        let mut pfd = libc::pollfd {
            fd,
            events: libc::POLLIN,
            revents: 0,
        };
        let timeout_ms = match timeout {
            Some(d) => i32::try_from(d.as_millis()).unwrap_or(i32::MAX),
            None => -1,
        };
        let ret = unsafe { libc::poll(&mut pfd, 1, timeout_ms) };
        ret > 0 && (pfd.revents & libc::POLLIN) != 0
    }

    /// Fires every timer whose deadline has passed, rescheduling repeating
    /// ones. Extracts due entries out of `self.timers` before invoking their
    /// callbacks (rather than iterating while borrowed) so a callback that
    /// itself calls `schedule_timer`/`schedule_once` - a very likely thing
    /// for a repeating timer's own callback to do - doesn't hit a `RefCell`
    /// double-borrow panic.
    fn fire_due_timers(&self) {
        let now = Instant::now();
        let due = {
            let mut timers = self.inner.timers.borrow_mut();
            timers.retain(|t| !t.cancelled.get());
            let (due, remaining): (Vec<_>, Vec<_>) = std::mem::take(&mut *timers)
                .into_iter()
                .partition(|t| t.next_fire <= now);
            *timers = remaining;
            due
        };

        for mut entry in due {
            if entry.cancelled.get() {
                continue;
            }
            (entry.callback)();
            if !entry.cancelled.get() {
                if let Some(interval) = entry.interval {
                    entry.next_fire = Instant::now() + interval;
                    self.inner.timers.borrow_mut().push(entry);
                }
            }
        }
    }

    fn handle_event(&mut self, event: Event) {
        match event {
            Event::Expose(e) => {
                if let Some(state) = self.inner.windows.borrow().get(&e.window).cloned() {
                    paint(&self.inner.conn, e.window, &state);
                }
            }
            Event::ConfigureNotify(e) => {
                // Repaint eagerly rather than waiting for a separate Expose,
                // since not every window manager reliably sends one after a
                // resize.
                if let Some(state) = self.inner.windows.borrow().get(&e.window).cloned() {
                    paint(&self.inner.conn, e.window, &state);
                }
            }
            Event::ButtonPress(e) | Event::ButtonRelease(e) => {
                let down = matches!(event, Event::ButtonPress(_));
                let Some(state) = self.inner.windows.borrow().get(&e.event).cloned() else {
                    return;
                };
                let pos = Point::new(e.event_x as f32, e.event_y as f32);
                let modifiers = translate_modifiers(u16::from(e.state));

                // X11 represents the scroll wheel as button presses (4/5 =
                // vertical, 6/7 = horizontal) rather than a distinct event.
                match e.detail {
                    4..=7 if down => {
                        let dir = match e.detail {
                            4 => Point::new(0.0, 1.0),
                            5 => Point::new(0.0, -1.0),
                            6 => Point::new(-1.0, 0.0),
                            _ => Point::new(1.0, 0.0),
                        };
                        with_content_context(&state, |content, ctx| {
                            let _ = content.handle_scroll(ctx, dir, pos);
                        });
                    }
                    4..=7 => {
                        // The matching release for a scroll "click" - nothing to do.
                    }
                    button => {
                        let mouse_btn = MouseButton {
                            down,
                            click_count: 1,
                            button: match button {
                                3 => MouseButtonKind::Right,
                                2 => MouseButtonKind::Middle,
                                _ => MouseButtonKind::Left,
                            },
                            modifiers,
                            pos,
                        };
                        with_content_context(&state, |content, ctx| {
                            if down {
                                content.clear_focus();
                            }
                            let _ = content.handle_click(ctx, mouse_btn);
                        });
                    }
                }
                paint(&self.inner.conn, e.event, &state);
            }
            Event::MotionNotify(e) => {
                let Some(state) = self.inner.windows.borrow().get(&e.event).cloned() else {
                    return;
                };
                // Only forward as a drag while a button is actually held,
                // matching the macOS/Windows backends (which don't wire up
                // hover/plain mouse-move tracking either).
                let held_buttons = u16::from(KeyButMask::BUTTON1)
                    | u16::from(KeyButMask::BUTTON2)
                    | u16::from(KeyButMask::BUTTON3);
                let buttons_down = (u16::from(e.state) & held_buttons) != 0;
                if buttons_down {
                    let mouse_btn = MouseButton {
                        down: true,
                        click_count: 1,
                        button: MouseButtonKind::Left,
                        modifiers: translate_modifiers(u16::from(e.state)),
                        pos: Point::new(e.event_x as f32, e.event_y as f32),
                    };
                    with_content_context(&state, |content, ctx| {
                        content.handle_drag(ctx, mouse_btn);
                    });
                    paint(&self.inner.conn, e.event, &state);
                }
            }
            Event::KeyPress(e) | Event::KeyRelease(e) => {
                let Some(state) = self.inner.windows.borrow().get(&e.event).cloned() else {
                    return;
                };
                let down = matches!(event, Event::KeyPress(_));
                let key_info = KeyInfo {
                    key: translate_key(e.detail),
                    action: if down {
                        KeyAction::Press
                    } else {
                        KeyAction::Release
                    },
                    modifiers: translate_modifiers(u16::from(e.state)),
                };
                let mut handled = false;
                with_content_context(&state, |content, ctx| {
                    handled = content.handle_key(ctx, key_info);
                });

                // This backend has no XKB/input-method integration, so text
                // input is approximated from the keycode's Latin-1 mapping
                // rather than going through a proper compose/IME pipeline -
                // enough for ASCII text entry, not for most non-Latin input.
                if down {
                    if let Some(c) = keycode_to_ascii(e.detail, u16::from(e.state)) {
                        let text_info = TextInfo {
                            codepoint: c,
                            modifiers: translate_modifiers(u16::from(e.state)),
                        };
                        with_content_context(&state, |content, ctx| {
                            handled = content.handle_text(ctx, text_info) || handled;
                        });
                    }
                }

                if handled {
                    paint(&self.inner.conn, e.event, &state);
                }
            }
            Event::DestroyNotify(e) => {
                self.inner.windows.borrow_mut().remove(&e.window);
                if self.inner.windows.borrow().is_empty()
                    && self.inner.quit_on_last_window_closed.get()
                {
                    self.running = false;
                }
            }
            _ => {}
        }
    }
}

/// A rough keycode -> ASCII mapping for text input, since this backend has
/// no XKB layout integration. Only covers unshifted/shifted Latin letters,
/// digits, and a handful of punctuation keys - not a real input pipeline.
fn keycode_to_ascii(keycode: u8, state: u16) -> Option<char> {
    let shift = state & 0x01 != 0;
    let c = match keycode {
        24 => 'q',
        25 => 'w',
        26 => 'e',
        27 => 'r',
        28 => 't',
        29 => 'y',
        30 => 'u',
        31 => 'i',
        32 => 'o',
        33 => 'p',
        38 => 'a',
        39 => 's',
        40 => 'd',
        41 => 'f',
        42 => 'g',
        43 => 'h',
        44 => 'j',
        45 => 'k',
        46 => 'l',
        52 => 'z',
        53 => 'x',
        54 => 'c',
        55 => 'v',
        56 => 'b',
        57 => 'n',
        58 => 'm',
        65 => ' ',
        10 => '1',
        11 => '2',
        12 => '3',
        13 => '4',
        14 => '5',
        15 => '6',
        16 => '7',
        17 => '8',
        18 => '9',
        19 => '0',
        _ => return None,
    };
    Some(if shift { c.to_ascii_uppercase() } else { c })
}

/// Linux/X11 window wrapper.
pub struct LinuxWindow {
    conn: Arc<RustConnection>,
    window: Window,
    view: Option<View>,
    state: Rc<WindowState>,
}

impl LinuxWindow {
    /// Creates a new Linux window. Requires an [`LinuxApp`] to already have
    /// been created on this thread (it registers the X11 connection used
    /// here via a thread-local when constructed) - matching every other
    /// `Window::new` caller's expectation of creating the `App` first.
    pub fn new(title: &str, size: Extent) -> Option<Self> {
        let inner = CURRENT_APP.with(|cell| cell.borrow().clone())?;
        let conn = inner.conn.clone();
        let screen = &conn.setup().roots[inner.screen_num];
        let depth = screen.root_depth;

        let bits_per_pixel = conn
            .setup()
            .pixmap_formats
            .iter()
            .find(|f| f.depth == depth)
            .map(|f| f.bits_per_pixel)
            .unwrap_or(32);
        let msb_first = conn.setup().image_byte_order == ImageOrder::MSB_FIRST;

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

        let gc = conn.generate_id().ok()?;
        conn.create_gc(gc, window, &CreateGCAux::default()).ok()?;

        conn.flush().ok()?;

        let state = Rc::new(WindowState {
            canvas: RefCell::new(None),
            content: RefCell::new(None),
            size: RefCell::new(size),
            gc,
            depth,
            bits_per_pixel,
            msb_first,
        });
        inner.windows.borrow_mut().insert(window, state.clone());

        Some(Self {
            conn,
            window,
            view: Some(View::new(size)),
            state,
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

    /// Sets the window title.
    pub fn set_title(&self, title: &str) {
        let _ = self.conn.change_property8(
            PropMode::REPLACE,
            self.window,
            AtomEnum::WM_NAME,
            AtomEnum::STRING,
            title.as_bytes(),
        );
        let _ = self.conn.flush();
    }

    /// Sets the window size.
    pub fn set_size(&self, size: Extent) {
        let aux = ConfigureWindowAux::default()
            .width(size.x as u32)
            .height(size.y as u32);
        let _ = self.conn.configure_window(self.window, &aux);
        let _ = self.conn.flush();
        *self.state.size.borrow_mut() = size;
    }

    /// Sets the window content.
    pub fn set_content(&self, content: ElementPtr) {
        *self.state.content.borrow_mut() = Some(content);
        paint(&self.conn, self.window, &self.state);
    }

    /// Triggers a redraw.
    pub fn refresh(&self) {
        paint(&self.conn, self.window, &self.state);
    }

    /// Returns the raw X window ID (as a pointer-sized value, matching the
    /// convention used for `WindowHandle` elsewhere), for embedding
    /// externally-managed native content into this window instead of using
    /// mkgraphic's own element tree for it.
    pub fn native_window_handle(&self) -> *mut std::ffi::c_void {
        self.window as usize as *mut std::ffi::c_void
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
