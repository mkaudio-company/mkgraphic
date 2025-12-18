//! Scrollable/scroll view element.

use std::any::Any;
use std::sync::RwLock;
use super::{Element, ElementPtr, ViewLimits, ViewStretch, share};
use super::context::{BasicContext, Context};
use crate::support::point::Point;
use crate::support::rect::Rect;
use crate::support::color::Color;
use crate::support::theme::get_theme;
use crate::view::{MouseButton, MouseButtonKind};

/// Scrollbar visibility options.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ScrollbarVisibility {
    #[default]
    Auto,
    Always,
    Never,
}

/// A scrollable container element.
pub struct ScrollView {
    content: Option<ElementPtr>,
    scroll_offset: RwLock<Point>,
    content_size: RwLock<Point>,
    h_scrollbar: ScrollbarVisibility,
    v_scrollbar: ScrollbarVisibility,
    scrollbar_color: Color,
    scrollbar_hover_color: Color,
    scrollbar_width: f32,
    width: f32,
    height: f32,
    dragging_v: RwLock<bool>,
    dragging_h: RwLock<bool>,
    drag_start: RwLock<Point>,
    drag_start_scroll: RwLock<Point>,
}

impl ScrollView {
    /// Creates a new scroll view.
    pub fn new() -> Self {
        let theme = get_theme();
        Self {
            content: None,
            scroll_offset: RwLock::new(Point::zero()),
            content_size: RwLock::new(Point::new(400.0, 400.0)),
            h_scrollbar: ScrollbarVisibility::Auto,
            v_scrollbar: ScrollbarVisibility::Auto,
            scrollbar_color: theme.scrollbar_color,
            scrollbar_hover_color: theme.scrollbar_color.level(1.3),
            scrollbar_width: theme.scrollbar_width,
            width: 200.0,
            height: 200.0,
            dragging_v: RwLock::new(false),
            dragging_h: RwLock::new(false),
            drag_start: RwLock::new(Point::zero()),
            drag_start_scroll: RwLock::new(Point::zero()),
        }
    }

