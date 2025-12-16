//! Margin elements for adding spacing around children.

use std::any::Any;
use super::{Element, ViewLimits, FocusRequest};
use super::context::{BasicContext, Context};
use crate::support::point::Point;
use crate::support::rect::Rect;
use crate::view::{MouseButton, KeyInfo, TextInfo, CursorTracking};

/// Margin values for all four sides.
#[derive(Debug, Clone, Copy, Default)]
pub struct Margin {
    pub left: f32,
    pub top: f32,
    pub right: f32,
    pub bottom: f32,
}

impl Margin {
    /// Creates new margins with the given values.
    pub const fn new(left: f32, top: f32, right: f32, bottom: f32) -> Self {
        Self { left, top, right, bottom }
    }

    /// Creates uniform margins.
    pub const fn uniform(value: f32) -> Self {
        Self {
            left: value,
            top: value,
            right: value,
            bottom: value,
        }
    }

    /// Creates horizontal margins (left and right).
    pub const fn horizontal(value: f32) -> Self {
        Self {
            left: value,
            top: 0.0,
            right: value,
            bottom: 0.0,
        }
    }

    /// Creates vertical margins (top and bottom).
    pub const fn vertical(value: f32) -> Self {
        Self {
            left: 0.0,
            top: value,
            right: 0.0,
            bottom: value,
        }
    }

    /// Returns the total horizontal margin.
    pub fn width(&self) -> f32 {
        self.left + self.right
    }

    /// Returns the total vertical margin.
    pub fn height(&self) -> f32 {
        self.top + self.bottom
    }
}

impl From<f32> for Margin {
    fn from(value: f32) -> Self {
        Self::uniform(value)
    }
}

impl From<(f32, f32)> for Margin {
    fn from((h, v): (f32, f32)) -> Self {
        Self::new(h, v, h, v)
    }
}

impl From<(f32, f32, f32, f32)> for Margin {
    fn from((l, t, r, b): (f32, f32, f32, f32)) -> Self {
        Self::new(l, t, r, b)
    }
}

impl From<Rect> for Margin {
    fn from(r: Rect) -> Self {
        Self::new(r.left, r.top, r.right, r.bottom)
    }
}

/// Margin element that adds spacing around its subject.
pub struct MarginElement<S: Element> {
    subject: S,
    margin: Margin,
}

impl<S: Element> MarginElement<S> {
    /// Creates a new margin element.
    pub fn new(margin: impl Into<Margin>, subject: S) -> Self {
        Self {
            subject,
            margin: margin.into(),
        }
    }

    /// Returns the margin.
    pub fn margin(&self) -> &Margin {
        &self.margin
    }

    /// Sets the margin.
    pub fn set_margin(&mut self, margin: impl Into<Margin>) {
        self.margin = margin.into();
    }

    /// Returns a reference to the subject.
    pub fn subject(&self) -> &S {
        &self.subject
    }

    /// Returns a mutable reference to the subject.
    pub fn subject_mut(&mut self) -> &mut S {
        &mut self.subject
    }

    fn adjust_bounds(&self, bounds: Rect) -> Rect {
        Rect {
            left: bounds.left + self.margin.left,
            top: bounds.top + self.margin.top,
            right: bounds.right - self.margin.right,
            bottom: bounds.bottom - self.margin.bottom,
        }
    }
}

impl<S: Element + 'static> Element for MarginElement<S> {
    fn limits(&self, ctx: &BasicContext) -> ViewLimits {
        let e_limits = self.subject.limits(ctx);
        let margin_w = self.margin.width();
        let margin_h = self.margin.height();

        ViewLimits {
            min: Point::new(e_limits.min.x + margin_w, e_limits.min.y + margin_h),
            max: Point::new(e_limits.max.x + margin_w, e_limits.max.y + margin_h),
        }
    }

    fn draw(&self, ctx: &Context) {
        self.subject.draw(ctx);
    }

    fn layout(&mut self, ctx: &Context) {
        self.subject.layout(ctx);
    }

    fn hit_test(&self, ctx: &Context, p: Point, leaf: bool, control: bool) -> Option<&dyn Element> {
        // Adjust point for margin
        let adjusted_p = Point::new(p.x - self.margin.left, p.y - self.margin.top);
        self.subject.hit_test(ctx, adjusted_p, leaf, control)
    }

    fn wants_control(&self) -> bool {
        self.subject.wants_control()
    }

    fn click(&mut self, ctx: &Context, btn: MouseButton) -> bool {
        self.subject.click(ctx, btn)
    }

    fn drag(&mut self, ctx: &Context, btn: MouseButton) {
        self.subject.drag(ctx, btn);
    }

    fn key(&mut self, ctx: &Context, k: KeyInfo) -> bool {
        self.subject.key(ctx, k)
    }

    fn text(&mut self, ctx: &Context, info: TextInfo) -> bool {
        self.subject.text(ctx, info)
    }

    fn cursor(&mut self, ctx: &Context, p: Point, status: CursorTracking) -> bool {
        self.subject.cursor(ctx, p, status)
    }

    fn scroll(&mut self, ctx: &Context, dir: Point, p: Point) -> bool {
        self.subject.scroll(ctx, dir, p)
    }

    fn is_enabled(&self) -> bool {
        self.subject.is_enabled()
    }

    fn enable(&mut self, state: bool) {
        self.subject.enable(state);
    }

    fn wants_focus(&self) -> bool {
        self.subject.wants_focus()
    }

    fn begin_focus(&mut self, req: FocusRequest) {
        self.subject.begin_focus(req);
    }

    fn end_focus(&mut self) -> bool {
        self.subject.end_focus()
    }

    fn focus(&self) -> Option<&dyn Element> {
        self.subject.focus()
    }

    fn focus_mut(&mut self) -> Option<&mut dyn Element> {
        self.subject.focus_mut()
    }

    fn as_any(&self) -> &dyn Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }
}

// Convenience functions

/// Adds margin to an element.
pub fn margin<S: Element>(margin: impl Into<Margin>, subject: S) -> MarginElement<S> {
    MarginElement::new(margin, subject)
}

/// Adds left margin to an element.
pub fn margin_left<S: Element>(value: f32, subject: S) -> MarginElement<S> {
    MarginElement::new(Margin::new(value, 0.0, 0.0, 0.0), subject)
}

/// Adds right margin to an element.
pub fn margin_right<S: Element>(value: f32, subject: S) -> MarginElement<S> {
    MarginElement::new(Margin::new(0.0, 0.0, value, 0.0), subject)
}

/// Adds top margin to an element.
pub fn margin_top<S: Element>(value: f32, subject: S) -> MarginElement<S> {
    MarginElement::new(Margin::new(0.0, value, 0.0, 0.0), subject)
}

/// Adds bottom margin to an element.
pub fn margin_bottom<S: Element>(value: f32, subject: S) -> MarginElement<S> {
    MarginElement::new(Margin::new(0.0, 0.0, 0.0, value), subject)
}

/// Adds horizontal margin to an element.
pub fn margin_horizontal<S: Element>(value: f32, subject: S) -> MarginElement<S> {
    MarginElement::new(Margin::horizontal(value), subject)
}

/// Adds vertical margin to an element.
pub fn margin_vertical<S: Element>(value: f32, subject: S) -> MarginElement<S> {
    MarginElement::new(Margin::vertical(value), subject)
}
