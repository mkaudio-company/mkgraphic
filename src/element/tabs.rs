//! Tab element for switching between views.

use super::context::{BasicContext, Context};
use super::{share, Element, ElementPtr, ViewLimits, ViewStretch};
use crate::support::color::Color;
use crate::support::point::Point;
use crate::support::rect::Rect;
use crate::support::theme::get_theme;
use crate::view::{CursorTracking, MouseButton, MouseButtonKind};
use std::any::Any;
use std::sync::RwLock;

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
/// Callback type for a tab's close ("x") button being clicked. Receives the
/// closed tab's index *before* removal.
pub type TabCloseCallback = Box<dyn Fn(usize) + Send + Sync>;

/// A single tab.
pub struct Tab {
    label: String,
    content: Option<ElementPtr>,
    closable: bool,
}

impl Tab {
    /// Creates a new tab.
    pub fn new(label: impl Into<String>) -> Self {
        Self {
            label: label.into(),
            content: None,
            closable: false,
        }
    }

    /// Sets the tab content.
    pub fn content<E: Element + 'static>(mut self, content: E) -> Self {
        self.content = Some(share(content));
        self
    }

    /// Shows a close ("x") button on this tab; clicking it fires
    /// `TabBar::on_close` instead of activating the tab. Needed for an
    /// editor-style tab bar (one tab per open file) where tabs are closed
    /// individually, unlike a fixed set of view tabs.
    pub fn closable(mut self, closable: bool) -> Self {
        self.closable = closable;
        self
    }
}

/// A tabbed container element.
pub struct TabBar {
    /// Behind a lock (not a plain `Vec`) so tabs can be added/removed at
    /// runtime through `&self` -- widgets are shared as `Arc<dyn Element>`
    /// once mounted, so an owning `&mut self` is never available again
    /// after construction. Needed for an IDE tab bar, where files get
    /// opened/closed while the app is running.
    tabs: RwLock<Vec<Tab>>,
    active_index: RwLock<usize>,
    hovered_index: RwLock<Option<usize>>,
    hovered_close: RwLock<Option<usize>>,
    position: TabPosition,
    active_color: Color,
    inactive_color: Color,
    hover_color: Color,
    text_color: Color,
    background_color: Color,
    tab_height: f32,
    tab_padding: f32,
    corner_radius: f32,
    close_button_size: f32,
    on_change: Option<TabChangeCallback>,
    on_close: Option<TabCloseCallback>,
}

impl TabBar {
    /// Creates a new tab bar.
    pub fn new() -> Self {
        let theme = get_theme();
        Self {
            tabs: RwLock::new(Vec::new()),
            active_index: RwLock::new(0),
            hovered_index: RwLock::new(None),
            hovered_close: RwLock::new(None),
            position: TabPosition::Top,
            active_color: theme.active_tab_color,
            inactive_color: theme.inactive_tab_color,
            hover_color: theme.tab_hilite_color,
            text_color: theme.label_font_color,
            background_color: theme.panel_color,
            tab_height: 32.0,
            tab_padding: 16.0,
            corner_radius: 4.0,
            close_button_size: 14.0,
            on_change: None,
            on_close: None,
        }
    }

    /// Adds tabs.
    pub fn tabs(self, tabs: Vec<Tab>) -> Self {
        *self.tabs.write().unwrap() = tabs;
        self
    }

