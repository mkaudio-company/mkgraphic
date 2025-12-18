//! Menu and popup elements.

use std::any::Any;
use std::sync::{RwLock, Arc, OnceLock};
use super::{Element, ElementPtr, ViewLimits, ViewStretch, share};
use super::context::{BasicContext, Context};
use crate::support::point::Point;
use crate::support::rect::Rect;
use crate::support::color::Color;
use crate::support::theme::get_theme;
use crate::view::{MouseButton, MouseButtonKind, CursorTracking};

/// Menu item callback type.
pub type MenuItemCallback = Box<dyn Fn() + Send + Sync>;

/// Keyboard modifier flags for menu shortcuts.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct MenuModifiers {
    pub command: bool,
    pub shift: bool,
    pub option: bool,
    pub control: bool,
}

impl MenuModifiers {
    /// No modifiers.
    pub fn none() -> Self {
        Self::default()
    }

    /// Command key (Cmd on macOS, Ctrl on other platforms).
    pub fn command() -> Self {
        Self { command: true, ..Default::default() }
    }

    /// Shift key.
    pub fn shift() -> Self {
        Self { shift: true, ..Default::default() }
    }

    /// Option key (Alt on other platforms).
    pub fn option() -> Self {
        Self { option: true, ..Default::default() }
    }

    /// Control key.
    pub fn control() -> Self {
        Self { control: true, ..Default::default() }
    }

    /// Adds command modifier.
    pub fn with_command(mut self) -> Self {
        self.command = true;
        self
    }

    /// Adds shift modifier.
    pub fn with_shift(mut self) -> Self {
        self.shift = true;
        self
    }

    /// Adds option modifier.
    pub fn with_option(mut self) -> Self {
        self.option = true;
        self
    }

    /// Adds control modifier.
    pub fn with_control(mut self) -> Self {
        self.control = true;
        self
    }
}

/// A keyboard shortcut for a menu item.
#[derive(Debug, Clone)]
pub struct MenuShortcut {
    /// The key character (e.g., 'n', 'o', 's').
    pub key: char,
    /// The modifier keys.
    pub modifiers: MenuModifiers,
}

impl MenuShortcut {
    /// Creates a new shortcut with Cmd modifier (standard for macOS).
    pub fn cmd(key: char) -> Self {
        Self {
            key,
            modifiers: MenuModifiers::command(),
        }
    }

    /// Creates a new shortcut with Cmd+Shift modifiers.
    pub fn cmd_shift(key: char) -> Self {
        Self {
            key,
            modifiers: MenuModifiers::command().with_shift(),
        }
    }

    /// Creates a new shortcut with Cmd+Option modifiers.
    pub fn cmd_option(key: char) -> Self {
        Self {
            key,
            modifiers: MenuModifiers::command().with_option(),
        }
    }

    /// Creates a new shortcut with custom modifiers.
    pub fn with_modifiers(key: char, modifiers: MenuModifiers) -> Self {
        Self { key, modifiers }
    }

    /// Returns the display string for this shortcut.
    pub fn display_string(&self) -> String {
        let mut s = String::new();
        if self.modifiers.control {
            s.push_str("Ctrl+");
        }
        if self.modifiers.option {
            s.push_str("Opt+");
        }
        if self.modifiers.shift {
            s.push_str("Shift+");
        }
        if self.modifiers.command {
            s.push_str("Cmd+");
        }
        s.push(self.key.to_ascii_uppercase());
        s
    }
}

/// A menu item.
pub struct MenuItem {
    label: String,
    shortcut: Option<String>,
    enabled: bool,
    checked: bool,
    submenu: Option<Vec<MenuItem>>,
    on_select: Option<MenuItemCallback>,
    hover: RwLock<bool>,
}

impl MenuItem {
    /// Creates a new menu item.
    pub fn new(label: impl Into<String>) -> Self {
        Self {
            label: label.into(),
            shortcut: None,
            enabled: true,
            checked: false,
            submenu: None,
            on_select: None,
            hover: RwLock::new(false),
        }
    }

    /// Creates a separator item.
    pub fn separator() -> Self {
        Self {
            label: String::new(),
            shortcut: None,
            enabled: false,
            checked: false,
            submenu: None,
            on_select: None,
            hover: RwLock::new(false),
        }
    }

    /// Sets the keyboard shortcut display text.
    pub fn shortcut(mut self, shortcut: impl Into<String>) -> Self {
        self.shortcut = Some(shortcut.into());
        self
    }

