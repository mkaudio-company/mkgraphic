//! Alignment elements for positioning children within their bounds.

use std::any::Any;
use super::{Element, ViewLimits, FocusRequest, FULL_EXTENT};
use super::context::{BasicContext, Context};
use crate::support::point::Point;
use crate::support::rect::Rect;
use crate::view::{MouseButton, KeyInfo, TextInfo, CursorTracking};

/// Horizontal alignment element.
pub struct HAlign<S: Element> {
    subject: S,
    align: f32,
}

impl<S: Element> HAlign<S> {
    /// Creates a new horizontal alignment element.
    ///
    /// `align` should be between 0.0 (left) and 1.0 (right).
    pub fn new(align: f32, subject: S) -> Self {
        Self {
            subject,
            align: align.clamp(0.0, 1.0),
        }
    }

    /// Returns the alignment value.
    pub fn align(&self) -> f32 {
        self.align
    }

    /// Sets the alignment value.
    pub fn set_align(&mut self, align: f32) {
        self.align = align.clamp(0.0, 1.0);
    }

    fn prepare_bounds(&self, ctx: &Context) -> Rect {
        // This would normally use ctx to get subject limits
        let bounds = ctx.bounds;
        // Simplified: just return bounds as-is
        bounds
    }
}

impl<S: Element + 'static> Element for HAlign<S> {
    fn limits(&self, ctx: &BasicContext) -> ViewLimits {
        let e_limits = self.subject.limits(ctx);
        ViewLimits {
            min: Point::new(e_limits.min.x, e_limits.min.y),
            max: Point::new(FULL_EXTENT, e_limits.max.y),
        }
    }

    fn draw(&self, ctx: &Context) {
        self.subject.draw(ctx);
    }

    fn layout(&mut self, ctx: &Context) {
        self.subject.layout(ctx);
    }

    fn hit_test(&self, ctx: &Context, p: Point, leaf: bool, control: bool) -> Option<&dyn Element> {
        self.subject.hit_test(ctx, p, leaf, control)
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

/// Vertical alignment element.
pub struct VAlign<S: Element> {
    subject: S,
    align: f32,
}

impl<S: Element> VAlign<S> {
    /// Creates a new vertical alignment element.
    ///
    /// `align` should be between 0.0 (top) and 1.0 (bottom).
    pub fn new(align: f32, subject: S) -> Self {
        Self {
            subject,
            align: align.clamp(0.0, 1.0),
        }
    }

    /// Returns the alignment value.
    pub fn align(&self) -> f32 {
        self.align
    }

    /// Sets the alignment value.
    pub fn set_align(&mut self, align: f32) {
        self.align = align.clamp(0.0, 1.0);
    }
}

impl<S: Element + 'static> Element for VAlign<S> {
    fn limits(&self, ctx: &BasicContext) -> ViewLimits {
        let e_limits = self.subject.limits(ctx);
        ViewLimits {
            min: Point::new(e_limits.min.x, e_limits.min.y),
            max: Point::new(e_limits.max.x, FULL_EXTENT),
        }
    }

    fn draw(&self, ctx: &Context) {
        self.subject.draw(ctx);
    }

    fn layout(&mut self, ctx: &Context) {
        self.subject.layout(ctx);
    }

    fn hit_test(&self, ctx: &Context, p: Point, leaf: bool, control: bool) -> Option<&dyn Element> {
        self.subject.hit_test(ctx, p, leaf, control)
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

/// Horizontally aligns an element.
pub fn halign<S: Element>(align: f32, subject: S) -> HAlign<S> {
    HAlign::new(align, subject)
}

/// Left-aligns an element.
pub fn align_left<S: Element>(subject: S) -> HAlign<S> {
    HAlign::new(0.0, subject)
}

/// Center-aligns an element horizontally.
pub fn align_center<S: Element>(subject: S) -> HAlign<S> {
    HAlign::new(0.5, subject)
}

/// Right-aligns an element.
pub fn align_right<S: Element>(subject: S) -> HAlign<S> {
    HAlign::new(1.0, subject)
}

/// Vertically aligns an element.
pub fn valign<S: Element>(align: f32, subject: S) -> VAlign<S> {
    VAlign::new(align, subject)
}

/// Top-aligns an element.
pub fn align_top<S: Element>(subject: S) -> VAlign<S> {
    VAlign::new(0.0, subject)
}

/// Middle-aligns an element vertically.
pub fn align_middle<S: Element>(subject: S) -> VAlign<S> {
    VAlign::new(0.5, subject)
}

/// Bottom-aligns an element.
pub fn align_bottom<S: Element>(subject: S) -> VAlign<S> {
    VAlign::new(1.0, subject)
}

/// Aligns an element both horizontally and vertically.
pub fn align_left_top<S: Element>(subject: S) -> HAlign<VAlign<S>> {
    align_left(align_top(subject))
}

/// Aligns an element to center-top.
pub fn align_center_top<S: Element>(subject: S) -> HAlign<VAlign<S>> {
    align_center(align_top(subject))
}

/// Aligns an element to right-top.
pub fn align_right_top<S: Element>(subject: S) -> HAlign<VAlign<S>> {
    align_right(align_top(subject))
}

/// Aligns an element to left-middle.
pub fn align_left_middle<S: Element>(subject: S) -> HAlign<VAlign<S>> {
    align_left(align_middle(subject))
}

/// Aligns an element to center-middle.
pub fn align_center_middle<S: Element>(subject: S) -> HAlign<VAlign<S>> {
    align_center(align_middle(subject))
}

/// Aligns an element to right-middle.
pub fn align_right_middle<S: Element>(subject: S) -> HAlign<VAlign<S>> {
    align_right(align_middle(subject))
}

/// Aligns an element to left-bottom.
pub fn align_left_bottom<S: Element>(subject: S) -> HAlign<VAlign<S>> {
    align_left(align_bottom(subject))
}

/// Aligns an element to center-bottom.
pub fn align_center_bottom<S: Element>(subject: S) -> HAlign<VAlign<S>> {
    align_center(align_bottom(subject))
}

/// Aligns an element to right-bottom.
pub fn align_right_bottom<S: Element>(subject: S) -> HAlign<VAlign<S>> {
    align_right(align_bottom(subject))
}