    /// Sets the content.
    pub fn content<E: Element + 'static>(mut self, content: E) -> Self {
        self.content = Some(share(content));
        self
    }

    /// Sets the content size.
    pub fn content_size(self, width: f32, height: f32) -> Self {
        *self.content_size.write().unwrap() = Point::new(width, height);
        self
    }

    /// Sets the view size.
    pub fn size(mut self, width: f32, height: f32) -> Self {
        self.width = width;
        self.height = height;
        self
    }

    /// Sets horizontal scrollbar visibility.
    pub fn h_scrollbar(mut self, visibility: ScrollbarVisibility) -> Self {
        self.h_scrollbar = visibility;
        self
    }

    /// Sets vertical scrollbar visibility.
    pub fn v_scrollbar(mut self, visibility: ScrollbarVisibility) -> Self {
        self.v_scrollbar = visibility;
        self
    }

    /// Sets the scrollbar color.
    pub fn scrollbar_color(mut self, color: Color) -> Self {
        self.scrollbar_color = color;
        self
    }

    /// Returns the current scroll offset.
    pub fn get_scroll(&self) -> Point {
        *self.scroll_offset.read().unwrap()
    }

    /// Sets the scroll offset.
    pub fn set_scroll(&self, offset: Point) {
        let content_size = *self.content_size.read().unwrap();
        let max_x = (content_size.x - self.width).max(0.0);
        let max_y = (content_size.y - self.height).max(0.0);

        *self.scroll_offset.write().unwrap() = Point::new(
            offset.x.clamp(0.0, max_x),
            offset.y.clamp(0.0, max_y),
        );
    }

    /// Scrolls to make a point visible.
    pub fn scroll_to_visible(&self, point: Point) {
        let scroll = *self.scroll_offset.read().unwrap();

        let mut new_scroll = scroll;

        if point.x < scroll.x {
            new_scroll.x = point.x;
        } else if point.x > scroll.x + self.width {
            new_scroll.x = point.x - self.width;
        }

        if point.y < scroll.y {
            new_scroll.y = point.y;
        } else if point.y > scroll.y + self.height {
            new_scroll.y = point.y - self.height;
        }

        self.set_scroll(new_scroll);
    }

    fn needs_v_scrollbar(&self) -> bool {
        match self.v_scrollbar {
            ScrollbarVisibility::Always => true,
            ScrollbarVisibility::Never => false,
            ScrollbarVisibility::Auto => {
                let content_size = self.content_size.read().unwrap();
                content_size.y > self.height
            }
        }
    }

    fn needs_h_scrollbar(&self) -> bool {
        match self.h_scrollbar {
            ScrollbarVisibility::Always => true,
            ScrollbarVisibility::Never => false,
            ScrollbarVisibility::Auto => {
                let content_size = self.content_size.read().unwrap();
                content_size.x > self.width
            }
        }
    }

    fn viewport_rect(&self, ctx: &Context) -> Rect {
        let has_v = self.needs_v_scrollbar();
        let has_h = self.needs_h_scrollbar();

        Rect::new(
            ctx.bounds.left,
            ctx.bounds.top,
            ctx.bounds.right - if has_v { self.scrollbar_width } else { 0.0 },
            ctx.bounds.bottom - if has_h { self.scrollbar_width } else { 0.0 },
        )
    }

    fn v_scrollbar_rect(&self, ctx: &Context) -> Rect {
        if !self.needs_v_scrollbar() {
            return Rect::zero();
        }

        let has_h = self.needs_h_scrollbar();

        Rect::new(
            ctx.bounds.right - self.scrollbar_width,
            ctx.bounds.top,
            ctx.bounds.right,
            ctx.bounds.bottom - if has_h { self.scrollbar_width } else { 0.0 },
        )
    }

    fn h_scrollbar_rect(&self, ctx: &Context) -> Rect {
        if !self.needs_h_scrollbar() {
            return Rect::zero();
        }

        let has_v = self.needs_v_scrollbar();

        Rect::new(
            ctx.bounds.left,
            ctx.bounds.bottom - self.scrollbar_width,
            ctx.bounds.right - if has_v { self.scrollbar_width } else { 0.0 },
            ctx.bounds.bottom,
        )
    }

    fn v_thumb_rect(&self, ctx: &Context) -> Rect {
        let track = self.v_scrollbar_rect(ctx);
        if track.is_empty() {
            return Rect::zero();
        }

        let content_size = self.content_size.read().unwrap();
        let scroll = self.scroll_offset.read().unwrap();
        let viewport = self.viewport_rect(ctx);

        let visible_ratio = (viewport.height() / content_size.y).min(1.0);
        let thumb_height = (track.height() * visible_ratio).max(20.0);

        let scroll_ratio = if content_size.y > viewport.height() {
            scroll.y / (content_size.y - viewport.height())
        } else {
            0.0
        };

        let thumb_y = track.top + scroll_ratio * (track.height() - thumb_height);

        Rect::new(
            track.left + 2.0,
            thumb_y,
            track.right - 2.0,
            thumb_y + thumb_height,
        )
    }

    fn h_thumb_rect(&self, ctx: &Context) -> Rect {
        let track = self.h_scrollbar_rect(ctx);
        if track.is_empty() {
            return Rect::zero();
        }

        let content_size = self.content_size.read().unwrap();
        let scroll = self.scroll_offset.read().unwrap();
        let viewport = self.viewport_rect(ctx);

        let visible_ratio = (viewport.width() / content_size.x).min(1.0);
        let thumb_width = (track.width() * visible_ratio).max(20.0);

        let scroll_ratio = if content_size.x > viewport.width() {
            scroll.x / (content_size.x - viewport.width())
        } else {
            0.0
        };

        let thumb_x = track.left + scroll_ratio * (track.width() - thumb_width);

        Rect::new(
            thumb_x,
            track.top + 2.0,
            thumb_x + thumb_width,
            track.bottom - 2.0,
        )
    }

    fn draw_scrollbars(&self, ctx: &Context) {
        let mut canvas = ctx.canvas.borrow_mut();

        // Vertical scrollbar
        if self.needs_v_scrollbar() {
            let track = self.v_scrollbar_rect(ctx);
            let thumb = self.v_thumb_rect(ctx);

            // Track background
            canvas.fill_style(self.scrollbar_color.with_alpha(0.2));
            canvas.fill_rect(track);

            // Thumb
            let color = if *self.dragging_v.read().unwrap() {
                self.scrollbar_hover_color
            } else {
                self.scrollbar_color
            };
            canvas.fill_style(color);
            canvas.fill_round_rect(thumb, 3.0);
        }

        // Horizontal scrollbar
        if self.needs_h_scrollbar() {
            let track = self.h_scrollbar_rect(ctx);
            let thumb = self.h_thumb_rect(ctx);

            // Track background
            canvas.fill_style(self.scrollbar_color.with_alpha(0.2));
            canvas.fill_rect(track);

            // Thumb
            let color = if *self.dragging_h.read().unwrap() {
                self.scrollbar_hover_color
            } else {
                self.scrollbar_color
            };
            canvas.fill_style(color);
            canvas.fill_round_rect(thumb, 3.0);
        }

        // Corner (if both scrollbars present)
        if self.needs_v_scrollbar() && self.needs_h_scrollbar() {
            let corner = Rect::new(
                ctx.bounds.right - self.scrollbar_width,
                ctx.bounds.bottom - self.scrollbar_width,
                ctx.bounds.right,
                ctx.bounds.bottom,
            );
            canvas.fill_style(self.scrollbar_color.with_alpha(0.3));
            canvas.fill_rect(corner);
        }
    }
}