    /// Sets whether the item is enabled.
    pub fn enabled(mut self, enabled: bool) -> Self {
        self.enabled = enabled;
        self
    }

    /// Sets the checked state.
    pub fn checked(mut self, checked: bool) -> Self {
        self.checked = checked;
        self
    }

    /// Adds a submenu.
    pub fn submenu(mut self, items: Vec<MenuItem>) -> Self {
        self.submenu = Some(items);
        self
    }

    /// Sets the selection callback.
    pub fn on_select<F: Fn() + Send + Sync + 'static>(mut self, callback: F) -> Self {
        self.on_select = Some(Box::new(callback));
        self
    }

    /// Returns whether this is a separator.
    pub fn is_separator(&self) -> bool {
        self.label.is_empty()
    }

    /// Returns the label.
    pub fn label(&self) -> &str {
        &self.label
    }

    fn height(&self) -> f32 {
        if self.is_separator() {
            8.0
        } else {
            28.0
        }
    }
}

/// A popup menu element.
pub struct Menu {
    items: Vec<MenuItem>,
    background_color: Color,
    hover_color: Color,
    text_color: Color,
    disabled_color: Color,
    check_color: Color,
    separator_color: Color,
    corner_radius: f32,
    padding: f32,
    min_width: f32,
    visible: RwLock<bool>,
    hovered_index: RwLock<Option<usize>>,
}

impl Menu {
    /// Creates a new menu.
    pub fn new(items: Vec<MenuItem>) -> Self {
        let theme = get_theme();
        Self {
            items,
            background_color: theme.menu_background_color,
            hover_color: theme.menu_item_hilite_color,
            text_color: theme.menu_font_color,
            disabled_color: theme.menu_font_color.with_alpha(0.5),
            check_color: theme.indicator_bright_color,
            separator_color: theme.menu_separator_color,
            corner_radius: 6.0,
            padding: 4.0,
            min_width: 150.0,
            visible: RwLock::new(false),
            hovered_index: RwLock::new(None),
        }
    }

    /// Sets the background color.
    pub fn background_color(mut self, color: Color) -> Self {
        self.background_color = color;
        self
    }

    /// Sets the minimum width.
    pub fn min_width(mut self, width: f32) -> Self {
        self.min_width = width;
        self
    }

    /// Shows the menu.
    pub fn show(&self) {
        *self.visible.write().unwrap() = true;
    }

    /// Hides the menu.
    pub fn hide(&self) {
        *self.visible.write().unwrap() = false;
        *self.hovered_index.write().unwrap() = None;
    }

    /// Returns whether the menu is visible.
    pub fn is_visible(&self) -> bool {
        *self.visible.read().unwrap()
    }

    fn calculate_size(&self) -> (f32, f32) {
        let theme = get_theme();

        let mut max_width = self.min_width;
        let mut total_height = self.padding * 2.0;

        for item in &self.items {
            let text_width = item.label.len() as f32 * theme.menu_font_size * 0.6;
            let shortcut_width = item.shortcut.as_ref()
                .map(|s| s.len() as f32 * theme.menu_font_size * 0.5 + 20.0)
                .unwrap_or(0.0);

            let item_width = self.padding * 2.0 + 24.0 + text_width + shortcut_width + 8.0;
            max_width = max_width.max(item_width);
            total_height += item.height();
        }

        (max_width, total_height)
    }

    fn item_bounds(&self, ctx: &Context, index: usize) -> Rect {
        let mut y = ctx.bounds.top + self.padding;

        for (i, item) in self.items.iter().enumerate() {
            let height = item.height();
            if i == index {
                return Rect::new(
                    ctx.bounds.left + self.padding,
                    y,
                    ctx.bounds.right - self.padding,
                    y + height,
                );
            }
            y += height;
        }

        Rect::zero()
    }

    fn draw_background(&self, ctx: &Context) {
        let mut canvas = ctx.canvas.borrow_mut();

        // Drop shadow
        let shadow_rect = ctx.bounds.translate(2.0, 2.0);
        canvas.fill_style(Color::new(0.0, 0.0, 0.0, 0.3));
        canvas.fill_round_rect(shadow_rect, self.corner_radius);

        // Background
        canvas.fill_style(self.background_color);
        canvas.fill_round_rect(ctx.bounds, self.corner_radius);
    }

