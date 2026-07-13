//! Margin elements for adding spacing around children.

use super::context::{BasicContext, Context};
use super::{Element, FocusRequest, ViewLimits};
use crate::support::point::Point;
use crate::support::rect::Rect;
use crate::view::{CursorTracking, KeyInfo, MouseButton, TextInfo};
use std::any::Any;

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
        Self {
            left,
            top,
            right,
            bottom,
        }
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

    fn stretch(&self) -> super::ViewStretch {
        // Without this, every `margin(...)` used the `Element` trait's
        // default stretch (1.0, 1.0) regardless of what the wrapped
        // subject actually wants -- `limits()` right above already
        // correctly delegates to the subject, but `stretch()` didn't, so a
        // margin-wrapped non-stretchy element (e.g. a fixed-size checkbox)
        // still claimed a full, equal share of a `VTile`/`HTile`'s "extra"
        // space alongside genuinely stretchy siblings. Since the
        // margin-wrapped element's own `limits().max` correctly still
        // capped it at its true (non-growing) size, that claimed share
        // then got clamped straight back down and simply discarded --
        // never reaching the sibling that should have received it.
        // Confirmed via direct layout tracing: exactly half of a VTile's
        // available extra height was being computed, allocated to a
        // `margin(4.0, checkbox(...))`, and then thrown away this way,
        // leaving a matching blank gap at the bottom of the window.
        self.subject.stretch()
    }

    fn draw(&self, ctx: &Context) {
        let adjusted_bounds = self.adjust_bounds(ctx.bounds);
        let adjusted_ctx = ctx.with_bounds(adjusted_bounds);
        self.subject.draw(&adjusted_ctx);
    }

    fn draw_overlay(&self, ctx: &Context) {
        let adjusted_bounds = self.adjust_bounds(ctx.bounds);
        let adjusted_ctx = ctx.with_bounds(adjusted_bounds);
        self.subject.draw_overlay(&adjusted_ctx);
    }

    fn layout(&mut self, ctx: &Context) {
        let adjusted_bounds = self.adjust_bounds(ctx.bounds);
        let adjusted_ctx = ctx.with_bounds(adjusted_bounds);
        self.subject.layout(&adjusted_ctx);
    }

    fn hit_test(&self, ctx: &Context, p: Point, leaf: bool, control: bool) -> Option<&dyn Element> {
        let adjusted_bounds = self.adjust_bounds(ctx.bounds);
        let adjusted_ctx = ctx.with_bounds(adjusted_bounds);
        // Let the subject determine if it handles the point
        // This allows popups/dropdowns that extend beyond bounds to receive hits
        self.subject.hit_test(&adjusted_ctx, p, leaf, control)
    }

    fn wants_control(&self) -> bool {
        self.subject.wants_control()
    }

    fn click(&mut self, ctx: &Context, btn: MouseButton) -> bool {
        let adjusted_bounds = self.adjust_bounds(ctx.bounds);
        let adjusted_ctx = ctx.with_bounds(adjusted_bounds);
        self.subject.click(&adjusted_ctx, btn)
    }

    fn handle_click(&self, ctx: &Context, btn: MouseButton) -> bool {
        let adjusted_bounds = self.adjust_bounds(ctx.bounds);
        let adjusted_ctx = ctx.with_bounds(adjusted_bounds);
        self.subject.handle_click(&adjusted_ctx, btn)
    }

    fn drag(&mut self, ctx: &Context, btn: MouseButton) {
        let adjusted_bounds = self.adjust_bounds(ctx.bounds);
        let adjusted_ctx = ctx.with_bounds(adjusted_bounds);
        self.subject.drag(&adjusted_ctx, btn);
    }

    fn handle_drag(&self, ctx: &Context, btn: MouseButton) {
        let adjusted_bounds = self.adjust_bounds(ctx.bounds);
        let adjusted_ctx = ctx.with_bounds(adjusted_bounds);
        self.subject.handle_drag(&adjusted_ctx, btn);
    }

    fn key(&mut self, ctx: &Context, k: KeyInfo) -> bool {
        self.subject.key(ctx, k)
    }

    fn handle_key(&self, ctx: &Context, k: KeyInfo) -> bool {
        self.subject.handle_key(ctx, k)
    }

    fn text(&mut self, ctx: &Context, info: TextInfo) -> bool {
        self.subject.text(ctx, info)
    }

    fn handle_text(&self, ctx: &Context, info: TextInfo) -> bool {
        self.subject.handle_text(ctx, info)
    }

    fn cursor(&mut self, ctx: &Context, p: Point, status: CursorTracking) -> bool {
        self.subject.cursor(ctx, p, status)
    }

    fn scroll(&mut self, ctx: &Context, dir: Point, p: Point) -> bool {
        self.subject.scroll(ctx, dir, p)
    }

    fn handle_scroll(&self, ctx: &Context, dir: Point, p: Point) -> bool {
        let adjusted_bounds = self.adjust_bounds(ctx.bounds);
        let adjusted_ctx = ctx.with_bounds(adjusted_bounds);
        self.subject.handle_scroll(&adjusted_ctx, dir, p)
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

    fn clear_focus(&self) {
        self.subject.clear_focus();
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::element::tile::VTile;
    use crate::element::ViewStretch;
    use crate::element::{composite::CompositeBase, share};
    use crate::support::canvas::Canvas;

    struct NonStretchy;
    impl Element for NonStretchy {
        fn limits(&self, _ctx: &BasicContext) -> ViewLimits {
            ViewLimits::fixed(50.0, 26.0)
        }
        fn stretch(&self) -> ViewStretch {
            ViewStretch::new(0.0, 0.0)
        }
        fn as_any(&self) -> &dyn Any {
            self
        }
        fn as_any_mut(&mut self) -> &mut dyn Any {
            self
        }
    }

    struct Stretchy;
    impl Element for Stretchy {
        fn limits(&self, _ctx: &BasicContext) -> ViewLimits {
            ViewLimits::min_size(50.0, 100.0)
        }
        fn stretch(&self) -> ViewStretch {
            ViewStretch::new(0.0, 1.0)
        }
        fn as_any(&self) -> &dyn Any {
            self
        }
        fn as_any_mut(&mut self) -> &mut dyn Any {
            self
        }
    }

    /// `margin(...)` used to report the `Element` trait's default stretch
    /// (1.0, 1.0) regardless of what its wrapped subject actually wanted --
    /// `limits()` already correctly delegated, `stretch()` didn't. In a
    /// `VTile` alongside a genuinely stretchy sibling, this meant a
    /// margin-wrapped *non*-stretchy element (e.g. a fixed-size checkbox)
    /// still claimed an equal share of "extra" space in the stretch
    /// calculation -- which its own (correctly delegated) `limits().max`
    /// then immediately clamped back down, discarding that share instead
    /// of it reaching the sibling that should have received it. Net
    /// effect: part of a window's height went nowhere, rendering as a
    /// blank gap. This reproduces the exact scenario (a `margin`-wrapped
    /// fixed checkbox next to a stretchy sibling in a `VTile`) and checks
    /// the stretchy sibling receives *all* the extra, not half of it.
    #[test]
    fn margin_delegates_stretch_to_its_subject() {
        let wrapped = MarginElement::new(Margin::uniform(4.0), NonStretchy);
        assert_eq!(
            wrapped.stretch(),
            ViewStretch::new(0.0, 0.0),
            "margin should report its subject's stretch, not the Element trait's default (1,1)"
        );

        let vtile = VTile::from_vec(vec![share(wrapped), share(Stretchy)]);
        let view = crate::view::View::new(crate::support::point::Extent::new(200.0, 400.0));
        let canvas = std::cell::RefCell::new(Canvas::new(200, 400).unwrap());
        let bounds = Rect::new(0.0, 0.0, 200.0, 400.0);
        let ctx = Context::new(&view, &canvas, bounds);

        let non_stretchy_bounds = vtile.bounds_of(&ctx, 0);
        let stretchy_bounds = vtile.bounds_of(&ctx, 1);

        // Margin-wrapped element: exactly its own min (4+4 margin + 26
        // content = 34), never any share of "extra".
        assert_eq!(non_stretchy_bounds.height(), 34.0);
        // The stretchy sibling should receive *all* 366pt of extra
        // (400 total - 34 margin-wrapped - 100 stretchy min - wait: total
        // min = 34 + 100 = 134, extra = 400 - 134 = 266), not half of it
        // discarded by the margin-wrapped sibling clamping its share away.
        assert_eq!(
            stretchy_bounds.height(),
            400.0 - 34.0,
            "stretchy sibling should absorb all the extra space, none of it lost"
        );
    }
}
