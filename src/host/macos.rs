//! macOS platform implementation.
//!
//! This module provides the macOS-specific implementation using Cocoa/AppKit
//! through the objc2 crate.

#![cfg(target_os = "macos")]

use std::cell::RefCell;

use objc2::rc::Retained;
use objc2::{declare_class, msg_send_id, mutability, ClassType, DeclaredClass};
use objc2_foundation::{
    NSString, MainThreadMarker, NSPoint, NSRect, NSSize,
};
use objc2_app_kit::{
    NSApplication, NSApplicationActivationPolicy, NSBackingStoreType,
    NSWindow, NSWindowStyleMask, NSCursor, NSPasteboard, NSView,
    NSGraphicsContext, NSEvent,
};
use core_graphics::color_space::CGColorSpace;
use core_graphics::context::CGContext;
use core_graphics::data_provider::CGDataProvider;
use core_graphics::image::CGImage;

use crate::support::point::{Point, Extent};
use crate::support::canvas::Canvas;
use crate::support::color::Color;
use crate::support::rect::Rect;
use crate::element::context::Context;
use crate::element::ElementPtr;
use crate::view::{View, KeyCode, CursorType, modifiers, MouseButton, MouseButtonKind};

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

/// State for our custom view.
#[derive(Default)]
struct MKViewIvars {
    canvas: RefCell<Option<Canvas>>,
    content: RefCell<Option<ElementPtr>>,
    size: RefCell<Extent>,
}

declare_class!(
    struct MKView;

    unsafe impl ClassType for MKView {
        type Super = NSView;
        type Mutability = mutability::MainThreadOnly;
        const NAME: &'static str = "MKView";
    }

    impl DeclaredClass for MKView {
        type Ivars = MKViewIvars;
    }

    unsafe impl MKView {
        #[method(isFlipped)]
        fn is_flipped(&self) -> bool {
            true
        }

        #[method(acceptsFirstResponder)]
        fn accepts_first_responder(&self) -> bool {
            true
        }

        #[method(mouseDown:)]
        fn mouse_down(&self, event: &NSEvent) {
            self.handle_mouse_event(event, true);
        }

        #[method(mouseUp:)]
        fn mouse_up(&self, event: &NSEvent) {
            self.handle_mouse_event(event, false);
        }

        #[method(rightMouseDown:)]
        fn right_mouse_down(&self, event: &NSEvent) {
            self.handle_mouse_event(event, true);
        }

        #[method(rightMouseUp:)]
        fn right_mouse_up(&self, event: &NSEvent) {
            self.handle_mouse_event(event, false);
        }

        #[method(mouseDragged:)]
        fn mouse_dragged(&self, event: &NSEvent) {
            self.handle_mouse_drag(event);
        }

        #[method(rightMouseDragged:)]
        fn right_mouse_dragged(&self, event: &NSEvent) {
            self.handle_mouse_drag(event);
        }

        #[method(scrollWheel:)]
        fn scroll_wheel(&self, event: &NSEvent) {
            self.handle_scroll(event);
        }

        #[method(keyDown:)]
        fn key_down(&self, event: &NSEvent) {
            self.handle_key_event(event, true);
        }

        #[method(keyUp:)]
        fn key_up(&self, event: &NSEvent) {
            self.handle_key_event(event, false);
        }

        #[method(drawRect:)]
        fn draw_rect(&self, _dirty_rect: NSRect) {
            let ivars = self.ivars();

            // Get actual view frame size
            let frame = self.frame();
            let size = Extent::new(frame.size.width as f32, frame.size.height as f32);
            *ivars.size.borrow_mut() = size;

            let width = size.x as u32;
            let height = size.y as u32;

            if width == 0 || height == 0 {
                return;
            }

            // Create or resize canvas
            {
                let mut canvas_opt = ivars.canvas.borrow_mut();
                let needs_new = match &*canvas_opt {
                    Some(c) => c.width() != width || c.height() != height,
                    None => true,
                };
                if needs_new {
                    *canvas_opt = Canvas::new(width, height);
                }
            }

            // Draw content and blit to screen
            let mut canvas_opt = ivars.canvas.borrow_mut();
            if let Some(ref mut canvas) = *canvas_opt {
                // Clear with dark background
                canvas.clear(Color::new(0.2, 0.2, 0.2, 1.0));

                // Draw elements if we have content
                let content_ref = ivars.content.borrow();
                if let Some(ref content) = *content_ref {
                    let bounds = Rect {
                        left: 0.0,
                        top: 0.0,
                        right: size.x,
                        bottom: size.y,
                    };

                    // Create a temporary view for the context
                    let temp_view = View::new(size);

                    // We need to temporarily move the canvas into a RefCell for the Context
                    // Take canvas out, wrap in RefCell, draw, then put back
                    let temp_canvas = std::mem::replace(canvas, Canvas::new(1, 1).unwrap());
                    let canvas_cell = RefCell::new(temp_canvas);

                    let ctx = Context::new(&temp_view, &canvas_cell, bounds);

                    // Draw the content element
                    content.draw(&ctx);

                    // Get the canvas back
                    *canvas = canvas_cell.into_inner();
                }

                // Blit to screen
                Self::blit_to_screen(canvas, width, height);
            }
        }
    }
);

