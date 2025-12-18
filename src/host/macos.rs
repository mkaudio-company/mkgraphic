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
    NSGraphicsContext, NSEvent, NSMenu, NSMenuItem,
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

        let macos_app = Self { app, mtm };
        macos_app.setup_menu();

        Some(macos_app)
    }

    /// Sets up the application menu bar based on configuration or defaults.
    fn setup_menu(&self) {
        use crate::element::menu::get_native_menu_bar;

        // Check if there's a custom menu bar configuration
        let config = get_native_menu_bar().unwrap_or_else(|| {
            crate::element::menu::NativeMenuBar::new()
        });

        unsafe {
            let main_menu = NSMenu::new(self.mtm);

            // App menu (always included)
            if config.include_app_menu {
                self.add_app_menu(&main_menu);
            }

            // Custom menus (inserted before Edit)
            for custom_menu in &config.menus {
                self.add_custom_menu(&main_menu, custom_menu);
            }

            // Edit menu
            if config.include_edit_menu {
                self.add_edit_menu(&main_menu);
            }

            // Window menu
            if config.include_window_menu {
                self.add_window_menu(&main_menu);
            }

            self.app.setMainMenu(Some(&main_menu));
        }
    }

    /// Adds the standard app menu.
    unsafe fn add_app_menu(&self, main_menu: &NSMenu) {
        let app_menu_item = NSMenuItem::new(self.mtm);
        let app_menu = NSMenu::new(self.mtm);

        // About item
        let about_title = NSString::from_str("About");
        let about_item = NSMenuItem::initWithTitle_action_keyEquivalent(
            self.mtm.alloc(),
            &about_title,
            Some(objc2::sel!(orderFrontStandardAboutPanel:)),
            &NSString::from_str(""),
        );
        app_menu.addItem(&about_item);

        app_menu.addItem(&NSMenuItem::separatorItem(self.mtm));

        // Services menu
        let services_title = NSString::from_str("Services");
        let services_item = NSMenuItem::initWithTitle_action_keyEquivalent(
            self.mtm.alloc(),
            &services_title,
            None,
            &NSString::from_str(""),
        );
        let services_menu = NSMenu::initWithTitle(self.mtm.alloc(), &services_title);
        services_item.setSubmenu(Some(&services_menu));
        app_menu.addItem(&services_item);
        self.app.setServicesMenu(Some(&services_menu));

        app_menu.addItem(&NSMenuItem::separatorItem(self.mtm));

        // Hide item
        let hide_title = NSString::from_str("Hide");
        let hide_item = NSMenuItem::initWithTitle_action_keyEquivalent(
            self.mtm.alloc(),
            &hide_title,
            Some(objc2::sel!(hide:)),
            &NSString::from_str("h"),
        );
        app_menu.addItem(&hide_item);

        // Hide Others item
        let hide_others_title = NSString::from_str("Hide Others");
        let hide_others_item = NSMenuItem::initWithTitle_action_keyEquivalent(
            self.mtm.alloc(),
            &hide_others_title,
            Some(objc2::sel!(hideOtherApplications:)),
            &NSString::from_str("h"),
        );
        hide_others_item.setKeyEquivalentModifierMask(
            objc2_app_kit::NSEventModifierFlags::NSEventModifierFlagCommand
            | objc2_app_kit::NSEventModifierFlags::NSEventModifierFlagOption
        );
        app_menu.addItem(&hide_others_item);

        // Show All item
        let show_all_title = NSString::from_str("Show All");
        let show_all_item = NSMenuItem::initWithTitle_action_keyEquivalent(
            self.mtm.alloc(),
            &show_all_title,
            Some(objc2::sel!(unhideAllApplications:)),
            &NSString::from_str(""),
        );
        app_menu.addItem(&show_all_item);

        app_menu.addItem(&NSMenuItem::separatorItem(self.mtm));

        // Quit item
        let quit_title = NSString::from_str("Quit");
        let quit_item = NSMenuItem::initWithTitle_action_keyEquivalent(
            self.mtm.alloc(),
            &quit_title,
            Some(objc2::sel!(terminate:)),
            &NSString::from_str("q"),
        );
        app_menu.addItem(&quit_item);

        app_menu_item.setSubmenu(Some(&app_menu));
        main_menu.addItem(&app_menu_item);
    }

    /// Adds the standard edit menu.
    unsafe fn add_edit_menu(&self, main_menu: &NSMenu) {
        let edit_menu_item = NSMenuItem::new(self.mtm);
        let edit_title = NSString::from_str("Edit");
        let edit_menu = NSMenu::initWithTitle(self.mtm.alloc(), &edit_title);

        // Undo
        let undo_title = NSString::from_str("Undo");
        let undo_item = NSMenuItem::initWithTitle_action_keyEquivalent(
            self.mtm.alloc(),
            &undo_title,
            Some(objc2::sel!(undo:)),
            &NSString::from_str("z"),
        );
        edit_menu.addItem(&undo_item);

        // Redo
        let redo_title = NSString::from_str("Redo");
        let redo_item = NSMenuItem::initWithTitle_action_keyEquivalent(
            self.mtm.alloc(),
            &redo_title,
            Some(objc2::sel!(redo:)),
            &NSString::from_str("Z"),
        );
        edit_menu.addItem(&redo_item);

        edit_menu.addItem(&NSMenuItem::separatorItem(self.mtm));

        // Cut
        let cut_title = NSString::from_str("Cut");
        let cut_item = NSMenuItem::initWithTitle_action_keyEquivalent(
            self.mtm.alloc(),
            &cut_title,
            Some(objc2::sel!(cut:)),
            &NSString::from_str("x"),
        );
        edit_menu.addItem(&cut_item);

        // Copy
        let copy_title = NSString::from_str("Copy");
        let copy_item = NSMenuItem::initWithTitle_action_keyEquivalent(
            self.mtm.alloc(),
            &copy_title,
            Some(objc2::sel!(copy:)),
            &NSString::from_str("c"),
        );
        edit_menu.addItem(&copy_item);

        // Paste
        let paste_title = NSString::from_str("Paste");
        let paste_item = NSMenuItem::initWithTitle_action_keyEquivalent(
            self.mtm.alloc(),
            &paste_title,
            Some(objc2::sel!(paste:)),
            &NSString::from_str("v"),
        );
        edit_menu.addItem(&paste_item);

        // Select All
        let select_all_title = NSString::from_str("Select All");
        let select_all_item = NSMenuItem::initWithTitle_action_keyEquivalent(
            self.mtm.alloc(),
            &select_all_title,
            Some(objc2::sel!(selectAll:)),
            &NSString::from_str("a"),
        );
        edit_menu.addItem(&select_all_item);

        edit_menu_item.setSubmenu(Some(&edit_menu));
        main_menu.addItem(&edit_menu_item);
    }

    /// Adds the standard window menu.
    unsafe fn add_window_menu(&self, main_menu: &NSMenu) {
        let window_menu_item = NSMenuItem::new(self.mtm);
        let window_title = NSString::from_str("Window");
        let window_menu = NSMenu::initWithTitle(self.mtm.alloc(), &window_title);

        // Minimize
        let minimize_title = NSString::from_str("Minimize");
        let minimize_item = NSMenuItem::initWithTitle_action_keyEquivalent(
            self.mtm.alloc(),
            &minimize_title,
            Some(objc2::sel!(performMiniaturize:)),
            &NSString::from_str("m"),
        );
        window_menu.addItem(&minimize_item);

        // Zoom
        let zoom_title = NSString::from_str("Zoom");
        let zoom_item = NSMenuItem::initWithTitle_action_keyEquivalent(
            self.mtm.alloc(),
            &zoom_title,
            Some(objc2::sel!(performZoom:)),
            &NSString::from_str(""),
        );
        window_menu.addItem(&zoom_item);

        window_menu.addItem(&NSMenuItem::separatorItem(self.mtm));

        // Bring All to Front
        let bring_all_title = NSString::from_str("Bring All to Front");
        let bring_all_item = NSMenuItem::initWithTitle_action_keyEquivalent(
            self.mtm.alloc(),
            &bring_all_title,
            Some(objc2::sel!(arrangeInFront:)),
            &NSString::from_str(""),
        );
        window_menu.addItem(&bring_all_item);

        window_menu_item.setSubmenu(Some(&window_menu));
        main_menu.addItem(&window_menu_item);
        self.app.setWindowsMenu(Some(&window_menu));
    }

    /// Adds a custom menu from NativeMenu configuration.
    unsafe fn add_custom_menu(&self, main_menu: &NSMenu, custom_menu: &crate::element::menu::NativeMenu) {
        let menu_item = NSMenuItem::new(self.mtm);
        let title = NSString::from_str(&custom_menu.title);
        let ns_menu = NSMenu::initWithTitle(self.mtm.alloc(), &title);

        for item in &custom_menu.items {
            self.add_native_menu_item(&ns_menu, item);
        }

        menu_item.setSubmenu(Some(&ns_menu));
        main_menu.addItem(&menu_item);
    }

    /// Adds a native menu item to a menu.
    unsafe fn add_native_menu_item(&self, menu: &NSMenu, item: &crate::element::menu::NativeMenuItem) {
        if item.is_separator() {
            menu.addItem(&NSMenuItem::separatorItem(self.mtm));
            return;
        }

        let title = NSString::from_str(&item.label);
        let key_equiv = item.shortcut.as_ref()
            .map(|s| s.key.to_string())
            .unwrap_or_default();
        let key_str = NSString::from_str(&key_equiv);

        // For items with callbacks, we can't easily map to Objective-C selectors
        // So for now, items without callbacks will have no action
        let ns_item = NSMenuItem::initWithTitle_action_keyEquivalent(
            self.mtm.alloc(),
            &title,
            None, // Custom callbacks require more complex setup
            &key_str,
        );

        // Set modifier mask if there's a shortcut
        if let Some(ref shortcut) = item.shortcut {
            let mut mask = objc2_app_kit::NSEventModifierFlags::empty();
            if shortcut.modifiers.command {
                mask |= objc2_app_kit::NSEventModifierFlags::NSEventModifierFlagCommand;
            }
            if shortcut.modifiers.shift {
                mask |= objc2_app_kit::NSEventModifierFlags::NSEventModifierFlagShift;
            }
            if shortcut.modifiers.option {
                mask |= objc2_app_kit::NSEventModifierFlags::NSEventModifierFlagOption;
            }
            if shortcut.modifiers.control {
                mask |= objc2_app_kit::NSEventModifierFlags::NSEventModifierFlagControl;
            }
            ns_item.setKeyEquivalentModifierMask(mask);
        }

        // Set enabled state
        if !item.enabled {
            ns_item.setEnabled(false);
        }

        // Handle submenu
        if let Some(ref submenu_items) = item.submenu {
            let submenu_title = NSString::from_str(&item.label);
            let submenu = NSMenu::initWithTitle(self.mtm.alloc(), &submenu_title);
            for sub_item in submenu_items {
                self.add_native_menu_item(&submenu, sub_item);
            }
            ns_item.setSubmenu(Some(&submenu));
        }

        menu.addItem(&ns_item);
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

                    // Handle the click first - this allows menus and other controls
                    // to process the click before focus is cleared
                    let handled = content.handle_click(&ctx, mouse_btn);

                    // Clear focus from all elements on mouse down
                    // This ensures text boxes lose focus when clicking elsewhere.
                    // Note: Controls like TextBox will re-establish focus in handle_click
                    // if they were the target of the click.
                    if down {
                        content.clear_focus();
                    }

                    // Trigger redraw
                    self.setNeedsDisplay(true);
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
