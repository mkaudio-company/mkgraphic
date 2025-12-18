//! Element module - the core of the UI framework.
//!
//! Elements are the fundamental building blocks of the UI. This module provides:
//!
//! - [`Element`]: The base trait for all UI elements
//! - [`proxy`]: Proxy elements that wrap other elements
//! - [`composite`]: Container elements that hold multiple children
//! - [`tile`]: Layout elements (vtile, htile)
//! - [`align`]: Alignment elements
//! - [`margin`]: Margin elements
//! - [`size`]: Size constraint elements
//! - [`layer`]: Layered elements

pub mod context;
pub mod proxy;
pub mod composite;
pub mod tile;
pub mod align;
pub mod margin;
pub mod size;
pub mod layer;
pub mod label;
pub mod button;

use std::sync::{Arc, Weak};
use std::any::Any;

use crate::support::point::{Point, Axis};
use crate::view::{MouseButton, KeyInfo, TextInfo, DropInfo, CursorTracking};

/// The maximum extent value (effectively infinite).
pub const FULL_EXTENT: f32 = 1e30;

/// View limits define the minimum and maximum sizes of an element.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ViewLimits {
    pub min: Point,
    pub max: Point,
}

impl ViewLimits {
    /// Creates new view limits.
    pub const fn new(min: Point, max: Point) -> Self {
        Self { min, max }
    }

    /// Creates view limits with zero minimum and full extent maximum.
    pub const fn full() -> Self {
        Self {
            min: Point::new(0.0, 0.0),
            max: Point::new(FULL_EXTENT, FULL_EXTENT),
        }
    }

    /// Creates fixed-size view limits.
    pub const fn fixed(width: f32, height: f32) -> Self {
        Self {
            min: Point::new(width, height),
            max: Point::new(width, height),
        }
    }

    /// Creates view limits with a minimum size.
    pub const fn min_size(width: f32, height: f32) -> Self {
        Self {
            min: Point::new(width, height),
            max: Point::new(FULL_EXTENT, FULL_EXTENT),
        }
    }

    /// Returns the minimum value for the given axis.
    pub fn min_for(&self, axis: Axis) -> f32 {
        self.min[axis]
    }

    /// Returns the maximum value for the given axis.
    pub fn max_for(&self, axis: Axis) -> f32 {
        self.max[axis]
    }
}

impl Default for ViewLimits {
    fn default() -> Self {
        Self::full()
    }
}

/// View stretch defines how an element stretches to fill available space.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ViewStretch {
    pub x: f32,
    pub y: f32,
}

impl ViewStretch {
    /// Creates a new view stretch.
    pub const fn new(x: f32, y: f32) -> Self {
        Self { x, y }
    }

    /// Creates a uniform view stretch.
    pub const fn uniform(value: f32) -> Self {
        Self { x: value, y: value }
    }

    /// Returns the stretch value for the given axis.
    pub fn for_axis(&self, axis: Axis) -> f32 {
        match axis {
            Axis::X => self.x,
            Axis::Y => self.y,
        }
    }
}

impl Default for ViewStretch {
    fn default() -> Self {
        Self { x: 1.0, y: 1.0 }
    }
}

/// Focus request type.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FocusRequest {
    /// Make the topmost element the focus.
    FromTop,
    /// Make the bottommost element the focus.
    FromBottom,
    /// Restore the previous focus state.
    RestorePrevious,
}

/// Tracking state for mouse interactions.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Tracking {
    /// No tracking is happening.
    None,
    /// Tracking has just started.
    Begin,
    /// Tracking is ongoing.
    While,
    /// Tracking has just ended.
    End,
}

/// The base trait for all UI elements.
///
/// Elements are lightweight objects that handle rendering, event processing,
/// and layout calculations. They form a hierarchical tree structure where
/// composite elements can contain child elements.
pub trait Element: Send + Sync + Any {
    // --- Display ---

    /// Returns the size limits of this element.
    fn limits(&self, ctx: &BasicContext) -> ViewLimits {
        ViewLimits::full()
    }

    /// Returns the stretch factor of this element.
    fn stretch(&self) -> ViewStretch {
        ViewStretch::default()
    }

    /// Returns the span (for grid layouts).
    fn span(&self) -> u32 {
        1
    }

    /// Performs hit testing to find the element at the given point.
    ///
    /// Returns `Some` if this element is hit, `None` otherwise.
    /// The default implementation returns `None` - concrete types should override this
    /// to return `Some(self)` when hit.
    fn hit_test(&self, ctx: &Context, p: Point, leaf: bool, control: bool) -> Option<&dyn Element> {
        None
    }