impl MKView {
    fn new(mtm: MainThreadMarker, size: Extent) -> Retained<Self> {
        let frame = NSRect::new(
            NSPoint::new(0.0, 0.0),
            NSSize::new(size.x as f64, size.y as f64),
        );

        let this = mtm.alloc::<MKView>().set_ivars(MKViewIvars {
            canvas: RefCell::new(None),
            content: RefCell::new(None),
            size: RefCell::new(size),
        });

        unsafe { msg_send_id![super(this), initWithFrame: frame] }
    }

    fn set_content(&self, content: ElementPtr) {
        *self.ivars().content.borrow_mut() = Some(content);
        unsafe { self.setNeedsDisplay(true); }
    }

    fn set_size(&self, size: Extent) {
        *self.ivars().size.borrow_mut() = size;
    }

    fn handle_mouse_event(&self, event: &NSEvent, down: bool) {
        unsafe {
            // Get the mouse location in view coordinates
            let location_in_window = event.locationInWindow();
            let location = self.convertPoint_fromView(location_in_window, None);
            let pos = ns_point_to_point(location);

            // Determine which button
            let button_number = event.buttonNumber();
            let button_kind = match button_number {
                0 => MouseButtonKind::Left,
                1 => MouseButtonKind::Right,
                2 => MouseButtonKind::Middle,
                _ => MouseButtonKind::Left,
            };

            // Create MouseButton event
            let mouse_btn = MouseButton {
                down,
                click_count: event.clickCount() as i32,
                button: button_kind,
                modifiers: translate_flags(event.modifierFlags().bits() as usize),
                pos,
            };

            // Forward to content element
            let ivars = self.ivars();
            let size = *ivars.size.borrow();
            let content_ref = ivars.content.borrow();

            if let Some(ref content) = *content_ref {
                let bounds = Rect {
                    left: 0.0,
                    top: 0.0,
                    right: size.x,
                    bottom: size.y,
                };

                // Create a dummy canvas for the context
                if let Some(dummy_canvas) = Canvas::new(1, 1) {
                    let canvas_cell = RefCell::new(dummy_canvas);
                    let temp_view = View::new(size);
                    let ctx = Context::new(&temp_view, &canvas_cell, bounds);

                    // Clear focus from all elements before handling mouse down
                    // This ensures text boxes and other focusable elements lose focus
                    // when clicking elsewhere. Only do this on mouse down, not mouse up.
                    if down {
                        content.clear_focus();
                    }

                    // Use handle_click for immutable click handling
                    if content.handle_click(&ctx, mouse_btn) {
                        // Trigger redraw after click
                        self.setNeedsDisplay(true);
                    }
                }
            }
        }
    }