    fn draw_item(&self, ctx: &Context, item: &MenuItem, bounds: Rect, hovered: bool) {
        let mut canvas = ctx.canvas.borrow_mut();
        let theme = get_theme();

        if item.is_separator() {
            // Draw separator line
            let y = bounds.center().y;
            canvas.stroke_style(self.separator_color);
            canvas.line_width(1.0);
            canvas.begin_path();
            canvas.move_to(Point::new(bounds.left + 8.0, y));
            canvas.line_to(Point::new(bounds.right - 8.0, y));
            canvas.stroke();
            return;
        }

        // Highlight if hovered
        if hovered && item.enabled {
            canvas.fill_style(self.hover_color);
            canvas.fill_round_rect(bounds, 4.0);
        }

        let text_color = if item.enabled {
            self.text_color
        } else {
            self.disabled_color
        };

        // Draw checkmark if checked
        if item.checked {
            canvas.fill_style(self.check_color);
            let check_x = bounds.left + 8.0;
            let check_y = bounds.center().y;
            canvas.fill_text("✓", Point::new(check_x, check_y + 4.0));
        }

        // Draw label
        canvas.fill_style(text_color);
        canvas.font_size(theme.menu_font_size);
        let x = bounds.left + 24.0;
        let y = bounds.center().y + theme.menu_font_size * 0.35;
        canvas.fill_text(&item.label, Point::new(x, y));

        // Draw shortcut
        if let Some(ref shortcut) = item.shortcut {
            let shortcut_color = text_color.with_alpha(0.6);
            canvas.fill_style(shortcut_color);
            let shortcut_x = bounds.right - 8.0 - shortcut.len() as f32 * theme.menu_font_size * 0.5;
            canvas.fill_text(shortcut, Point::new(shortcut_x, y));
        }

        // Draw submenu arrow
        if item.submenu.is_some() {
            canvas.fill_style(text_color);
            canvas.fill_text("▶", Point::new(bounds.right - 16.0, y));
        }
    }
}

impl Element for Menu {
    fn limits(&self, _ctx: &BasicContext) -> ViewLimits {
        let (width, height) = self.calculate_size();
        ViewLimits::fixed(width, height)
    }

    fn stretch(&self) -> ViewStretch {
        ViewStretch::new(0.0, 0.0)
    }

    fn draw(&self, ctx: &Context) {
        if !self.is_visible() {
            return;
        }

        self.draw_background(ctx);

        let hovered = *self.hovered_index.read().unwrap();
        for (i, item) in self.items.iter().enumerate() {
            let bounds = self.item_bounds(ctx, i);
            let is_hovered = hovered == Some(i);
            self.draw_item(ctx, item, bounds, is_hovered);
        }
    }

    fn hit_test(&self, ctx: &Context, p: Point, _leaf: bool, _control: bool) -> Option<&dyn Element> {
        if self.is_visible() && ctx.bounds.contains(p) {
            Some(self)
        } else {
            None
        }
    }

    fn wants_control(&self) -> bool {
        self.is_visible()
    }

    fn handle_click(&self, ctx: &Context, btn: MouseButton) -> bool {
        if !self.is_visible() || btn.button != MouseButtonKind::Left {
            return false;
        }

        if !btn.down {
            // Find clicked item
            for (i, item) in self.items.iter().enumerate() {
                if !item.is_separator() && item.enabled {
                    let bounds = self.item_bounds(ctx, i);
                    if bounds.contains(btn.pos) {
                        if let Some(ref callback) = item.on_select {
                            callback();
                        }
                        self.hide();
                        return true;
                    }
                }
            }

            // Click outside menu closes it
            if !ctx.bounds.contains(btn.pos) {
                self.hide();
            }
        }

        true
    }

    fn cursor(&mut self, ctx: &Context, p: Point, status: CursorTracking) -> bool {
        if !self.is_visible() {
            return false;
        }

        match status {
            CursorTracking::Leaving => {
                *self.hovered_index.write().unwrap() = None;
            }
            _ => {
                let mut hovered = self.hovered_index.write().unwrap();
                *hovered = None;

                for (i, item) in self.items.iter().enumerate() {
                    if !item.is_separator() {
                        let bounds = self.item_bounds(ctx, i);
                        if bounds.contains(p) {
                            *hovered = Some(i);
                            break;
                        }
                    }
                }
            }
        }

        true
    }

    fn as_any(&self) -> &dyn Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }
}

/// A popup container element that can show/hide content.
pub struct Popup {
    content: Option<ElementPtr>,
    visible: RwLock<bool>,
    background_color: Color,
    corner_radius: f32,
    shadow: bool,
}

