//! Tile layout elements (vtile, htile).
//!
//! Tiles arrange elements in vertical or horizontal sequences.

use std::any::Any;
use std::sync::RwLock;
use super::{Element, ElementPtr, ViewLimits, FocusRequest, FULL_EXTENT, share};
use super::context::{BasicContext, Context};
use super::composite::{Storage, CompositeBase, Composite};
use crate::support::point::Point;
use crate::support::rect::Rect;

/// Vertical tile element - stacks children vertically.
pub struct VTile {
    inner: Composite,
    tiles: RwLock<Vec<f32>>,
}

impl VTile {
    /// Creates a new empty vertical tile.
    pub fn new() -> Self {
        Self {
            inner: Composite::new(),
            tiles: RwLock::new(Vec::new()),
        }
    }

    /// Creates a vertical tile from a vector of elements.
    pub fn from_vec(children: Vec<ElementPtr>) -> Self {
        let len = children.len();
        Self {
            inner: Composite::from_vec(children),
            tiles: RwLock::new(vec![0.0; len + 1]),
        }
    }

    /// Adds an element.
    pub fn push(&mut self, element: ElementPtr) {
        self.inner.push(element);
        self.tiles.write().unwrap().push(0.0);
    }

    fn compute_layout(&self, ctx: &BasicContext, height: f32) -> Vec<f32> {
        let count = self.inner.len();
        if count == 0 {
            return vec![0.0];
        }

        let mut tiles = vec![0.0; count + 1];
        let mut total_min = 0.0f32;
        let mut total_stretch = 0.0f32;

        // Calculate minimum heights and stretch factors
        for i in 0..count {
            if let Some(child) = self.inner.at(i) {
                let limits = child.limits(ctx);
                let stretch = child.stretch();
                total_min += limits.min.y;
                total_stretch += stretch.y;
            }
        }

        // Distribute extra space
        let extra = (height - total_min).max(0.0);
        let mut y = 0.0f32;

        for i in 0..count {
            tiles[i] = y;
            if let Some(child) = self.inner.at(i) {
                let limits = child.limits(ctx);
                let stretch = child.stretch();

                let mut elem_height = limits.min.y;
                if total_stretch > 0.0 {
                    let alloc = extra * (stretch.y / total_stretch);
                    elem_height = (elem_height + alloc).min(limits.max.y);
                }
                y += elem_height;
            }
        }
        tiles[count] = y;

        tiles
    }
}

impl Default for VTile {
    fn default() -> Self {
        Self::new()
    }
}

impl Storage for VTile {
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

impl CompositeBase for VTile {
    fn bounds_of(&self, ctx: &Context, index: usize) -> Rect {
        // Compute layout if needed
        // tiles should have count+1 elements, and the last element should be non-zero if properly computed
        let count = self.inner.len();
        {
            let mut tiles = self.tiles.write().unwrap();
            // Recompute if wrong size or not yet computed (last element is 0)
            let needs_compute = tiles.len() != count + 1 ||
                (count > 0 && tiles.get(count).map_or(true, |&v| v == 0.0));
            if needs_compute && count > 0 {
                let basic_ctx = BasicContext::new(ctx.view, ctx.canvas);
                let height = ctx.bounds.height();
                *tiles = self.compute_layout(&basic_ctx, height);
            }
        }

        let tiles = self.tiles.read().unwrap();
        if index >= tiles.len().saturating_sub(1) {
            return Rect::zero();
        }

        Rect {
            left: ctx.bounds.left,
            top: ctx.bounds.top + tiles[index],
            right: ctx.bounds.right,
            bottom: ctx.bounds.top + tiles[index + 1],
        }
    }
}

impl Element for VTile {
    fn limits(&self, ctx: &BasicContext) -> ViewLimits {
        let mut min_height = 0.0f32;
        let mut max_height = 0.0f32;
        let mut min_width = 0.0f32;
        let mut max_width = FULL_EXTENT;

        for i in 0..self.inner.len() {
            if let Some(child) = self.inner.at(i) {
                let limits = child.limits(ctx);
                min_height += limits.min.y;
                max_height += limits.max.y;
                min_width = min_width.max(limits.min.x);
                max_width = max_width.min(limits.max.x);
            }
        }

        ViewLimits {
            min: Point::new(min_width, min_height),
            max: Point::new(max_width.max(min_width), max_height.max(min_height)),
        }
    }