    fn handle_mouse_drag(&self, event: &NSEvent) {
        unsafe {
            let location_in_window = event.locationInWindow();
            let location = self.convertPoint_fromView(location_in_window, None);
            let pos = ns_point_to_point(location);

            let button_number = event.buttonNumber();
            let button_kind = match button_number {
                0 => MouseButtonKind::Left,
                1 => MouseButtonKind::Right,
                2 => MouseButtonKind::Middle,
                _ => MouseButtonKind::Left,
            };

            let mouse_btn = MouseButton {
                down: true,
                click_count: 1,
                button: button_kind,
                modifiers: translate_flags(event.modifierFlags().bits() as usize),
                pos,
            };

            let ivars = self.ivars();
            let size = *ivars.size.borrow();

            // For drag, we need mutable access to the content
            // We use a RwLock pattern here through the ElementPtr (Arc<RwLock<dyn Element>>)
            let content_ref = ivars.content.borrow();
            if let Some(ref content) = *content_ref {
                let bounds = Rect {
                    left: 0.0,
                    top: 0.0,
                    right: size.x,
                    bottom: size.y,
                };

                if let Some(dummy_canvas) = Canvas::new(1, 1) {
                    let canvas_cell = RefCell::new(dummy_canvas);
                    let temp_view = View::new(size);
                    let ctx = Context::new(&temp_view, &canvas_cell, bounds);

                    // Call handle_drag on the content (immutable version)
                    content.handle_drag(&ctx, mouse_btn);
                    self.setNeedsDisplay(true);
                }
            }
        }
    }

    fn handle_scroll(&self, event: &NSEvent) {
        unsafe {
            let location_in_window = event.locationInWindow();
            let location = self.convertPoint_fromView(location_in_window, None);
            let pos = ns_point_to_point(location);

            let delta_x = event.scrollingDeltaX() as f32;
            let delta_y = event.scrollingDeltaY() as f32;
            let dir = Point::new(delta_x, delta_y);

            let ivars = self.ivars();
            let size = *ivars.size.borrow();
            let content_ref = ivars.content.borrow();

            if let Some(ref content) = *content_ref {
                let bounds = Rect {
                    left: 0.0,
                    top: 0.0,
                    right: size.x,
                    bottom: size.y,
                };

                if let Some(dummy_canvas) = Canvas::new(1, 1) {
                    let canvas_cell = RefCell::new(dummy_canvas);
                    let temp_view = View::new(size);
                    let ctx = Context::new(&temp_view, &canvas_cell, bounds);

                    if content.handle_scroll(&ctx, dir, pos) {
                        self.setNeedsDisplay(true);
                    }
                }
            }
        }
    }

