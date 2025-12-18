//! Size constraint elements.

use std::any::Any;
use super::{Element, ViewLimits, ViewStretch, FocusRequest};
use super::context::{BasicContext, Context};
use crate::support::point::Point;
use crate::view::{MouseButton, KeyInfo, TextInfo, CursorTracking};

/// Fixed size element.
pub struct FixedSize<S: Element> {
    subject: S,
    width: f32,
    height: f32,
}

impl<S: Element> FixedSize<S> {
    /// Creates a new fixed size element.
    pub fn new(width: f32, height: f32, subject: S) -> Self {
        Self { subject, width, height }
    }

    /// Returns the width.
    pub fn width(&self) -> f32 {
        self.width
    }

    /// Returns the height.
    pub fn height(&self) -> f32 {
        self.height
    }

    /// Sets the size.
    pub fn set_size(&mut self, width: f32, height: f32) {
        self.width = width;
        self.height = height;
    }
}

impl<S: Element + 'static> Element for FixedSize<S> {
    fn limits(&self, ctx: &BasicContext) -> ViewLimits {
        ViewLimits::fixed(self.width, self.height)
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

    fn handle_click(&self, ctx: &Context, btn: MouseButton) -> bool {
        self.subject.handle_click(ctx, btn)
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

    fn handle_drag(&self, ctx: &Context, btn: MouseButton) {
        self.subject.handle_drag(ctx, btn);
    }

    fn handle_key(&self, ctx: &Context, k: KeyInfo) -> bool {
        self.subject.handle_key(ctx, k)
    }

    fn handle_text(&self, ctx: &Context, info: TextInfo) -> bool {
        self.subject.handle_text(ctx, info)
    }

    fn handle_scroll(&self, ctx: &Context, dir: Point, p: Point) -> bool {
        self.subject.handle_scroll(ctx, dir, p)
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

/// Minimum size constraint element.
pub struct MinSize<S: Element> {
    subject: S,
    min_width: f32,
    min_height: f32,
}

impl<S: Element> MinSize<S> {
    /// Creates a new minimum size element.
    pub fn new(min_width: f32, min_height: f32, subject: S) -> Self {
        Self { subject, min_width, min_height }
    }
}

impl<S: Element + 'static> Element for MinSize<S> {
    fn limits(&self, ctx: &BasicContext) -> ViewLimits {
        let e_limits = self.subject.limits(ctx);
        ViewLimits {
            min: Point::new(
                e_limits.min.x.max(self.min_width),
                e_limits.min.y.max(self.min_height),
            ),
            max: e_limits.max,
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

    fn handle_click(&self, ctx: &Context, btn: MouseButton) -> bool {
        self.subject.handle_click(ctx, btn)
    }

    fn handle_drag(&self, ctx: &Context, btn: MouseButton) {
        self.subject.handle_drag(ctx, btn);
    }

    fn handle_key(&self, ctx: &Context, k: KeyInfo) -> bool {
        self.subject.handle_key(ctx, k)
    }

    fn handle_text(&self, ctx: &Context, info: TextInfo) -> bool {
        self.subject.handle_text(ctx, info)
    }

    fn handle_scroll(&self, ctx: &Context, dir: Point, p: Point) -> bool {
        self.subject.handle_scroll(ctx, dir, p)
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

/// Maximum size constraint element.
pub struct MaxSize<S: Element> {
    subject: S,
    max_width: f32,
    max_height: f32,
}

impl<S: Element> MaxSize<S> {
    /// Creates a new maximum size element.
    pub fn new(max_width: f32, max_height: f32, subject: S) -> Self {
        Self { subject, max_width, max_height }
    }
}

impl<S: Element + 'static> Element for MaxSize<S> {
    fn limits(&self, ctx: &BasicContext) -> ViewLimits {
        let e_limits = self.subject.limits(ctx);
        ViewLimits {
            min: e_limits.min,
            max: Point::new(
                e_limits.max.x.min(self.max_width),
                e_limits.max.y.min(self.max_height),
            ),
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

    fn handle_click(&self, ctx: &Context, btn: MouseButton) -> bool {
        self.subject.handle_click(ctx, btn)
    }

    fn handle_drag(&self, ctx: &Context, btn: MouseButton) {
        self.subject.handle_drag(ctx, btn);
    }

    fn handle_key(&self, ctx: &Context, k: KeyInfo) -> bool {
        self.subject.handle_key(ctx, k)
    }

    fn handle_text(&self, ctx: &Context, info: TextInfo) -> bool {
        self.subject.handle_text(ctx, info)
    }

    fn handle_scroll(&self, ctx: &Context, dir: Point, p: Point) -> bool {
        self.subject.handle_scroll(ctx, dir, p)
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

/// Stretch factor element.
pub struct Stretch<S: Element> {
    subject: S,
    stretch: ViewStretch,
}

impl<S: Element> Stretch<S> {
    /// Creates a new stretch element.
    pub fn new(x: f32, y: f32, subject: S) -> Self {
        Self {
            subject,
            stretch: ViewStretch::new(x, y),
        }
    }

    /// Creates a stretch element with uniform stretch factor.
    pub fn uniform(factor: f32, subject: S) -> Self {
        Self {
            subject,
            stretch: ViewStretch::uniform(factor),
        }
    }
}

impl<S: Element + 'static> Element for Stretch<S> {
    fn limits(&self, ctx: &BasicContext) -> ViewLimits {
        self.subject.limits(ctx)
    }

    fn stretch(&self) -> ViewStretch {
        self.stretch
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

    fn handle_click(&self, ctx: &Context, btn: MouseButton) -> bool {
        self.subject.handle_click(ctx, btn)
    }

    fn handle_drag(&self, ctx: &Context, btn: MouseButton) {
        self.subject.handle_drag(ctx, btn);
    }

    fn handle_key(&self, ctx: &Context, k: KeyInfo) -> bool {
        self.subject.handle_key(ctx, k)
    }

    fn handle_text(&self, ctx: &Context, info: TextInfo) -> bool {
        self.subject.handle_text(ctx, info)
    }

    fn handle_scroll(&self, ctx: &Context, dir: Point, p: Point) -> bool {
        self.subject.handle_scroll(ctx, dir, p)
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

/// Creates a fixed size element.
pub fn fixed_size<S: Element>(width: f32, height: f32, subject: S) -> FixedSize<S> {
    FixedSize::new(width, height, subject)
}

/// Creates a minimum size element.
pub fn min_size<S: Element>(min_width: f32, min_height: f32, subject: S) -> MinSize<S> {
    MinSize::new(min_width, min_height, subject)
}

/// Creates a maximum size element.
pub fn max_size<S: Element>(max_width: f32, max_height: f32, subject: S) -> MaxSize<S> {
    MaxSize::new(max_width, max_height, subject)
}

/// Creates a horizontal stretch element.
pub fn hstretch<S: Element>(factor: f32, subject: S) -> Stretch<S> {
    Stretch::new(factor, 1.0, subject)
}

/// Creates a vertical stretch element.
pub fn vstretch<S: Element>(factor: f32, subject: S) -> Stretch<S> {
    Stretch::new(1.0, factor, subject)
}

/// Creates a stretch element.
pub fn stretch<S: Element>(x: f32, y: f32, subject: S) -> Stretch<S> {
    Stretch::new(x, y, subject)
}

/// Creates a zero-stretch element (doesn't expand).
pub fn no_stretch<S: Element>(subject: S) -> Stretch<S> {
    Stretch::new(0.0, 0.0, subject)
}
