//! Context types for element rendering and event handling.

use std::cell::RefCell;

use crate::support::point::Point;
use crate::support::rect::Rect;
use crate::support::canvas::Canvas;
use crate::view::View;
use super::Element;

/// Basic context containing view and canvas references.
///
/// The canvas is wrapped in RefCell to allow mutable access during drawing
/// while keeping the context itself immutable.
pub struct BasicContext<'a> {
    pub view: &'a View,
    pub canvas: &'a RefCell<Canvas>,
}

impl<'a> BasicContext<'a> {
    /// Creates a new basic context.
    pub fn new(view: &'a View, canvas: &'a RefCell<Canvas>) -> Self {
        Self { view, canvas }
    }

    /// Returns the bounds of the view.
    pub fn view_bounds(&self) -> Rect {
        self.view.bounds()
    }

    /// Returns the current cursor position.
    pub fn cursor_pos(&self) -> Point {
        self.view.cursor_pos()
    }
}

/// Full context with element bounds and hierarchy information.
///
/// The canvas is wrapped in RefCell to allow mutable access during drawing
/// while keeping the context itself immutable. This matches the C++ semantics
/// where draw() is logically const but modifies the canvas.
pub struct Context<'a> {
    pub view: &'a View,
    pub canvas: &'a RefCell<Canvas>,
    pub element: Option<&'a dyn Element>,
    pub parent: Option<&'a Context<'a>>,
    pub bounds: Rect,
    pub enabled: bool,
}

impl<'a> Context<'a> {
    /// Creates a new root context.
    pub fn new(view: &'a View, canvas: &'a RefCell<Canvas>, bounds: Rect) -> Self {
        Self {
            view,
            canvas,
            element: None,
            parent: None,
            bounds,
            enabled: true,
        }
    }

    /// Creates a child context with the given bounds.
    /// Note: parent is not set in this version to avoid lifetime complexity.
    pub fn with_bounds(&self, bounds: Rect) -> Context<'a> {
        Context {
            view: self.view,
            canvas: self.canvas,
            element: self.element,
            parent: None, // Cannot set parent due to lifetime constraints
            bounds,
            enabled: self.enabled,
        }
    }

    /// Returns the bounds of the view.
    pub fn view_bounds(&self) -> Rect {
        self.view.bounds()
    }

    /// Returns the current cursor position.
    pub fn cursor_pos(&self) -> Point {
        self.view.cursor_pos()
    }

    /// Returns true if the context is enabled.
    pub fn is_enabled(&self) -> bool {
        self.enabled
    }
}

/// A context builder for creating child contexts.
pub struct ContextBuilder<'a> {
    view: &'a View,
    element: Option<&'a dyn Element>,
    parent: Option<&'a Context<'a>>,
    bounds: Rect,
    enabled: bool,
}

impl<'a> ContextBuilder<'a> {
    /// Creates a new context builder from a parent context.
    pub fn from_parent(parent: &'a Context<'a>) -> Self {
        Self {
            view: parent.view,
            element: parent.element,
            parent: Some(parent),
            bounds: parent.bounds,
            enabled: parent.enabled,
        }
    }

    /// Sets the bounds.
    pub fn bounds(mut self, bounds: Rect) -> Self {
        self.bounds = bounds;
        self
    }

    /// Sets the element.
    pub fn element(mut self, element: &'a dyn Element) -> Self {
        self.element = Some(element);
        self.enabled = self.enabled && element.is_enabled();
        self
    }

    /// Builds the context.
    pub fn build(self, canvas: &'a RefCell<Canvas>) -> Context<'a> {
        Context {
            view: self.view,
            canvas,
            element: self.element,
            parent: self.parent,
            bounds: self.bounds,
            enabled: self.enabled,
        }
    }
}