    fn handle_key_event(&self, event: &NSEvent, down: bool) {
        unsafe {
            use crate::view::{KeyInfo, KeyAction};

            let keycode = event.keyCode();
            let key = translate_key(keycode);
            let modifiers = translate_flags(event.modifierFlags().bits() as usize);

            let action = if down { KeyAction::Press } else { KeyAction::Release };

            let key_info = KeyInfo {
                key,
                action,
                modifiers,
            };

            let ivars = self.ivars();
            let size = *ivars.size.borrow();
            let content_ref = ivars.content.borrow();

            if let Some(ref content) = *content_ref {
                let bounds = Rect {
                    left: 0.0,
                    top: 0.0,
                    right: size.x,
                    bottom: size.y,
                };

                if let Some(dummy_canvas) = Canvas::new(1, 1) {
                    let canvas_cell = RefCell::new(dummy_canvas);
                    let temp_view = View::new(size);
                    let ctx = Context::new(&temp_view, &canvas_cell, bounds);

                    if content.handle_key(&ctx, key_info) {
                        self.setNeedsDisplay(true);
                    }
                }
            }

            // Also handle text input for keyDown events
            if down {
                if let Some(characters) = event.characters() {
                    let text: String = characters.to_string();
                    if !text.is_empty() {
                        for c in text.chars() {
                            // Skip control characters
                            if c.is_control() && c != '\n' && c != '\t' {
                                continue;
                            }

                            let text_info = crate::view::TextInfo {
                                codepoint: c,
                                modifiers,
                            };

                            let content_ref = ivars.content.borrow();
                            if let Some(ref content) = *content_ref {
                                let bounds = Rect {
                                    left: 0.0,
                                    top: 0.0,
                                    right: size.x,
                                    bottom: size.y,
                                };

                                if let Some(dummy_canvas) = Canvas::new(1, 1) {
                                    let canvas_cell = RefCell::new(dummy_canvas);
                                    let temp_view = View::new(size);
                                    let ctx = Context::new(&temp_view, &canvas_cell, bounds);

                                    if content.handle_text(&ctx, text_info) {
                                        self.setNeedsDisplay(true);
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    fn blit_to_screen(canvas: &Canvas, width: u32, height: u32) {
        unsafe {
            // Get the current graphics context
            let Some(ns_ctx) = NSGraphicsContext::currentContext() else {
                return;
            };

            // Get the CGContextRef - use graphicsPort for compatibility
            // (CGContext property returns an objc2 type that doesn't match)
            let cg_ctx_ptr: *mut std::ffi::c_void = objc2::msg_send![&ns_ctx, graphicsPort];
            if cg_ctx_ptr.is_null() {
                return;
            }

            // Get pixmap data - tiny-skia stores premultiplied RGBA
            let pixmap = canvas.pixmap();
            let data = pixmap.data();

            // Create CGImage from our pixmap
            let color_space = CGColorSpace::create_device_rgb();
            let provider = CGDataProvider::from_slice(data);

            // tiny-skia uses premultiplied RGBA in native byte order
            // On macOS (little-endian), we need:
            // kCGImageAlphaPremultipliedLast (1) = RGBA with premultiplied alpha
            // kCGBitmapByteOrderDefault (0) = native byte order
            // Combined: just 1 for standard RGBA premultiplied
            let cg_image = CGImage::new(
                width as usize,
                height as usize,
                8,
                32,
                width as usize * 4,
                &color_space,
                1, // kCGImageAlphaPremultipliedLast (RGBA order)
                &provider,
                false,
                0, // kCGRenderingIntentDefault
            );

            let rect = core_graphics::geometry::CGRect::new(
                &core_graphics::geometry::CGPoint::new(0.0, 0.0),
                &core_graphics::geometry::CGSize::new(width as f64, height as f64),
            );

            let cg_ctx = CGContext::from_existing_context_ptr(cg_ctx_ptr as *mut _);

            // Flip the context to match our top-left origin coordinate system
            // Core Graphics has origin at bottom-left, we need top-left
            cg_ctx.save();
            cg_ctx.translate(0.0, height as f64);
            cg_ctx.scale(1.0, -1.0);
            cg_ctx.draw_image(rect, &cg_image);
            cg_ctx.restore();
        }
    }
}

/// macOS window wrapper.
pub struct MacOSWindow {
    window: Retained<NSWindow>,
    mk_view: Retained<MKView>,
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

        // Create our custom view
        let mk_view = MKView::new(mtm, size);
        window.setContentView(Some(&mk_view));

        Self {
            window,
            mk_view,
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
        self.mk_view.set_size(size);
    }

    /// Sets the window content.
    pub fn set_content(&self, content: ElementPtr) {
        self.mk_view.set_content(content);
    }

    /// Returns a reference to the view.
    pub fn view(&self) -> Option<&View> {
        self.view.as_ref()
    }

    /// Returns a mutable reference to the view.
    pub fn view_mut(&mut self) -> Option<&mut View> {
        self.view.as_mut()
    }

    /// Triggers a redraw.
    pub fn refresh(&self) {
        unsafe { self.mk_view.setNeedsDisplay(true); }
    }
}
