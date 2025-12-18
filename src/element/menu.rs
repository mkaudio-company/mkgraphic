//! Menu and popup elements.

use std::any::Any;
use std::sync::RwLock;
use super::{Element, ElementPtr, ViewLimits, ViewStretch, share};
use super::context::{BasicContext, Context};
use crate::support::point::Point;
use crate::support::rect::Rect;
use crate::support::color::Color;
use crate::support::theme::get_theme;
use crate::view::{MouseButton, MouseButtonKind, CursorTracking};

/// Menu item callback type.
pub type MenuItemCallback = Box<dyn Fn() + Send + Sync>;

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