    /// Sets the close-button callback (only relevant for tabs created with
    /// `Tab::closable(true)`).
    pub fn on_close<F: Fn(usize) + Send + Sync + 'static>(mut self, callback: F) -> Self {
        self.on_close = Some(Box::new(callback));
        self
    }

    /// Appends a new tab at runtime and returns its index. Does not change
    /// which tab is active.
    pub fn add_tab(&self, tab: Tab) -> usize {
        let mut tabs = self.tabs.write().unwrap();
        tabs.push(tab);
        tabs.len() - 1
    }

    /// Removes the tab at `index` at runtime. Keeps the active index
    /// pointing at the same logical tab it did before the removal: a tab
    /// closed *before* the active one shifts the active index down by one;
    /// closing the active tab itself activates whichever tab slides into
    /// its place (or the new last tab, if it was the last one open).
    pub fn remove_tab(&self, index: usize) {
        let mut tabs = self.tabs.write().unwrap();
        if index >= tabs.len() {
            return;
        }
        tabs.remove(index);
        let len = tabs.len();
        drop(tabs);

        let mut active = self.active_index.write().unwrap();
        if len == 0 {
            *active = 0;
        } else if index < *active {
            *active -= 1;
        } else if index == *active {
            *active = (*active).min(len - 1);
        }
    }

    /// Returns the current number of tabs.
    pub fn tab_count(&self) -> usize {
        self.tabs.read().unwrap().len()
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
        if index < self.tabs.read().unwrap().len() {
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

    /// Computes tab `index`'s rect given an already-locked `tabs` slice.
    /// Takes the slice rather than locking `self.tabs` itself so callers
    /// that already hold the read guard (draw/hit-test/click, which need it
    /// for more than just this calculation) don't have to take a second,
    /// nested lock on the same `RwLock` -- std's `RwLock` doesn't guarantee
    /// that's deadlock-free if a writer happens to be queued in between.
    fn tab_rect(&self, ctx: &Context, tabs: &[Tab], index: usize) -> Rect {
        let bar = self.tab_bar_rect(ctx);
        let theme = get_theme();

        match self.position {
            TabPosition::Top | TabPosition::Bottom => {
                let mut x = bar.left;
                for (i, tab) in tabs.iter().enumerate() {
                    let width = tab.label.len() as f32 * theme.label_font_size * 0.6
                        + self.tab_padding * 2.0
                        + if tab.closable { self.close_button_size + self.tab_padding * 0.5 } else { 0.0 };
                    if i == index {
                        return Rect::new(x, bar.top, x + width, bar.bottom);
                    }
                    x += width;
                }
            }
            TabPosition::Left | TabPosition::Right => {
                let mut y = bar.top;
                for i in 0..tabs.len() {
                    if i == index {
                        return Rect::new(bar.left, y, bar.right, y + self.tab_height);
                    }
                    y += self.tab_height;
                }
            }
        }

        Rect::zero()
    }

    /// The close ("x") button's hit/draw rect within a tab's own rect, or
    /// `Rect::zero()` if that tab isn't closable.
    fn close_rect(&self, tabs: &[Tab], index: usize, tab_rect: Rect) -> Rect {
        if !tabs.get(index).is_some_and(|t| t.closable) {
            return Rect::zero();
        }
        let size = self.close_button_size;
        let y = tab_rect.center().y - size / 2.0;
        Rect::new(
            tab_rect.right - self.tab_padding * 0.5 - size,
            y,
            tab_rect.right - self.tab_padding * 0.5,
            y + size,
        )
    }

    fn draw_tabs(&self, ctx: &Context, tabs: &[Tab]) {
        let mut canvas = ctx.canvas.borrow_mut();
        let theme = get_theme();
        let bar = self.tab_bar_rect(ctx);
        let active = *self.active_index.read().unwrap();
        let hovered = *self.hovered_index.read().unwrap();
        let hovered_close = *self.hovered_close.read().unwrap();

        // Tab bar background
        canvas.fill_style(self.background_color);
        canvas.fill_rect(bar);

        // Draw each tab
        for (i, tab) in tabs.iter().enumerate() {
            let rect = self.tab_rect(ctx, tabs, i);

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

            if tab.closable {
                let close = self.close_rect(tabs, i, rect);
                let close_color = if hovered_close == Some(i) {
                    self.text_color
                } else {
                    self.text_color.with_alpha(0.5)
                };
                canvas.fill_style(close_color);
                canvas.font_size(theme.label_font_size * 0.9);
                canvas.fill_text("\u{00d7}", Point::new(close.left, close.center().y + theme.label_font_size * 0.32));
            }
        }
    }

    fn draw_content(&self, ctx: &Context, tabs: &[Tab]) {
        let active = *self.active_index.read().unwrap();
        if let Some(tab) = tabs.get(active) {
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
        let tabs = self.tabs.read().unwrap();
        self.draw_content(ctx, &tabs);
        self.draw_tabs(ctx, &tabs);
    }

    fn hit_test(&self, ctx: &Context, p: Point, _leaf: bool, _control: bool) -> Option<&dyn Element> {
        // Unlike the other containers here, `tabs` sits behind a `RwLock`
        // (needed so `add_tab`/`remove_tab` can mutate it through `&self` at
        // runtime -- see that field's doc comment) rather than being a
        // plain owned `Vec`, so there's no way to hand back a `&dyn Element`
        // borrowed from a tab's content with a lifetime tied to `&self`: the
        // read guard needed to reach it doesn't live that long. So, like
        // `List` (which has the same kind of `RwLock<Vec<_>>` item storage),
        // this reports itself as the hit target rather than forwarding into
        // nested content; `draw`/`handle_click` (which don't need to return
        // a reference) still fully delegate to the active tab's content.
        if ctx.bounds.contains(p) {
            Some(self)
        } else {
            None
        }
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

        // Check if clicking on a tab (or its close button)
        {
            let tabs = self.tabs.read().unwrap();
            for i in 0..tabs.len() {
                let rect = self.tab_rect(ctx, &tabs, i);
                if !rect.contains(btn.pos) {
                    continue;
                }

                if self.close_rect(&tabs, i, rect).contains(btn.pos) {
                    drop(tabs);
                    if let Some(ref callback) = self.on_close {
                        callback(i);
                    }
                    self.remove_tab(i);
                    return true;
                }

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
        let tabs = self.tabs.read().unwrap();
        if let Some(tab) = tabs.get(active) {
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
                *self.hovered_close.write().unwrap() = None;
            }
            _ => {
                let mut hovered = self.hovered_index.write().unwrap();
                let mut hovered_close = self.hovered_close.write().unwrap();
                *hovered = None;
                *hovered_close = None;

                let tabs = self.tabs.read().unwrap();
                for i in 0..tabs.len() {
                    let rect = self.tab_rect(ctx, &tabs, i);
                    if rect.contains(p) {
                        *hovered = Some(i);
                        if self.close_rect(&tabs, i, rect).contains(p) {
                            *hovered_close = Some(i);
                        }
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
