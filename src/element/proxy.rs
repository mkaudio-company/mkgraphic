//! Proxy elements that wrap and delegate to a subject element.
//!
//! Proxies are elements that encapsulate another element (the "subject")
//! and delegate most operations to it, while potentially augmenting or
//! overriding certain behaviors.

use std::any::Any;
use super::{Element, ElementPtr, ViewLimits, ViewStretch, FocusRequest};
use super::context::{BasicContext, Context};
use crate::support::point::Point;
use crate::view::{MouseButton, KeyInfo, TextInfo, DropInfo, CursorTracking};

/// Base trait for proxy elements.
pub trait ProxyBase: Element {
    /// Returns a reference to the subject element.
    fn subject(&self) -> &dyn Element;

    /// Returns a mutable reference to the subject element.
    fn subject_mut(&mut self) -> &mut dyn Element;

    /// Prepares the context for the subject (e.g., adjusts bounds).
    fn prepare_subject(&self, ctx: &mut Context) {}

    /// Restores the context after subject operations.
    fn restore_subject(&self, ctx: &mut Context) {}
}

/// A generic proxy that wraps any element.
pub struct Proxy<S: Element> {
    subject: S,
}

impl<S: Element> Proxy<S> {
    /// Creates a new proxy wrapping the given subject.
    pub fn new(subject: S) -> Self {
        Self { subject }
    }

    /// Returns a reference to the actual subject type.
    pub fn actual_subject(&self) -> &S {
        &self.subject
    }

    /// Returns a mutable reference to the actual subject type.
    pub fn actual_subject_mut(&mut self) -> &mut S {
        &mut self.subject
    }
}

impl<S: Element + 'static> ProxyBase for Proxy<S> {
    fn subject(&self) -> &dyn Element {
        &self.subject
    }

    fn subject_mut(&mut self) -> &mut dyn Element {
        &mut self.subject
    }
}

impl<S: Element + 'static> Element for Proxy<S> {
    fn limits(&self, ctx: &BasicContext) -> ViewLimits {
        self.subject.limits(ctx)
    }

    fn stretch(&self) -> ViewStretch {
        self.subject.stretch()
    }

    fn span(&self) -> u32 {
        self.subject.span()
    }

    fn hit_test(&self, ctx: &Context, p: Point, leaf: bool, control: bool) -> Option<&dyn Element> {
        self.subject.hit_test(ctx, p, leaf, control)
    }

    fn draw(&self, ctx: &Context) {
        self.subject.draw(ctx);
    }

    fn layout(&mut self, ctx: &Context) {
        self.subject.layout(ctx);
    }

    fn refresh(&self, ctx: &Context, outward: i32) {
        self.subject.refresh(ctx, outward);
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

    fn enable(&mut self, state: bool) {
        self.subject.enable(state);
    }

    fn is_enabled(&self) -> bool {
        self.subject.is_enabled()
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

    fn track_drop(&mut self, ctx: &Context, info: &DropInfo, status: CursorTracking) {
        self.subject.track_drop(ctx, info, status);
    }

    fn drop(&mut self, ctx: &Context, info: &DropInfo) -> bool {
        self.subject.drop(ctx, info)
    }

    fn as_any(&self) -> &dyn Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }
}

/// A proxy that holds an element pointer (Arc).
pub struct RefProxy {
    subject: ElementPtr,
}

impl RefProxy {
    /// Creates a new ref proxy.
    pub fn new(subject: ElementPtr) -> Self {
        Self { subject }
    }

    /// Returns the element pointer.
    pub fn ptr(&self) -> &ElementPtr {
        &self.subject
    }
}

impl Element for RefProxy {
    fn limits(&self, ctx: &BasicContext) -> ViewLimits {
        self.subject.limits(ctx)
    }

    fn stretch(&self) -> ViewStretch {
        self.subject.stretch()
    }

    fn span(&self) -> u32 {
        self.subject.span()
    }

    fn hit_test(&self, ctx: &Context, p: Point, leaf: bool, control: bool) -> Option<&dyn Element> {
        self.subject.hit_test(ctx, p, leaf, control)
    }

    fn draw(&self, ctx: &Context) {
        self.subject.draw(ctx);
    }

    fn wants_control(&self) -> bool {
        self.subject.wants_control()
    }

    fn is_enabled(&self) -> bool {
        self.subject.is_enabled()
    }

    fn wants_focus(&self) -> bool {
        self.subject.wants_focus()
    }

    fn focus(&self) -> Option<&dyn Element> {
        self.subject.focus()
    }

    fn as_any(&self) -> &dyn Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }
}
