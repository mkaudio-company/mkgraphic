//! Floating/draggable element.

use std::any::Any;
use std::sync::RwLock;
use super::{Element, ElementPtr, ViewLimits, ViewStretch, share};
use super::context::{BasicContext, Context};
use crate::support::point::Point;
use crate::support::rect::Rect;
use crate::support::color::Color;
use crate::support::theme::get_theme;
use crate::view::{MouseButton, MouseButtonKind};

/// A floating element that can be positioned freely and dragged.
pub struct Floating {
    content: Option<ElementPtr>,
    position: RwLock<Point>,
    size: RwLock<Point>,
    dragging: RwLock<bool>,
    drag_offset: RwLock<Point>,
    background_color: Color,
    border_color: Color,
    corner_radius: f32,
    shadow: bool,
    draggable: bool,
    visible: RwLock<bool>,
}

impl Floating {
    /// Creates a new floating element.
    pub fn new() -> Self {
        let theme = get_theme();
        Self {
            content: None,
            position: RwLock::new(Point::new(100.0, 100.0)),
            size: RwLock::new(Point::new(200.0, 150.0)),
            dragging: RwLock::new(false),
            drag_offset: RwLock::new(Point::zero()),
            background_color: theme.element_background_color,
            border_color: theme.frame_color,
            corner_radius: 8.0,
            shadow: true,
            draggable: true,
            visible: RwLock::new(true),
        }
    }

    /// Sets the content.
    pub fn content<E: Element + 'static>(mut self, content: E) -> Self {
        self.content = Some(share(content));
        self
    }

    /// Sets the initial position.
    pub fn position(self, x: f32, y: f32) -> Self {
        *self.position.write().unwrap() = Point::new(x, y);
        self
    }

    /// Sets the size.
    pub fn size(self, width: f32, height: f32) -> Self {
        *self.size.write().unwrap() = Point::new(width, height);
        self
    }

    /// Sets the background color.
    pub fn background_color(mut self, color: Color) -> Self {
        self.background_color = color;
        self
    }

    /// Sets whether the element is draggable.
    pub fn draggable(mut self, draggable: bool) -> Self {
        self.draggable = draggable;
        self
    }

    /// Sets whether to show a shadow.
    pub fn shadow(mut self, shadow: bool) -> Self {
        self.shadow = shadow;
        self
    }

    /// Shows the floating element.
    pub fn show(&self) {
        *self.visible.write().unwrap() = true;
    }

    /// Hides the floating element.
    pub fn hide(&self) {
        *self.visible.write().unwrap() = false;
    }

    /// Returns whether visible.
    pub fn is_visible(&self) -> bool {
        *self.visible.read().unwrap()
    }

    /// Gets the current position.
    pub fn get_position(&self) -> Point {
        *self.position.read().unwrap()
    }

    /// Sets the position.
    pub fn set_position(&self, pos: Point) {
        *self.position.write().unwrap() = pos;
    }

    fn floating_bounds(&self) -> Rect {
        let pos = *self.position.read().unwrap();
        let size = *self.size.read().unwrap();
        Rect::new(pos.x, pos.y, pos.x + size.x, pos.y + size.y)
    }
}

impl Default for Floating {
    fn default() -> Self {
        Self::new()
    }
}

impl Element for Floating {
    fn limits(&self, _ctx: &BasicContext) -> ViewLimits {
        // Floating elements don't participate in normal layout
        ViewLimits::fixed(0.0, 0.0)
    }

    fn stretch(&self) -> ViewStretch {
        ViewStretch::new(0.0, 0.0)
    }

    fn draw(&self, ctx: &Context) {
        if !self.is_visible() {
            return;
        }

        let bounds = self.floating_bounds();
        let mut canvas = ctx.canvas.borrow_mut();

        // Shadow
        if self.shadow {
            let shadow_rect = bounds.translate(4.0, 4.0);
            canvas.fill_style(Color::new(0.0, 0.0, 0.0, 0.3));
            canvas.fill_round_rect(shadow_rect, self.corner_radius);
        }

        // Background
        canvas.fill_style(self.background_color);
        canvas.fill_round_rect(bounds, self.corner_radius);

        // Border
        canvas.stroke_style(self.border_color);
        canvas.line_width(1.0);
        canvas.begin_path();
        canvas.add_round_rect(bounds, self.corner_radius);
        canvas.stroke();

        drop(canvas);

        // Content
        if let Some(ref content) = self.content {
            let inset = 8.0;
            let content_bounds = bounds.inset(inset, inset);
            let content_ctx = ctx.with_bounds(content_bounds);
            content.draw(&content_ctx);
        }
    }

    fn hit_test(&self, ctx: &Context, p: Point, leaf: bool, control: bool) -> Option<&dyn Element> {
        if !self.is_visible() {
            return None;
        }

        let bounds = self.floating_bounds();
        if bounds.contains(p) {
            if let Some(ref content) = self.content {
                let inset = 8.0;
                let content_bounds = bounds.inset(inset, inset);
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
        self.is_visible() && self.draggable
    }

    fn handle_click(&self, ctx: &Context, btn: MouseButton) -> bool {
        if !self.is_visible() || btn.button != MouseButtonKind::Left {
            return false;
        }

        let bounds = self.floating_bounds();

        if btn.down {
            if bounds.contains(btn.pos) {
                // Check if clicking on content first
                if let Some(ref content) = self.content {
                    let inset = 8.0;
                    let content_bounds = bounds.inset(inset, inset);
                    let content_ctx = ctx.with_bounds(content_bounds);
                    if content.handle_click(&content_ctx, btn) {
                        return true;
                    }
                }

                // Start dragging
                if self.draggable {
                    *self.dragging.write().unwrap() = true;
                    let pos = *self.position.read().unwrap();
                    *self.drag_offset.write().unwrap() = Point::new(btn.pos.x - pos.x, btn.pos.y - pos.y);
                }
                return true;
            }
        } else {
            *self.dragging.write().unwrap() = false;

            // Forward to content
            if let Some(ref content) = self.content {
                let inset = 8.0;
                let content_bounds = bounds.inset(inset, inset);
                let content_ctx = ctx.with_bounds(content_bounds);
                if content.handle_click(&content_ctx, btn) {
                    return true;
                }
            }
        }

        bounds.contains(btn.pos)
    }

    fn drag(&mut self, _ctx: &Context, btn: MouseButton) {
        if *self.dragging.read().unwrap() {
            let offset = *self.drag_offset.read().unwrap();
            *self.position.write().unwrap() = Point::new(btn.pos.x - offset.x, btn.pos.y - offset.y);
        }
    }

    fn as_any(&self) -> &dyn Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }
}

/// Creates a floating element.
pub fn floating() -> Floating {
    Floating::new()
}
