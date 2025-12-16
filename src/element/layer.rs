//! Layer elements for stacking children on top of each other.

use std::any::Any;
use super::{Element, ElementPtr, ViewLimits, FocusRequest, share};
use super::context::{BasicContext, Context};
use super::composite::{Storage, CompositeBase, Composite};
use crate::support::point::Point;
use crate::support::rect::Rect;
use crate::view::MouseButton;

/// Layer element - stacks children on top of each other.
///
/// All children occupy the same bounds. The last child is drawn on top.
pub struct Layer {
    inner: Composite,
}

impl Layer {
    /// Creates a new empty layer.
    pub fn new() -> Self {
        Self {
            inner: Composite::new(),
        }
    }

    /// Creates a layer from a vector of elements.
    pub fn from_vec(children: Vec<ElementPtr>) -> Self {
        Self {
            inner: Composite::from_vec(children),
        }
    }

    /// Adds an element on top.
    pub fn push(&mut self, element: ElementPtr) {
        self.inner.push(element);
    }

    /// Removes and returns the top element.
    pub fn pop(&mut self) -> Option<ElementPtr> {
        self.inner.pop()
    }

    /// Clears all elements.
    pub fn clear(&mut self) {
        self.inner.clear();
    }

    /// Returns the number of layers.
    pub fn count(&self) -> usize {
        self.inner.len()
    }
}

impl Default for Layer {
    fn default() -> Self {
        Self::new()
    }
}

impl Storage for Layer {
    fn len(&self) -> usize {
        self.inner.len()
    }

    fn at(&self, index: usize) -> Option<&dyn Element> {
        self.inner.at(index)
    }

    fn at_mut(&mut self, index: usize) -> Option<&mut dyn Element> {
        self.inner.at_mut(index)
    }
}

impl CompositeBase for Layer {
    fn bounds_of(&self, ctx: &Context, index: usize) -> Rect {
        // All layers have the same bounds
        ctx.bounds
    }

    fn reverse_index(&self) -> bool {
        true // Hit test from top to bottom
    }
}

impl Element for Layer {
    fn limits(&self, ctx: &BasicContext) -> ViewLimits {
        // Return the union of all children's limits
        let mut limits = ViewLimits::new(
            Point::new(0.0, 0.0),
            Point::new(super::FULL_EXTENT, super::FULL_EXTENT),
        );

        for i in 0..self.inner.len() {
            if let Some(child) = self.inner.at(i) {
                let child_limits = child.limits(ctx);
                limits.min.x = limits.min.x.max(child_limits.min.x);
                limits.min.y = limits.min.y.max(child_limits.min.y);
                // For max, we use min of maxes to be safe
                limits.max.x = limits.max.x.min(child_limits.max.x);
                limits.max.y = limits.max.y.min(child_limits.max.y);
            }
        }

        // Ensure max >= min
        limits.max.x = limits.max.x.max(limits.min.x);
        limits.max.y = limits.max.y.max(limits.min.y);

        limits
    }

    fn draw(&self, ctx: &Context) {
        // Draw from bottom to top
        for i in 0..self.inner.len() {
            if let Some(child) = self.inner.at(i) {
                child.draw(ctx);
            }
        }
    }

    fn layout(&mut self, ctx: &Context) {
        // All children get the same bounds
        // In a real implementation, we'd update each child's layout
    }

    fn hit_test(&self, ctx: &Context, p: Point, leaf: bool, control: bool) -> Option<&dyn Element> {
        if !ctx.bounds.contains(p) {
            return None;
        }

        // Hit test from top to bottom
        for i in (0..self.inner.len()).rev() {
            if let Some(child) = self.inner.at(i) {
                if let Some(hit) = child.hit_test(ctx, p, leaf, control) {
                    return Some(hit);
                }
            }
        }

        if leaf { None } else { Some(self) }
    }

    fn wants_control(&self) -> bool {
        self.inner.wants_control()
    }

    fn click(&mut self, ctx: &Context, btn: MouseButton) -> bool {
        // Delegate to focused layer or top layer
        false
    }

    fn is_enabled(&self) -> bool {
        self.inner.is_enabled()
    }

    fn enable(&mut self, state: bool) {
        self.inner.enable(state);
    }

    fn wants_focus(&self) -> bool {
        self.inner.wants_focus()
    }

    fn begin_focus(&mut self, req: FocusRequest) {
        self.inner.begin_focus(req);
    }

    fn end_focus(&mut self) -> bool {
        self.inner.end_focus()
    }

    fn focus(&self) -> Option<&dyn Element> {
        self.inner.focus()
    }

    fn as_any(&self) -> &dyn Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }
}

/// Creates a layer from elements.
pub fn layer<E: Element + 'static>(elements: Vec<E>) -> Layer {
    let ptrs: Vec<ElementPtr> = elements.into_iter().map(|e| share(e)).collect();
    Layer::from_vec(ptrs)
}

/// Macro for creating layers.
#[macro_export]
macro_rules! layer {
    ($($elem:expr),* $(,)?) => {{
        let mut l = $crate::element::layer::Layer::new();
        $(
            l.push($crate::element::share($elem));
        )*
        l
    }};
}

/// Deck element - only shows one child at a time.
pub struct Deck {
    inner: Composite,
    active_index: usize,
}

impl Deck {
    /// Creates a new empty deck.
    pub fn new() -> Self {
        Self {
            inner: Composite::new(),
            active_index: 0,
        }
    }

    /// Creates a deck from a vector of elements.
    pub fn from_vec(children: Vec<ElementPtr>) -> Self {
        Self {
            inner: Composite::from_vec(children),
            active_index: 0,
        }
    }

    /// Adds an element to the deck.
    pub fn push(&mut self, element: ElementPtr) {
        self.inner.push(element);
    }

    /// Returns the active index.
    pub fn active_index(&self) -> usize {
        self.active_index
    }

    /// Sets the active index.
    pub fn set_active(&mut self, index: usize) {
        if index < self.inner.len() {
            self.active_index = index;
        }
    }

    /// Returns the active element.
    pub fn active(&self) -> Option<&dyn Element> {
        self.inner.at(self.active_index)
    }

    /// Returns the number of cards in the deck.
    pub fn count(&self) -> usize {
        self.inner.len()
    }
}

impl Default for Deck {
    fn default() -> Self {
        Self::new()
    }
}

impl Element for Deck {
    fn limits(&self, ctx: &BasicContext) -> ViewLimits {
        // Return limits of active child
        if let Some(child) = self.inner.at(self.active_index) {
            child.limits(ctx)
        } else {
            ViewLimits::full()
        }
    }

    fn draw(&self, ctx: &Context) {
        // Only draw active child
        if let Some(child) = self.inner.at(self.active_index) {
            child.draw(ctx);
        }
    }

    fn hit_test(&self, ctx: &Context, p: Point, leaf: bool, control: bool) -> Option<&dyn Element> {
        if let Some(child) = self.inner.at(self.active_index) {
            child.hit_test(ctx, p, leaf, control)
        } else {
            None
        }
    }

    fn wants_control(&self) -> bool {
        if let Some(child) = self.inner.at(self.active_index) {
            child.wants_control()
        } else {
            false
        }
    }

    fn is_enabled(&self) -> bool {
        self.inner.is_enabled()
    }

    fn enable(&mut self, state: bool) {
        self.inner.enable(state);
    }

    fn wants_focus(&self) -> bool {
        if let Some(child) = self.inner.at(self.active_index) {
            child.wants_focus()
        } else {
            false
        }
    }

    fn as_any(&self) -> &dyn Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }
}