impl Default for ScrollView {
    fn default() -> Self {
        Self::new()
    }
}

impl Element for ScrollView {
    fn limits(&self, _ctx: &BasicContext) -> ViewLimits {
        ViewLimits::fixed(self.width, self.height)
    }

    fn stretch(&self) -> ViewStretch {
        ViewStretch::new(1.0, 1.0)
    }

    fn draw(&self, ctx: &Context) {
        let viewport = self.viewport_rect(ctx);
        let scroll = *self.scroll_offset.read().unwrap();
        let content_size = *self.content_size.read().unwrap();

        // Draw content
        if let Some(ref content) = self.content {
            // Content bounds (scrolled)
            let content_bounds = Rect::new(
                viewport.left - scroll.x,
                viewport.top - scroll.y,
                viewport.left - scroll.x + content_size.x,
                viewport.top - scroll.y + content_size.y,
            );

            // Clip to viewport (simplified - would need proper clipping)
            let content_ctx = ctx.with_bounds(content_bounds);
            content.draw(&content_ctx);
        }

        self.draw_scrollbars(ctx);
    }

    fn hit_test(&self, ctx: &Context, p: Point, leaf: bool, control: bool) -> Option<&dyn Element> {
        if !ctx.bounds.contains(p) {
            return None;
        }

        // Check scrollbars first
        if self.v_thumb_rect(ctx).contains(p) || self.h_thumb_rect(ctx).contains(p) {
            return Some(self);
        }

        // Check content
        let viewport = self.viewport_rect(ctx);
        if viewport.contains(p) {
            if let Some(ref content) = self.content {
                let scroll = *self.scroll_offset.read().unwrap();
                let content_size = *self.content_size.read().unwrap();
                let content_bounds = Rect::new(
                    viewport.left - scroll.x,
                    viewport.top - scroll.y,
                    viewport.left - scroll.x + content_size.x,
                    viewport.top - scroll.y + content_size.y,
                );
                let content_ctx = ctx.with_bounds(content_bounds);
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

        if btn.down {
            // Check vertical scrollbar
            if self.v_thumb_rect(ctx).contains(btn.pos) {
                *self.dragging_v.write().unwrap() = true;
                *self.drag_start.write().unwrap() = btn.pos;
                *self.drag_start_scroll.write().unwrap() = *self.scroll_offset.read().unwrap();
                return true;
            }

            // Check horizontal scrollbar
            if self.h_thumb_rect(ctx).contains(btn.pos) {
                *self.dragging_h.write().unwrap() = true;
                *self.drag_start.write().unwrap() = btn.pos;
                *self.drag_start_scroll.write().unwrap() = *self.scroll_offset.read().unwrap();
                return true;
            }

            // Forward to content
            let viewport = self.viewport_rect(ctx);
            if viewport.contains(btn.pos) {
                if let Some(ref content) = self.content {
                    let scroll = *self.scroll_offset.read().unwrap();
                    let content_size = *self.content_size.read().unwrap();
                    let content_bounds = Rect::new(
                        viewport.left - scroll.x,
                        viewport.top - scroll.y,
                        viewport.left - scroll.x + content_size.x,
                        viewport.top - scroll.y + content_size.y,
                    );
                    let content_ctx = ctx.with_bounds(content_bounds);
                    if content.handle_click(&content_ctx, btn) {
                        return true;
                    }
                }
            }
        } else {
            *self.dragging_v.write().unwrap() = false;
            *self.dragging_h.write().unwrap() = false;

            // Forward to content
            let viewport = self.viewport_rect(ctx);
            if viewport.contains(btn.pos) {
                if let Some(ref content) = self.content {
                    let scroll = *self.scroll_offset.read().unwrap();
                    let content_size = *self.content_size.read().unwrap();
                    let content_bounds = Rect::new(
                        viewport.left - scroll.x,
                        viewport.top - scroll.y,
                        viewport.left - scroll.x + content_size.x,
                        viewport.top - scroll.y + content_size.y,
                    );
                    let content_ctx = ctx.with_bounds(content_bounds);
                    if content.handle_click(&content_ctx, btn) {
                        return true;
                    }
                }
            }
        }

        true
    }

    fn drag(&mut self, ctx: &Context, btn: MouseButton) {
        let dragging_v = *self.dragging_v.read().unwrap();
        let dragging_h = *self.dragging_h.read().unwrap();

        if dragging_v {
            let drag_start = *self.drag_start.read().unwrap();
            let start_scroll = *self.drag_start_scroll.read().unwrap();
            let track = self.v_scrollbar_rect(ctx);
            let thumb = self.v_thumb_rect(ctx);
            let content_size = *self.content_size.read().unwrap();
            let viewport = self.viewport_rect(ctx);

            let delta_y = btn.pos.y - drag_start.y;
            let track_range = track.height() - thumb.height();
            let scroll_range = content_size.y - viewport.height();

            if track_range > 0.0 {
                let new_scroll_y = start_scroll.y + delta_y * scroll_range / track_range;
                self.set_scroll(Point::new(start_scroll.x, new_scroll_y));
            }
        }

        if dragging_h {
            let drag_start = *self.drag_start.read().unwrap();
            let start_scroll = *self.drag_start_scroll.read().unwrap();
            let track = self.h_scrollbar_rect(ctx);
            let thumb = self.h_thumb_rect(ctx);
            let content_size = *self.content_size.read().unwrap();
            let viewport = self.viewport_rect(ctx);

            let delta_x = btn.pos.x - drag_start.x;
            let track_range = track.width() - thumb.width();
            let scroll_range = content_size.x - viewport.width();

            if track_range > 0.0 {
                let new_scroll_x = start_scroll.x + delta_x * scroll_range / track_range;
                self.set_scroll(Point::new(new_scroll_x, start_scroll.y));
            }
        }
    }

    fn scroll(&mut self, ctx: &Context, dir: Point, _p: Point) -> bool {
        let current = *self.scroll_offset.read().unwrap();
        let new_scroll = Point::new(
            current.x - dir.x * 20.0,
            current.y - dir.y * 20.0,
        );
        self.set_scroll(new_scroll);
        true
    }

    fn as_any(&self) -> &dyn Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }
}

/// Creates a scroll view.
pub fn scroll_view() -> ScrollView {
    ScrollView::new()
}

/// Creates a vertical-only scroll view.
pub fn vscroll_view() -> ScrollView {
    ScrollView::new().h_scrollbar(ScrollbarVisibility::Never)
}

/// Creates a horizontal-only scroll view.
pub fn hscroll_view() -> ScrollView {
    ScrollView::new().v_scrollbar(ScrollbarVisibility::Never)
}
