//! Tooltip element.

use std::any::Any;
use std::sync::RwLock;
use super::{Element, ElementPtr, ViewLimits, ViewStretch, share};
use super::context::{BasicContext, Context};
use crate::support::point::Point;
use crate::support::rect::Rect;
use crate::support::color::Color;
use crate::support::theme::get_theme;
use crate::view::CursorTracking;

/// A tooltip wrapper element.
pub struct Tooltip {
    content: Option<ElementPtr>,
    tooltip_text: String,
    visible: RwLock<bool>,
    position: RwLock<Point>,
    background_color: Color,
    text_color: Color,
    font_size: f32,
    padding: f32,
    corner_radius: f32,
    delay_ms: u32,
}

impl Tooltip {
    /// Creates a new tooltip wrapper.
    pub fn new(text: impl Into<String>) -> Self {
        let theme = get_theme();
        Self {
            content: None,
            tooltip_text: text.into(),
            visible: RwLock::new(false),
            position: RwLock::new(Point::zero()),
            background_color: theme.tooltip_color,
            text_color: theme.tooltip_text_color,
            font_size: theme.tooltip_font_size,
            padding: 6.0,
            corner_radius: 4.0,
            delay_ms: 500,
        }
    }

    /// Sets the wrapped content.
    pub fn content<E: Element + 'static>(mut self, content: E) -> Self {
        self.content = Some(share(content));
        self
    }

    /// Sets the tooltip text.
    pub fn text(mut self, text: impl Into<String>) -> Self {
        self.tooltip_text = text.into();
        self
    }

    /// Sets the background color.
    pub fn background_color(mut self, color: Color) -> Self {
        self.background_color = color;
        self
    }

    /// Sets the text color.
    pub fn text_color(mut self, color: Color) -> Self {
        self.text_color = color;
        self
    }

    /// Sets the delay in milliseconds.
    pub fn delay(mut self, ms: u32) -> Self {
        self.delay_ms = ms;
        self
    }

    /// Shows the tooltip.
    pub fn show(&self, position: Point) {
        *self.position.write().unwrap() = position;
        *self.visible.write().unwrap() = true;
    }

    /// Hides the tooltip.
    pub fn hide(&self) {
        *self.visible.write().unwrap() = false;
    }

    /// Returns whether the tooltip is visible.
    pub fn is_visible(&self) -> bool {
        *self.visible.read().unwrap()
    }

    fn tooltip_bounds(&self) -> Rect {
        let pos = *self.position.read().unwrap();
        let width = self.tooltip_text.len() as f32 * self.font_size * 0.55 + self.padding * 2.0;
        let height = self.font_size + self.padding * 2.0;

        // Position tooltip below and slightly to the right of cursor
        Rect::new(
            pos.x + 10.0,
            pos.y + 20.0,
            pos.x + 10.0 + width,
            pos.y + 20.0 + height,
        )
    }

    fn draw_tooltip(&self, _ctx: &Context) {
        if !self.is_visible() || self.tooltip_text.is_empty() {
            return;
        }

        // Note: In a real implementation, tooltips would typically be drawn
        // in a separate overlay layer after all other content, to ensure
        // they appear on top. This simplified version draws inline.
    }
}

impl Element for Tooltip {
    fn limits(&self, ctx: &BasicContext) -> ViewLimits {
        if let Some(ref content) = self.content {
            content.limits(ctx)
        } else {
            ViewLimits::full()
        }
    }

    fn stretch(&self) -> ViewStretch {
        if let Some(ref content) = self.content {
            content.stretch()
        } else {
            ViewStretch::default()
        }
    }

    fn draw(&self, ctx: &Context) {
        // Draw content
        if let Some(ref content) = self.content {
            content.draw(ctx);
        }

        // Draw tooltip if visible
        if self.is_visible() && !self.tooltip_text.is_empty() {
            let bounds = self.tooltip_bounds();
            let mut canvas = ctx.canvas.borrow_mut();

            // Shadow
            let shadow_rect = bounds.translate(2.0, 2.0);
            canvas.fill_style(Color::new(0.0, 0.0, 0.0, 0.3));
            canvas.fill_round_rect(shadow_rect, self.corner_radius);

            // Background
            canvas.fill_style(self.background_color);
            canvas.fill_round_rect(bounds, self.corner_radius);

            // Text
            canvas.fill_style(self.text_color);
            canvas.font_size(self.font_size);

            let x = bounds.left + self.padding;
            let y = bounds.center().y + self.font_size * 0.35;
            canvas.fill_text(&self.tooltip_text, Point::new(x, y));
        }
    }

