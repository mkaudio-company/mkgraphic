//! Composite elements that contain multiple child elements.

use std::any::Any;
use std::collections::HashSet;
use super::{Element, ElementPtr, ViewLimits, FocusRequest};
use super::context::{BasicContext, Context};
use crate::support::point::Point;
use crate::support::rect::Rect;

/// Storage trait for accessing elements by index.
pub trait Storage {
    /// Returns the number of elements.
    fn len(&self) -> usize;

    /// Returns true if empty.
    fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Returns the element at the given index.
    fn at(&self, index: usize) -> Option<&dyn Element>;

    /// Returns a mutable reference to the element at the given index.
    fn at_mut(&mut self, index: usize) -> Option<&mut dyn Element>;
}

/// Hit information for composite elements.
#[derive(Debug, Clone)]
pub struct HitInfo {
    pub element_index: Option<usize>,
    pub leaf_element: bool,
    pub bounds: Rect,
}

impl Default for HitInfo {
    fn default() -> Self {
        Self {
            element_index: None,
            leaf_element: false,
            bounds: Rect::zero(),
        }
    }
}

/// Base trait for composite elements.
pub trait CompositeBase: Element + Storage {
    /// Returns the bounds of the element at the given index.
    fn bounds_of(&self, ctx: &Context, index: usize) -> Rect;

    /// Returns true if indices should be processed in reverse order.
    fn reverse_index(&self) -> bool {
        false
    }

    /// Performs hit testing on child elements.
    fn hit_element(&self, ctx: &Context, p: Point, control: bool) -> HitInfo {
        let mut info = HitInfo::default();

        let indices: Box<dyn Iterator<Item = usize>> = if self.reverse_index() {
            Box::new((0..self.len()).rev())
        } else {
            Box::new(0..self.len())
        };

        for i in indices {
            if let Some(element) = self.at(i) {
                let bounds = self.bounds_of(ctx, i);
                if bounds.contains(p) {
                    info.element_index = Some(i);
                    info.bounds = bounds;
                    info.leaf_element = element.hit_test(ctx, p, true, control).is_some();
                    if !control || element.wants_control() {
                        break;
                    }
                }
            }
        }

        info
    }

    /// Calls a function for each visible element.
    fn for_each_visible<F>(&self, ctx: &Context, reverse: bool, mut f: F)
    where
        F: FnMut(&dyn Element, usize, Rect) -> bool,
    {
        let indices: Box<dyn Iterator<Item = usize>> = if reverse {
            Box::new((0..self.len()).rev())
        } else {
            Box::new(0..self.len())
        };

        for i in indices {
            if let Some(element) = self.at(i) {
                let bounds = self.bounds_of(ctx, i);
                // Check if bounds intersect with view bounds
                if crate::support::rect::intersects(&bounds, &ctx.bounds) {
                    if !f(element, i, bounds) {
                        break;
                    }
                }
            }
        }
    }
}

/// A basic composite element using a vector of element pointers.
pub struct Composite {
    children: Vec<ElementPtr>,
    focus_index: Option<usize>,
    saved_focus: Option<usize>,
    click_tracking: Option<usize>,
    cursor_tracking: Option<usize>,
    cursor_hovering: HashSet<usize>,
    enabled: bool,
    cached_bounds: Vec<Rect>,
}

impl Composite {
    /// Creates a new empty composite.
    pub fn new() -> Self {
        Self {
            children: Vec::new(),
            focus_index: None,
            saved_focus: None,
            click_tracking: None,
            cursor_tracking: None,
            cursor_hovering: HashSet::new(),
            enabled: true,
            cached_bounds: Vec::new(),
        }
    }

    /// Creates a composite from a vector of elements.
    pub fn from_vec(children: Vec<ElementPtr>) -> Self {
        let len = children.len();
        Self {
            children,
            focus_index: None,
            saved_focus: None,
            click_tracking: None,
            cursor_tracking: None,
            cursor_hovering: HashSet::new(),
            enabled: true,
            cached_bounds: vec![Rect::zero(); len],
        }
    }

    /// Adds an element to the composite.
    pub fn push(&mut self, element: ElementPtr) {
        self.children.push(element);
        self.cached_bounds.push(Rect::zero());
    }

    /// Removes and returns the last element.
    pub fn pop(&mut self) -> Option<ElementPtr> {
        self.cached_bounds.pop();
        self.children.pop()
    }

    /// Clears all elements.
    pub fn clear(&mut self) {
        self.children.clear();
        self.cached_bounds.clear();
        self.focus_index = None;
        self.saved_focus = None;
    }

    /// Returns a slice of the children.
    pub fn children(&self) -> &[ElementPtr] {
        &self.children
    }

    /// Returns the focus index.
    pub fn focus_index(&self) -> Option<usize> {
        self.focus_index
    }

    /// Sets the focus index.
    pub fn set_focus(&mut self, index: Option<usize>) {
        self.focus_index = index;
    }

    /// Resets tracking state.
    pub fn reset(&mut self) {
        self.click_tracking = None;
        self.cursor_tracking = None;
        self.cursor_hovering.clear();
    }
}

impl Default for Composite {
    fn default() -> Self {
        Self::new()
    }
}

impl Storage for Composite {
    fn len(&self) -> usize {
        self.children.len()
    }

    fn at(&self, index: usize) -> Option<&dyn Element> {
        self.children.get(index).map(|e| e.as_ref())
    }