    fn draw(&self, ctx: &Context) {
        for i in 0..self.inner.len() {
            if let Some(child) = self.inner.at(i) {
                let bounds = self.bounds_of(ctx, i);
                if crate::support::rect::intersects(&bounds, &ctx.bounds) {
                    let child_ctx = ctx.with_bounds(bounds);
                    child.draw(&child_ctx);
                }
            }
        }
    }

    fn layout(&mut self, _ctx: &Context) {
        // Layout is handled by allocate
    }

    fn hit_test(&self, ctx: &Context, p: Point, leaf: bool, control: bool) -> Option<&dyn Element> {
        if !ctx.bounds.contains(p) {
            return None;
        }

        for i in 0..self.inner.len() {
            let bounds = self.bounds_of(ctx, i);
            if bounds.contains(p) {
                if let Some(child) = self.inner.at(i) {
                    let child_ctx = ctx.with_bounds(bounds);
                    if let Some(hit) = child.hit_test(&child_ctx, p, leaf, control) {
                        return Some(hit);
                    }
                }
            }
        }

        if leaf { None } else { Some(self) }
    }

    fn handle_click(&self, ctx: &Context, btn: crate::view::MouseButton) -> bool {
        // Find child at click position and forward the click
        for i in 0..self.inner.len() {
            let bounds = self.bounds_of(ctx, i);
            if bounds.contains(btn.pos) {
                if let Some(child) = self.inner.at(i) {
                    let child_ctx = ctx.with_bounds(bounds);
                    if child.handle_click(&child_ctx, btn) {
                        return true;
                    }
                }
            }
        }
        false
    }