    fn hit_test(&self, ctx: &Context, p: Point, leaf: bool, control: bool) -> Option<&dyn Element> {
        if let Some(ref content) = self.content {
            content.hit_test(ctx, p, leaf, control)
        } else {
            if ctx.bounds.contains(p) {
                Some(self)
            } else {
                None
            }
        }
    }

    fn wants_control(&self) -> bool {
        if let Some(ref content) = self.content {
            content.wants_control()
        } else {
            false
        }
    }

    fn handle_click(&self, ctx: &Context, btn: crate::view::MouseButton) -> bool {
        // Hide tooltip on click
        self.hide();

        if let Some(ref content) = self.content {
            content.handle_click(ctx, btn)
        } else {
            false
        }
    }

    fn cursor(&mut self, ctx: &Context, p: Point, status: CursorTracking) -> bool {
        match status {
            CursorTracking::Entering | CursorTracking::Hovering => {
                if ctx.bounds.contains(p) {
                    // Show tooltip after delay (simplified - immediate show)
                    self.show(p);
                }
            }
            CursorTracking::Leaving => {
                self.hide();
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

/// Creates a standalone tooltip element (for overlay rendering).
pub struct TooltipOverlay {
    text: RwLock<String>,
    visible: RwLock<bool>,
    position: RwLock<Point>,
    background_color: Color,
    text_color: Color,
    font_size: f32,
    padding: f32,
    corner_radius: f32,
}

impl TooltipOverlay {
    /// Creates a new tooltip overlay.
    pub fn new() -> Self {
        let theme = get_theme();
        Self {
            text: RwLock::new(String::new()),
            visible: RwLock::new(false),
            position: RwLock::new(Point::zero()),
            background_color: theme.tooltip_color,
            text_color: theme.tooltip_text_color,
            font_size: theme.tooltip_font_size,
            padding: 6.0,
            corner_radius: 4.0,
        }
    }

    /// Shows the tooltip with text at position.
    pub fn show(&self, text: impl Into<String>, position: Point) {
        *self.text.write().unwrap() = text.into();
        *self.position.write().unwrap() = position;
        *self.visible.write().unwrap() = true;
    }

    /// Hides the tooltip.
    pub fn hide(&self) {
        *self.visible.write().unwrap() = false;
    }

    /// Returns whether visible.
    pub fn is_visible(&self) -> bool {
        *self.visible.read().unwrap()
    }

    fn tooltip_bounds(&self) -> Rect {
        let pos = *self.position.read().unwrap();
        let text = self.text.read().unwrap();
        let width = text.len() as f32 * self.font_size * 0.55 + self.padding * 2.0;
        let height = self.font_size + self.padding * 2.0;

        Rect::new(
            pos.x + 10.0,
            pos.y + 20.0,
            pos.x + 10.0 + width,
            pos.y + 20.0 + height,
        )
    }
}

impl Default for TooltipOverlay {
    fn default() -> Self {
        Self::new()
    }
}

impl Element for TooltipOverlay {
    fn limits(&self, _ctx: &BasicContext) -> ViewLimits {
        ViewLimits::fixed(0.0, 0.0)
    }

    fn stretch(&self) -> ViewStretch {
        ViewStretch::new(0.0, 0.0)
    }

    fn draw(&self, ctx: &Context) {
        if !self.is_visible() {
            return;
        }

        let text = self.text.read().unwrap();
        if text.is_empty() {
            return;
        }

        let bounds = self.tooltip_bounds();
        let mut canvas = ctx.canvas.borrow_mut();

        // Shadow
        let shadow_rect = bounds.translate(2.0, 2.0);
        canvas.fill_style(Color::new(0.0, 0.0, 0.0, 0.3));
        canvas.fill_round_rect(shadow_rect, self.corner_radius);

        // Background
        canvas.fill_style(self.background_color);
        canvas.fill_round_rect(bounds, self.corner_radius);

        // Text
        canvas.fill_style(self.text_color);
        canvas.font_size(self.font_size);

        let x = bounds.left + self.padding;
        let y = bounds.center().y + self.font_size * 0.35;
        canvas.fill_text(&text, Point::new(x, y));
    }

    fn as_any(&self) -> &dyn Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }
}

/// Creates a tooltip wrapper.
pub fn tooltip(text: impl Into<String>) -> Tooltip {
    Tooltip::new(text)
}

/// Creates a tooltip overlay.
pub fn tooltip_overlay() -> TooltipOverlay {
    TooltipOverlay::new()
}