impl Popup {
    /// Creates a new popup.
    pub fn new() -> Self {
        let theme = get_theme();
        Self {
            content: None,
            visible: RwLock::new(false),
            background_color: theme.menu_background_color,
            corner_radius: 8.0,
            shadow: true,
        }
    }

    /// Sets the popup content.
    pub fn content<E: Element + 'static>(mut self, content: E) -> Self {
        self.content = Some(share(content));
        self
    }

    /// Sets the background color.
    pub fn background_color(mut self, color: Color) -> Self {
        self.background_color = color;
        self
    }

    /// Sets whether to show a shadow.
    pub fn shadow(mut self, shadow: bool) -> Self {
        self.shadow = shadow;
        self
    }

    /// Shows the popup.
    pub fn show(&self) {
        *self.visible.write().unwrap() = true;
    }

    /// Hides the popup.
    pub fn hide(&self) {
        *self.visible.write().unwrap() = false;
    }

    /// Returns whether the popup is visible.
    pub fn is_visible(&self) -> bool {
        *self.visible.read().unwrap()
    }

    /// Toggles visibility.
    pub fn toggle(&self) {
        let mut visible = self.visible.write().unwrap();
        *visible = !*visible;
    }
}

impl Default for Popup {
    fn default() -> Self {
        Self::new()
    }
}

impl Element for Popup {
    fn limits(&self, ctx: &BasicContext) -> ViewLimits {
        if let Some(ref content) = self.content {
            content.limits(ctx)
        } else {
            ViewLimits::fixed(100.0, 100.0)
        }
    }

    fn stretch(&self) -> ViewStretch {
        ViewStretch::new(0.0, 0.0)
    }

    fn draw(&self, ctx: &Context) {
        if !self.is_visible() {
            return;
        }

        let mut canvas = ctx.canvas.borrow_mut();

        // Shadow
        if self.shadow {
            let shadow_rect = ctx.bounds.translate(3.0, 3.0);
            canvas.fill_style(Color::new(0.0, 0.0, 0.0, 0.25));
            canvas.fill_round_rect(shadow_rect, self.corner_radius);
        }

        // Background
        canvas.fill_style(self.background_color);
        canvas.fill_round_rect(ctx.bounds, self.corner_radius);

        drop(canvas);

        // Draw content
        if let Some(ref content) = self.content {
            let inset = 8.0;
            let content_bounds = ctx.bounds.inset(inset, inset);
            let content_ctx = ctx.with_bounds(content_bounds);
            content.draw(&content_ctx);
        }
    }

    fn hit_test(&self, ctx: &Context, p: Point, leaf: bool, control: bool) -> Option<&dyn Element> {
        if !self.is_visible() {
            return None;
        }

        if ctx.bounds.contains(p) {
            if let Some(ref content) = self.content {
                let inset = 8.0;
                let content_bounds = ctx.bounds.inset(inset, inset);
                let content_ctx = ctx.with_bounds(content_bounds);
                if let Some(hit) = content.hit_test(&content_ctx, p, leaf, control) {
                    return Some(hit);
                }
            }
            Some(self)
        } else {
            None
        }
    }

    fn wants_control(&self) -> bool {
        self.is_visible()
    }

    fn handle_click(&self, ctx: &Context, btn: MouseButton) -> bool {
        if !self.is_visible() {
            return false;
        }

        // Click outside closes popup
        if !btn.down && !ctx.bounds.contains(btn.pos) {
            self.hide();
            return true;
        }

        // Forward to content
        if let Some(ref content) = self.content {
            let inset = 8.0;
            let content_bounds = ctx.bounds.inset(inset, inset);
            let content_ctx = ctx.with_bounds(content_bounds);
            if content.handle_click(&content_ctx, btn) {
                return true;
            }
        }

        true
    }

    fn as_any(&self) -> &dyn Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }
}

// =============================================================================
// Native Menu Bar API
// =============================================================================

/// A native menu item for the OS menu bar.
#[derive(Clone)]
pub struct NativeMenuItem {
    /// The label text.
    pub label: String,
    /// The keyboard shortcut.
    pub shortcut: Option<MenuShortcut>,
    /// Whether this item is enabled.
    pub enabled: bool,
    /// Whether this item is checked.
    pub checked: bool,
    /// Submenu items (if this is a submenu).
    pub submenu: Option<Vec<NativeMenuItem>>,
    /// The action callback.
    pub action: Option<Arc<dyn Fn() + Send + Sync>>,
    /// Unique identifier for this item.
    pub id: Option<String>,
}