    fn wants_control(&self) -> bool {
        self.inner.wants_control()
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

/// Horizontal tile element - arranges children horizontally.
pub struct HTile {
    inner: Composite,
    tiles: RwLock<Vec<f32>>,
}

impl HTile {
    /// Creates a new empty horizontal tile.
    pub fn new() -> Self {
        Self {
            inner: Composite::new(),
            tiles: RwLock::new(Vec::new()),
        }
    }

    /// Creates a horizontal tile from a vector of elements.
    pub fn from_vec(children: Vec<ElementPtr>) -> Self {
        let len = children.len();
        Self {
            inner: Composite::from_vec(children),
            tiles: RwLock::new(vec![0.0; len + 1]),
        }
    }

    /// Adds an element.
    pub fn push(&mut self, element: ElementPtr) {
        self.inner.push(element);
        self.tiles.write().unwrap().push(0.0);
    }

    fn compute_layout(&self, ctx: &BasicContext, width: f32) -> Vec<f32> {
        let count = self.inner.len();
        if count == 0 {
            return vec![0.0];
        }

        let mut tiles = vec![0.0; count + 1];
        let mut total_min = 0.0f32;
        let mut total_stretch = 0.0f32;

        for i in 0..count {
            if let Some(child) = self.inner.at(i) {
                let limits = child.limits(ctx);
                let stretch = child.stretch();
                total_min += limits.min.x;
                total_stretch += stretch.x;
            }
        }

        let extra = (width - total_min).max(0.0);
        let mut x = 0.0f32;

        for i in 0..count {
            tiles[i] = x;
            if let Some(child) = self.inner.at(i) {
                let limits = child.limits(ctx);
                let stretch = child.stretch();

                let mut elem_width = limits.min.x;
                if total_stretch > 0.0 {
                    let alloc = extra * (stretch.x / total_stretch);
                    elem_width = (elem_width + alloc).min(limits.max.x);
                }
                x += elem_width;
            }
        }
        tiles[count] = x;

        tiles
    }
}

impl Default for HTile {
    fn default() -> Self {
        Self::new()
    }
}

impl Storage for HTile {
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

impl CompositeBase for HTile {
    fn bounds_of(&self, ctx: &Context, index: usize) -> Rect {
        // Compute layout if needed
        let count = self.inner.len();
        {
            let mut tiles = self.tiles.write().unwrap();
            // Recompute if wrong size or not yet computed (last element is 0)
            let needs_compute = tiles.len() != count + 1 ||
                (count > 0 && tiles.get(count).map_or(true, |&v| v == 0.0));
            if needs_compute && count > 0 {
                let basic_ctx = BasicContext::new(ctx.view, ctx.canvas);
                let width = ctx.bounds.width();
                *tiles = self.compute_layout(&basic_ctx, width);
            }
        }

        let tiles = self.tiles.read().unwrap();
        if index >= tiles.len().saturating_sub(1) {
            return Rect::zero();
        }

        Rect {
            left: ctx.bounds.left + tiles[index],
            top: ctx.bounds.top,
            right: ctx.bounds.left + tiles[index + 1],
            bottom: ctx.bounds.bottom,
        }
    }
}

impl Element for HTile {
    fn limits(&self, ctx: &BasicContext) -> ViewLimits {
        let mut min_width = 0.0f32;
        let mut max_width = 0.0f32;
        let mut min_height = 0.0f32;
        let mut max_height = FULL_EXTENT;

        for i in 0..self.inner.len() {
            if let Some(child) = self.inner.at(i) {
                let limits = child.limits(ctx);
                min_width += limits.min.x;
                max_width += limits.max.x;
                min_height = min_height.max(limits.min.y);
                max_height = max_height.min(limits.max.y);
            }
        }

        ViewLimits {
            min: Point::new(min_width, min_height),
            max: Point::new(max_width.max(min_width), max_height.max(min_height)),
        }
    }

    fn draw(&self, ctx: &Context) {
        for i in 0..self.inner.len() {
            if let Some(child) = self.inner.at(i) {
                let bounds = self.bounds_of(ctx, i);
                if crate::support::rect::intersects(&bounds, &ctx.bounds) {
                    let child_ctx = ctx.with_bounds(bounds);
                    child.draw(&child_ctx);
                }
            }
        }
    }

    fn hit_test(&self, ctx: &Context, p: Point, leaf: bool, control: bool) -> Option<&dyn Element> {
        if !ctx.bounds.contains(p) {
            return None;
        }

        for i in 0..self.inner.len() {
            let bounds = self.bounds_of(ctx, i);
            if bounds.contains(p) {
                if let Some(child) = self.inner.at(i) {
                    let child_ctx = ctx.with_bounds(bounds);
                    if let Some(hit) = child.hit_test(&child_ctx, p, leaf, control) {
                        return Some(hit);
                    }
                }
            }
        }

        if leaf { None } else { Some(self) }
    }

    fn handle_click(&self, ctx: &Context, btn: crate::view::MouseButton) -> bool {
        // Find child at click position and forward the click
        for i in 0..self.inner.len() {
            let bounds = self.bounds_of(ctx, i);
            if bounds.contains(btn.pos) {
                if let Some(child) = self.inner.at(i) {
                    let child_ctx = ctx.with_bounds(bounds);
                    if child.handle_click(&child_ctx, btn) {
                        return true;
                    }
                }
            }
        }
        false
    }

    fn wants_control(&self) -> bool {
        self.inner.wants_control()
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

/// Creates a vertical tile from elements.
pub fn vtile<E: Element + 'static>(elements: Vec<E>) -> VTile {
    let ptrs: Vec<ElementPtr> = elements.into_iter().map(|e| share(e)).collect();
    VTile::from_vec(ptrs)
}

/// Creates a horizontal tile from elements.
pub fn htile<E: Element + 'static>(elements: Vec<E>) -> HTile {
    let ptrs: Vec<ElementPtr> = elements.into_iter().map(|e| share(e)).collect();
    HTile::from_vec(ptrs)
}

/// Macro for creating vertical tiles.
#[macro_export]
macro_rules! vtile {
    ($($elem:expr),* $(,)?) => {{
        let mut tile = $crate::element::tile::VTile::new();
        $(
            tile.push($crate::element::share($elem));
        )*
        tile
    }};
}

/// Macro for creating horizontal tiles.
#[macro_export]
macro_rules! htile {
    ($($elem:expr),* $(,)?) => {{
        let mut tile = $crate::element::tile::HTile::new();
        $(
            tile.push($crate::element::share($elem));
        )*
        tile
    }};
}