    /// Returns true if the element contains the given point (within current bounds).
    fn contains(&self, ctx: &Context, p: Point) -> bool {
        ctx.bounds.contains(p)
    }

    /// Draws this element.
    fn draw(&self, ctx: &Context) {}

    /// Performs layout calculations.
    fn layout(&mut self, ctx: &Context) {}

    /// Refreshes the element, triggering a redraw.
    fn refresh(&self, ctx: &Context, outward: i32) {}

    // --- Control ---

    /// Returns true if this element wants to receive control events.
    fn wants_control(&self) -> bool {
        false
    }

    /// Handles mouse click events.
    ///
    /// Returns true if the event was handled.
    fn click(&mut self, ctx: &Context, btn: MouseButton) -> bool {
        false
    }

    /// Handles mouse click events (immutable version for use with Arc).
    ///
    /// Returns true if the event was handled.
    /// Default implementation returns false - override this for elements
    /// that need to handle clicks through Arc<dyn Element>.
    fn handle_click(&self, _ctx: &Context, _btn: MouseButton) -> bool {
        false
    }

    /// Handles mouse drag events.
    fn drag(&mut self, ctx: &Context, btn: MouseButton) {}

    /// Handles keyboard events.
    ///
    /// Returns true if the event was handled.
    fn key(&mut self, ctx: &Context, k: KeyInfo) -> bool {
        false
    }

    /// Handles text input events.
    ///
    /// Returns true if the event was handled.
    fn text(&mut self, ctx: &Context, info: TextInfo) -> bool {
        false
    }

    /// Handles cursor (mouse move) events.
    ///
    /// Returns true if the event was handled.
    fn cursor(&mut self, ctx: &Context, p: Point, status: CursorTracking) -> bool {
        false
    }

    /// Handles scroll events.
    ///
    /// Returns true if the event was handled.
    fn scroll(&mut self, ctx: &Context, dir: Point, p: Point) -> bool {
        false
    }

    /// Enables or disables the element.
    fn enable(&mut self, state: bool) {}

    /// Returns true if the element is enabled.
    fn is_enabled(&self) -> bool {
        true
    }

    // --- Focus ---

    /// Returns true if this element wants to receive focus.
    fn wants_focus(&self) -> bool {
        false
    }

    /// Called when the element begins receiving focus.
    fn begin_focus(&mut self, req: FocusRequest) {}

    /// Called when the element loses focus.
    ///
    /// Returns true if focus was successfully released.
    fn end_focus(&mut self) -> bool {
        true
    }

    /// Returns the currently focused child element, if any.
    fn focus(&self) -> Option<&dyn Element> {
        None
    }

    /// Returns a mutable reference to the currently focused child element, if any.
    fn focus_mut(&mut self) -> Option<&mut dyn Element> {
        None
    }

    // --- Drag and Drop ---

    /// Handles drag tracking events.
    fn track_drop(&mut self, ctx: &Context, info: &DropInfo, status: CursorTracking) {}

    /// Handles drop events.
    ///
    /// Returns true if the drop was accepted.
    fn drop(&mut self, ctx: &Context, info: &DropInfo) -> bool {
        false
    }

    // --- Type info ---

    /// Returns the class name of this element (for debugging).
    fn class_name(&self) -> &'static str {
        std::any::type_name::<Self>()
    }

    /// Returns this element as Any for downcasting.
    fn as_any(&self) -> &dyn Any;

    /// Returns this element as mutable Any for downcasting.
    fn as_any_mut(&mut self) -> &mut dyn Any;
}

/// A shared pointer to an element.
pub type ElementPtr = Arc<dyn Element>;

/// A weak pointer to an element.
pub type WeakElementPtr = Weak<dyn Element>;

/// Creates a shared element pointer.
pub fn share<E: Element + 'static>(element: E) -> ElementPtr {
    Arc::new(element)
}

/// An empty element that does nothing.
#[derive(Debug, Clone, Copy, Default)]
pub struct Empty;

impl Element for Empty {
    fn as_any(&self) -> &dyn Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }
}

/// Creates an empty element.
pub fn empty() -> Empty {
    Empty
}

// Re-exports
pub use context::{BasicContext, Context};
pub use proxy::{Proxy, ProxyBase};
pub use composite::{Composite, CompositeBase, Storage};