impl NativeMenuItem {
    /// Creates a new native menu item.
    pub fn new(label: impl Into<String>) -> Self {
        Self {
            label: label.into(),
            shortcut: None,
            enabled: true,
            checked: false,
            submenu: None,
            action: None,
            id: None,
        }
    }

    /// Creates a separator item.
    pub fn separator() -> Self {
        Self {
            label: String::new(),
            shortcut: None,
            enabled: false,
            checked: false,
            submenu: None,
            action: None,
            id: None,
        }
    }

    /// Returns whether this is a separator.
    pub fn is_separator(&self) -> bool {
        self.label.is_empty() && self.submenu.is_none()
    }

    /// Sets the keyboard shortcut with Cmd modifier.
    pub fn shortcut_cmd(mut self, key: char) -> Self {
        self.shortcut = Some(MenuShortcut::cmd(key));
        self
    }

    /// Sets the keyboard shortcut with Cmd+Shift modifiers.
    pub fn shortcut_cmd_shift(mut self, key: char) -> Self {
        self.shortcut = Some(MenuShortcut::cmd_shift(key));
        self
    }

    /// Sets the keyboard shortcut with Cmd+Option modifiers.
    pub fn shortcut_cmd_option(mut self, key: char) -> Self {
        self.shortcut = Some(MenuShortcut::cmd_option(key));
        self
    }

    /// Sets a custom keyboard shortcut.
    pub fn shortcut(mut self, shortcut: MenuShortcut) -> Self {
        self.shortcut = Some(shortcut);
        self
    }

    /// Sets whether this item is enabled.
    pub fn enabled(mut self, enabled: bool) -> Self {
        self.enabled = enabled;
        self
    }

    /// Sets the checked state.
    pub fn checked(mut self, checked: bool) -> Self {
        self.checked = checked;
        self
    }

    /// Sets a submenu.
    pub fn submenu(mut self, items: Vec<NativeMenuItem>) -> Self {
        self.submenu = Some(items);
        self
    }

    /// Sets the action callback.
    pub fn on_select<F: Fn() + Send + Sync + 'static>(mut self, callback: F) -> Self {
        self.action = Some(Arc::new(callback));
        self
    }

    /// Sets a unique identifier.
    pub fn id(mut self, id: impl Into<String>) -> Self {
        self.id = Some(id.into());
        self
    }
}

/// A native menu category (top-level menu in the menu bar).
#[derive(Clone)]
pub struct NativeMenu {
    /// The menu title (shown in the menu bar).
    pub title: String,
    /// The menu items.
    pub items: Vec<NativeMenuItem>,
}

impl NativeMenu {
    /// Creates a new native menu.
    pub fn new(title: impl Into<String>) -> Self {
        Self {
            title: title.into(),
            items: Vec::new(),
        }
    }

    /// Creates a new native menu with items.
    pub fn with_items(title: impl Into<String>, items: Vec<NativeMenuItem>) -> Self {
        Self {
            title: title.into(),
            items,
        }
    }

    /// Adds an item to this menu.
    pub fn add_item(mut self, item: NativeMenuItem) -> Self {
        self.items.push(item);
        self
    }

    /// Adds a separator.
    pub fn add_separator(mut self) -> Self {
        self.items.push(NativeMenuItem::separator());
        self
    }

    /// Adds multiple items.
    pub fn add_items(mut self, items: Vec<NativeMenuItem>) -> Self {
        self.items.extend(items);
        self
    }
}

/// Standard menu actions that can be handled by the system.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StandardAction {
    /// About menu item.
    About,
    /// Preferences/Settings.
    Preferences,
    /// Hide application.
    Hide,
    /// Hide other applications.
    HideOthers,
    /// Show all applications.
    ShowAll,
    /// Quit application.
    Quit,
    /// Undo action.
    Undo,
    /// Redo action.
    Redo,
    /// Cut to clipboard.
    Cut,
    /// Copy to clipboard.
    Copy,
    /// Paste from clipboard.
    Paste,
    /// Select all.
    SelectAll,
    /// Delete.
    Delete,
    /// Minimize window.
    Minimize,
    /// Zoom window.
    Zoom,
    /// Bring all windows to front.
    BringAllToFront,
    /// Close window.
    Close,
}

