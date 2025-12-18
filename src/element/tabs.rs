//! Tab element for switching between views.

use std::any::Any;
use std::sync::RwLock;
use super::{Element, ElementPtr, ViewLimits, ViewStretch, share};
use super::context::{BasicContext, Context};
use crate::support::point::Point;
use crate::support::rect::Rect;
use crate::support::color::Color;
use crate::support::theme::get_theme;
use crate::view::{MouseButton, MouseButtonKind, CursorTracking};

/// Tab position.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum TabPosition {
    #[default]
    Top,
    Bottom,
    Left,
    Right,
}

/// Callback type for tab changes.
pub type TabChangeCallback = Box<dyn Fn(usize) + Send + Sync>;

/// A single tab.
pub struct Tab {
    label: String,
    content: Option<ElementPtr>,
}

impl Tab {
    /// Creates a new tab.
    pub fn new(label: impl Into<String>) -> Self {
        Self {
            label: label.into(),
            content: None,
        }
    }

    /// Sets the tab content.
    pub fn content<E: Element + 'static>(mut self, content: E) -> Self {
        self.content = Some(share(content));
        self
    }
}

/// A tabbed container element.
pub struct TabBar {
    tabs: Vec<Tab>,
    active_index: RwLock<usize>,
    hovered_index: RwLock<Option<usize>>,
    position: TabPosition,
    active_color: Color,
    inactive_color: Color,
    hover_color: Color,
    text_color: Color,
    background_color: Color,
    tab_height: f32,
    tab_padding: f32,
    corner_radius: f32,
    on_change: Option<TabChangeCallback>,
}

impl TabBar {
    /// Creates a new tab bar.
    pub fn new() -> Self {
        let theme = get_theme();
        Self {
            tabs: Vec::new(),
            active_index: RwLock::new(0),
            hovered_index: RwLock::new(None),
            position: TabPosition::Top,
            active_color: theme.active_tab_color,
            inactive_color: theme.inactive_tab_color,
            hover_color: theme.tab_hilite_color,
            text_color: theme.label_font_color,
            background_color: theme.panel_color,
            tab_height: 32.0,
            tab_padding: 16.0,
            corner_radius: 4.0,
            on_change: None,
        }
    }

    /// Adds tabs.
    pub fn tabs(mut self, tabs: Vec<Tab>) -> Self {
        self.tabs = tabs;
        self
    }

    /// Sets the tab position.
    pub fn position(mut self, position: TabPosition) -> Self {
        self.position = position;
        self
    }

    /// Sets the active color.
    pub fn active_color(mut self, color: Color) -> Self {
        self.active_color = color;
        self
    }

    /// Sets the inactive color.
    pub fn inactive_color(mut self, color: Color) -> Self {
        self.inactive_color = color;
        self
    }

    /// Sets the change callback.
    pub fn on_change<F: Fn(usize) + Send + Sync + 'static>(mut self, callback: F) -> Self {
        self.on_change = Some(Box::new(callback));
        self
    }

    /// Returns the active tab index.
    pub fn get_active(&self) -> usize {
        *self.active_index.read().unwrap()
    }

    /// Sets the active tab index.
    pub fn set_active(&self, index: usize) {
        if index < self.tabs.len() {
            *self.active_index.write().unwrap() = index;
        }
    }

    fn tab_bar_rect(&self, ctx: &Context) -> Rect {
        match self.position {
            TabPosition::Top => Rect::new(
                ctx.bounds.left,
                ctx.bounds.top,
                ctx.bounds.right,
                ctx.bounds.top + self.tab_height,
            ),
            TabPosition::Bottom => Rect::new(
                ctx.bounds.left,
                ctx.bounds.bottom - self.tab_height,
                ctx.bounds.right,
                ctx.bounds.bottom,
            ),
            TabPosition::Left => Rect::new(
                ctx.bounds.left,
                ctx.bounds.top,
                ctx.bounds.left + 100.0,
                ctx.bounds.bottom,
            ),
            TabPosition::Right => Rect::new(
                ctx.bounds.right - 100.0,
                ctx.bounds.top,
                ctx.bounds.right,
                ctx.bounds.bottom,
            ),
        }
    }

    fn content_rect(&self, ctx: &Context) -> Rect {
        match self.position {
            TabPosition::Top => Rect::new(
                ctx.bounds.left,
                ctx.bounds.top + self.tab_height,
                ctx.bounds.right,
                ctx.bounds.bottom,
            ),
            TabPosition::Bottom => Rect::new(
                ctx.bounds.left,
                ctx.bounds.top,
                ctx.bounds.right,
                ctx.bounds.bottom - self.tab_height,
            ),
            TabPosition::Left => Rect::new(
                ctx.bounds.left + 100.0,
                ctx.bounds.top,
                ctx.bounds.right,
                ctx.bounds.bottom,
            ),
            TabPosition::Right => Rect::new(
                ctx.bounds.left,
                ctx.bounds.top,
                ctx.bounds.right - 100.0,
                ctx.bounds.bottom,
            ),
        }
    }

    fn tab_rect(&self, ctx: &Context, index: usize) -> Rect {
        let bar = self.tab_bar_rect(ctx);
        let theme = get_theme();

        match self.position {
            TabPosition::Top | TabPosition::Bottom => {
                let mut x = bar.left;
                for (i, tab) in self.tabs.iter().enumerate() {
                    let width = tab.label.len() as f32 * theme.label_font_size * 0.6 + self.tab_padding * 2.0;
                    if i == index {
                        return Rect::new(x, bar.top, x + width, bar.bottom);
                    }
                    x += width;
                }
            }
            TabPosition::Left | TabPosition::Right => {
                let mut y = bar.top;
                for i in 0..self.tabs.len() {
                    if i == index {
                        return Rect::new(bar.left, y, bar.right, y + self.tab_height);
                    }
                    y += self.tab_height;
                }
            }
        }

        Rect::zero()
    }

    fn draw_tabs(&self, ctx: &Context) {
        let mut canvas = ctx.canvas.borrow_mut();
        let theme = get_theme();
        let bar = self.tab_bar_rect(ctx);
        let active = *self.active_index.read().unwrap();
        let hovered = *self.hovered_index.read().unwrap();

        // Tab bar background
        canvas.fill_style(self.background_color);
        canvas.fill_rect(bar);

        // Draw each tab
        for (i, tab) in self.tabs.iter().enumerate() {
            let rect = self.tab_rect(ctx, i);

            let is_active = i == active;
            let is_hovered = hovered == Some(i) && !is_active;

            // Tab background
            let bg_color = if is_active {
                self.active_color
            } else if is_hovered {
                self.hover_color
            } else {
                self.inactive_color
            };

            let tab_rect = match self.position {
                TabPosition::Top => Rect::new(
                    rect.left + 1.0,
                    rect.top + 2.0,
                    rect.right - 1.0,
                    rect.bottom,
                ),
                TabPosition::Bottom => Rect::new(
                    rect.left + 1.0,
                    rect.top,
                    rect.right - 1.0,
                    rect.bottom - 2.0,
                ),
                _ => rect.inset(1.0, 1.0),
            };

            canvas.fill_style(bg_color);
            canvas.fill_round_rect(tab_rect, self.corner_radius);

            // Tab text
            let text_color = if is_active {
                self.text_color
            } else {
                self.text_color.with_alpha(0.7)
            };

            canvas.fill_style(text_color);
            canvas.font_size(theme.label_font_size);

            let x = rect.left + self.tab_padding;
            let y = rect.center().y + theme.label_font_size * 0.35;
            canvas.fill_text(&tab.label, Point::new(x, y));
        }
    }

    fn draw_content(&self, ctx: &Context) {
        let active = *self.active_index.read().unwrap();
        if let Some(tab) = self.tabs.get(active) {
            if let Some(ref content) = tab.content {
                let content_rect = self.content_rect(ctx);
                let content_ctx = ctx.with_bounds(content_rect);
                content.draw(&content_ctx);
            }
        }
    }
}