    fn at_mut(&mut self, index: usize) -> Option<&mut dyn Element> {
        // Note: This requires interior mutability pattern for Arc
        // For now, return None as we can't get mutable access to Arc contents
        None
    }
}

impl CompositeBase for Composite {
    fn bounds_of(&self, ctx: &Context, index: usize) -> Rect {
        self.cached_bounds.get(index).copied().unwrap_or(Rect::zero())
    }
}

impl Element for Composite {
    fn limits(&self, ctx: &BasicContext) -> ViewLimits {
        // Default: combine limits of all children
        let mut limits = ViewLimits::new(
            Point::new(0.0, 0.0),
            Point::new(0.0, 0.0),
        );

        for child in &self.children {
            let child_limits = child.limits(ctx);
            limits.min.x = limits.min.x.max(child_limits.min.x);
            limits.min.y = limits.min.y.max(child_limits.min.y);
            limits.max.x = limits.max.x.max(child_limits.max.x);
            limits.max.y = limits.max.y.max(child_limits.max.y);
        }

        limits
    }

    fn draw(&self, ctx: &Context) {
        for (i, child) in self.children.iter().enumerate() {
            let bounds = self.bounds_of(ctx, i);
            if crate::support::rect::intersects(&bounds, &ctx.bounds) {
                // Would need to create a child context with the element's bounds
                child.draw(ctx);
            }
        }
    }

    fn hit_test(&self, ctx: &Context, p: Point, leaf: bool, control: bool) -> Option<&dyn Element> {
        if !ctx.bounds.contains(p) {
            return None;
        }

        let hit = self.hit_element(ctx, p, control);
        if let Some(index) = hit.element_index {
            if let Some(child) = self.at(index) {
                return child.hit_test(ctx, p, leaf, control);
            }
        }

        if leaf {
            None
        } else {
            Some(self)
        }
    }

    fn wants_control(&self) -> bool {
        self.children.iter().any(|c| c.wants_control())
    }

    fn enable(&mut self, state: bool) {
        self.enabled = state;
    }

    fn is_enabled(&self) -> bool {
        self.enabled
    }

    fn wants_focus(&self) -> bool {
        self.children.iter().any(|c| c.wants_focus())
    }

    fn begin_focus(&mut self, req: FocusRequest) {
        match req {
            FocusRequest::FromTop => {
                // Find first focusable child
                for (i, child) in self.children.iter().enumerate() {
                    if child.wants_focus() {
                        self.focus_index = Some(i);
                        break;
                    }
                }
            }
            FocusRequest::FromBottom => {
                // Find last focusable child
                for (i, child) in self.children.iter().enumerate().rev() {
                    if child.wants_focus() {
                        self.focus_index = Some(i);
                        break;
                    }
                }
            }
            FocusRequest::RestorePrevious => {
                self.focus_index = self.saved_focus;
            }
        }
    }

    fn end_focus(&mut self) -> bool {
        self.saved_focus = self.focus_index;
        self.focus_index = None;
        true
    }

    fn focus(&self) -> Option<&dyn Element> {
        self.focus_index
            .and_then(|i| self.children.get(i))
            .map(|e| e.as_ref())
    }

    fn as_any(&self) -> &dyn Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }
}

/// A fixed-size array composite.
pub struct ArrayComposite<const N: usize> {
    children: [Option<ElementPtr>; N],
    focus_index: Option<usize>,
    enabled: bool,
    cached_bounds: [Rect; N],
}

impl<const N: usize> ArrayComposite<N> {
    /// Creates a new array composite with empty slots.
    pub fn new() -> Self {
        Self {
            children: std::array::from_fn(|_| None),
            focus_index: None,
            enabled: true,
            cached_bounds: [Rect::zero(); N],
        }
    }

    /// Sets the element at the given index.
    pub fn set(&mut self, index: usize, element: ElementPtr) {
        if index < N {
            self.children[index] = Some(element);
        }
    }

    /// Returns the number of filled slots.
    pub fn count(&self) -> usize {
        self.children.iter().filter(|c| c.is_some()).count()
    }
}

impl<const N: usize> Default for ArrayComposite<N> {
    fn default() -> Self {
        Self::new()
    }
}

impl<const N: usize> Storage for ArrayComposite<N> {
    fn len(&self) -> usize {
        N
    }

    fn at(&self, index: usize) -> Option<&dyn Element> {
        self.children.get(index)?.as_ref().map(|e| e.as_ref())
    }

    fn at_mut(&mut self, index: usize) -> Option<&mut dyn Element> {
        None
    }
}

impl<const N: usize> CompositeBase for ArrayComposite<N> {
    fn bounds_of(&self, ctx: &Context, index: usize) -> Rect {
        self.cached_bounds.get(index).copied().unwrap_or(Rect::zero())
    }
}

impl<const N: usize> Element for ArrayComposite<N> {
    fn limits(&self, ctx: &BasicContext) -> ViewLimits {
        ViewLimits::full()
    }

    fn draw(&self, ctx: &Context) {
        for (i, child) in self.children.iter().enumerate() {
            if let Some(child) = child {
                let bounds = self.bounds_of(ctx, i);
                if crate::support::rect::intersects(&bounds, &ctx.bounds) {
                    child.draw(ctx);
                }
            }
        }
    }

    fn is_enabled(&self) -> bool {
        self.enabled
    }

    fn enable(&mut self, state: bool) {
        self.enabled = state;
    }

    fn as_any(&self) -> &dyn Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }
}