/// Configuration for the native menu bar.
#[derive(Clone, Default)]
pub struct NativeMenuBar {
    /// The application name (shown in the app menu).
    pub app_name: Option<String>,
    /// Custom menus to add.
    pub menus: Vec<NativeMenu>,
    /// Whether to include standard app menu items.
    pub include_app_menu: bool,
    /// Whether to include standard edit menu.
    pub include_edit_menu: bool,
    /// Whether to include standard window menu.
    pub include_window_menu: bool,
}

impl NativeMenuBar {
    /// Creates a new native menu bar configuration.
    pub fn new() -> Self {
        Self {
            app_name: None,
            menus: Vec::new(),
            include_app_menu: true,
            include_edit_menu: true,
            include_window_menu: true,
        }
    }

    /// Sets the application name.
    pub fn app_name(mut self, name: impl Into<String>) -> Self {
        self.app_name = Some(name.into());
        self
    }

    /// Adds a custom menu.
    pub fn add_menu(mut self, menu: NativeMenu) -> Self {
        self.menus.push(menu);
        self
    }

    /// Sets whether to include the standard app menu.
    pub fn include_app_menu(mut self, include: bool) -> Self {
        self.include_app_menu = include;
        self
    }

    /// Sets whether to include the standard edit menu.
    pub fn include_edit_menu(mut self, include: bool) -> Self {
        self.include_edit_menu = include;
        self
    }

    /// Sets whether to include the standard window menu.
    pub fn include_window_menu(mut self, include: bool) -> Self {
        self.include_window_menu = include;
        self
    }

    /// Creates a File menu with common items.
    pub fn file_menu(items: Vec<NativeMenuItem>) -> NativeMenu {
        NativeMenu::with_items("File", items)
    }

    /// Creates a standard File menu with New, Open, Save, etc.
    pub fn standard_file_menu() -> NativeMenu {
        NativeMenu::with_items("File", vec![
            NativeMenuItem::new("New").shortcut_cmd('n'),
            NativeMenuItem::new("Open...").shortcut_cmd('o'),
            NativeMenuItem::separator(),
            NativeMenuItem::new("Save").shortcut_cmd('s'),
            NativeMenuItem::new("Save As...").shortcut_cmd_shift('s'),
            NativeMenuItem::separator(),
            NativeMenuItem::new("Close").shortcut_cmd('w'),
        ])
    }

    /// Creates a View menu.
    pub fn view_menu(items: Vec<NativeMenuItem>) -> NativeMenu {
        NativeMenu::with_items("View", items)
    }

    /// Creates a Help menu.
    pub fn help_menu(items: Vec<NativeMenuItem>) -> NativeMenu {
        NativeMenu::with_items("Help", items)
    }
}

/// Global storage for the native menu bar configuration.
static NATIVE_MENU_BAR: OnceLock<RwLock<Option<NativeMenuBar>>> = OnceLock::new();

/// Sets the global native menu bar configuration.
/// This should be called before App::run() to configure the menu bar.
pub fn set_native_menu_bar(menu_bar: NativeMenuBar) {
    let storage = NATIVE_MENU_BAR.get_or_init(|| RwLock::new(None));
    *storage.write().unwrap() = Some(menu_bar);
}

/// Gets the current native menu bar configuration.
pub fn get_native_menu_bar() -> Option<NativeMenuBar> {
    NATIVE_MENU_BAR.get()
        .and_then(|storage| storage.read().ok())
        .and_then(|guard| guard.clone())
}

// =============================================================================
// Factory Functions
// =============================================================================

/// Creates a menu.
pub fn menu(items: Vec<MenuItem>) -> Menu {
    Menu::new(items)
}

/// Creates a menu item.
pub fn menu_item(label: impl Into<String>) -> MenuItem {
    MenuItem::new(label)
}

/// Creates a menu separator.
pub fn menu_separator() -> MenuItem {
    MenuItem::separator()
}

/// Creates a popup.
pub fn popup() -> Popup {
    Popup::new()
}

/// Creates a native menu item.
pub fn native_menu_item(label: impl Into<String>) -> NativeMenuItem {
    NativeMenuItem::new(label)
}

/// Creates a native menu item separator.
pub fn native_separator() -> NativeMenuItem {
    NativeMenuItem::separator()
}

/// Creates a native menu.
pub fn native_menu(title: impl Into<String>) -> NativeMenu {
    NativeMenu::new(title)
}

/// Creates a native menu bar configuration.
pub fn native_menu_bar() -> NativeMenuBar {
    NativeMenuBar::new()
}