impl Default for TabBar {
    fn default() -> Self {
        Self::new()
    }
}

impl Element for TabBar {
    fn limits(&self, _ctx: &BasicContext) -> ViewLimits {
        ViewLimits {
            min: Point::new(200.0, 100.0),
            max: Point::new(super::FULL_EXTENT, super::FULL_EXTENT),
        }
    }

    fn stretch(&self) -> ViewStretch {
        ViewStretch::new(1.0, 1.0)
    }

    fn draw(&self, ctx: &Context) {
        self.draw_content(ctx);
        self.draw_tabs(ctx);
    }

    fn hit_test(&self, ctx: &Context, p: Point, leaf: bool, control: bool) -> Option<&dyn Element> {
        if !ctx.bounds.contains(p) {
            return None;
        }

        // Check tabs
        let bar = self.tab_bar_rect(ctx);
        if bar.contains(p) {
            return Some(self);
        }

        // Check content
        let active = *self.active_index.read().unwrap();
        if let Some(tab) = self.tabs.get(active) {
            if let Some(ref content) = tab.content {
                let content_rect = self.content_rect(ctx);
                let content_ctx = ctx.with_bounds(content_rect);
                if let Some(hit) = content.hit_test(&content_ctx, p, leaf, control) {
                    return Some(hit);
                }
            }
        }

        Some(self)
    }

    fn wants_control(&self) -> bool {
        true
    }

    fn handle_click(&self, ctx: &Context, btn: MouseButton) -> bool {
        if btn.button != MouseButtonKind::Left {
            return false;
        }

        if !btn.down {
            return true;
        }

        // Check if clicking on a tab
        for i in 0..self.tabs.len() {
            let rect = self.tab_rect(ctx, i);
            if rect.contains(btn.pos) {
                let old_active = *self.active_index.read().unwrap();
                if i != old_active {
                    *self.active_index.write().unwrap() = i;
                    if let Some(ref callback) = self.on_change {
                        callback(i);
                    }
                }
                return true;
            }
        }

        // Forward to content
        let active = *self.active_index.read().unwrap();
        if let Some(tab) = self.tabs.get(active) {
            if let Some(ref content) = tab.content {
                let content_rect = self.content_rect(ctx);
                let content_ctx = ctx.with_bounds(content_rect);
                if content.handle_click(&content_ctx, btn) {
                    return true;
                }
            }
        }

        true
    }

    fn cursor(&mut self, ctx: &Context, p: Point, status: CursorTracking) -> bool {
        match status {
            CursorTracking::Leaving => {
                *self.hovered_index.write().unwrap() = None;
            }
            _ => {
                let mut hovered = self.hovered_index.write().unwrap();
                *hovered = None;

                for i in 0..self.tabs.len() {
                    let rect = self.tab_rect(ctx, i);
                    if rect.contains(p) {
                        *hovered = Some(i);
                        break;
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

/// Creates a tab bar.
pub fn tab_bar() -> TabBar {
    TabBar::new()
}

/// Creates a tab.
pub fn tab(label: impl Into<String>) -> Tab {
    Tab::new(label)
}
