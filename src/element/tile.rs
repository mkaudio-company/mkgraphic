//! Tile layout elements (vtile, htile).
//!
//! Tiles arrange elements in vertical or horizontal sequences.

use super::composite::{Composite, CompositeBase, Storage};
use super::context::{BasicContext, Context};
use super::{share, Element, ElementPtr, FocusRequest, ViewLimits, FULL_EXTENT};
use crate::support::point::Point;
use crate::support::rect::Rect;
use std::any::Any;
use std::sync::RwLock;

/// Vertical tile element - stacks children vertically.
pub struct VTile {
    inner: Composite,
    /// Index of the child currently capturing mouse drag events, if any.
    drag_capture: RwLock<Option<usize>>,
}

impl VTile {
    /// Creates a new empty vertical tile.
    pub fn new() -> Self {
        Self {
            inner: Composite::new(),
            drag_capture: RwLock::new(None),
        }
    }

    /// Creates a vertical tile from a vector of elements.
    pub fn from_vec(children: Vec<ElementPtr>) -> Self {
        Self {
            inner: Composite::from_vec(children),
            drag_capture: RwLock::new(None),
        }
    }

    /// Adds an element.
    pub fn push(&mut self, element: ElementPtr) {
        self.inner.push(element);
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

        for (i, tile_slot) in tiles.iter_mut().enumerate().take(count) {
            *tile_slot = y;
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
        eprintln!("[verify] total_y={y} (should equal height={height})");

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
        // Recomputed on every call rather than cached keyed on the
        // container's own outer height: a child's `limits()` can change
        // without the container's height changing at all (e.g. `Splitter`
        // dragging calls `CodeEditor::set_height` on a sibling), and a
        // cache keyed only on outer height has no way to notice that --
        // the previous version of this cache went stale exactly that way,
        // silently freezing the layout the first time something like that
        // happened. Cheap in practice: children counts here are small
        // (a handful), so this is O(children) per call, not a hot path
        // worth a fragile cache for.
        let count = self.inner.len();
        if count == 0 {
            return Rect::zero();
        }
        let height = ctx.bounds.height();
        let basic_ctx = BasicContext::new(ctx.view, ctx.canvas);
        let tiles = self.compute_layout(&basic_ctx, height);

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

        // Second pass: overlays (e.g. an expanded dropdown) always draw last
        // so a later sibling's normal content never paints over them.
        for i in 0..self.inner.len() {
            if let Some(child) = self.inner.at(i) {
                let bounds = self.bounds_of(ctx, i);
                let child_ctx = ctx.with_bounds(bounds);
                child.draw_overlay(&child_ctx);
            }
        }
    }

    fn layout(&mut self, _ctx: &Context) {
        // Layout is handled by allocate
    }

    fn hit_test(&self, ctx: &Context, p: Point, leaf: bool, control: bool) -> Option<&dyn Element> {
        // First check all children - some may have popups extending beyond bounds
        for i in 0..self.inner.len() {
            let bounds = self.bounds_of(ctx, i);
            if let Some(child) = self.inner.at(i) {
                let child_ctx = ctx.with_bounds(bounds);
                if let Some(hit) = child.hit_test(&child_ctx, p, leaf, control) {
                    return Some(hit);
                }
            }
        }

        // If point is within our bounds but no child handled it
        if ctx.bounds.contains(p) {
            if leaf {
                None
            } else {
                Some(self)
            }
        } else {
            None
        }
    }

    fn handle_click(&self, ctx: &Context, btn: crate::view::MouseButton) -> bool {
        // On release, deliver to whichever child captured the drag, regardless
        // of whether the pointer is still within that child's bounds.
        if !btn.down {
            if let Some(index) = self.drag_capture.write().unwrap().take() {
                if let Some(child) = self.inner.at(index) {
                    let bounds = self.bounds_of(ctx, index);
                    let child_ctx = ctx.with_bounds(bounds);
                    return child.handle_click(&child_ctx, btn);
                }
            }
        }

        // Only forward to child that passes hit_test for this position
        for i in 0..self.inner.len() {
            let bounds = self.bounds_of(ctx, i);
            if let Some(child) = self.inner.at(i) {
                let child_ctx = ctx.with_bounds(bounds);
                // Check if this child wants the click via hit_test
                if child.hit_test(&child_ctx, btn.pos, false, false).is_some()
                    && child.handle_click(&child_ctx, btn)
                {
                    if btn.down {
                        *self.drag_capture.write().unwrap() = Some(i);
                    }
                    return true;
                }
            }
        }
        false
    }

    fn handle_drag(&self, ctx: &Context, btn: crate::view::MouseButton) {
        // Route to the child that captured the drag on mouse-down, even if the
        // pointer has since moved outside that child's bounds (e.g. dragging a
        // small dial well beyond its own hit region).
        if let Some(index) = *self.drag_capture.read().unwrap() {
            if let Some(child) = self.inner.at(index) {
                let bounds = self.bounds_of(ctx, index);
                let child_ctx = ctx.with_bounds(bounds);
                child.handle_drag(&child_ctx, btn);
                return;
            }
        }

        for i in 0..self.inner.len() {
            let bounds = self.bounds_of(ctx, i);
            if let Some(child) = self.inner.at(i) {
                let child_ctx = ctx.with_bounds(bounds);
                if child.hit_test(&child_ctx, btn.pos, false, false).is_some() {
                    child.handle_drag(&child_ctx, btn);
                    return;
                }
            }
        }
    }

    fn handle_scroll(
        &self,
        ctx: &Context,
        dir: crate::support::point::Point,
        p: crate::support::point::Point,
    ) -> bool {
        for i in 0..self.inner.len() {
            let bounds = self.bounds_of(ctx, i);
            if let Some(child) = self.inner.at(i) {
                let child_ctx = ctx.with_bounds(bounds);
                if child.hit_test(&child_ctx, p, false, false).is_some()
                    && child.handle_scroll(&child_ctx, dir, p)
                {
                    return true;
                }
            }
        }
        false
    }

    fn handle_key(&self, ctx: &Context, k: crate::view::KeyInfo) -> bool {
        for i in 0..self.inner.len() {
            let bounds = self.bounds_of(ctx, i);
            if let Some(child) = self.inner.at(i) {
                let child_ctx = ctx.with_bounds(bounds);
                if child.handle_key(&child_ctx, k) {
                    return true;
                }
            }
        }
        false
    }

    fn handle_text(&self, ctx: &Context, info: crate::view::TextInfo) -> bool {
        for i in 0..self.inner.len() {
            let bounds = self.bounds_of(ctx, i);
            if let Some(child) = self.inner.at(i) {
                let child_ctx = ctx.with_bounds(bounds);
                if child.handle_text(&child_ctx, info) {
                    return true;
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

    fn clear_focus(&self) {
        for i in 0..self.inner.len() {
            if let Some(child) = self.inner.at(i) {
                child.clear_focus();
            }
        }
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
    /// Index of the child currently capturing mouse drag events, if any.
    drag_capture: RwLock<Option<usize>>,
}

impl HTile {
    /// Creates a new empty horizontal tile.
    pub fn new() -> Self {
        Self {
            inner: Composite::new(),
            drag_capture: RwLock::new(None),
        }
    }

    /// Creates a horizontal tile from a vector of elements.
    pub fn from_vec(children: Vec<ElementPtr>) -> Self {
        Self {
            inner: Composite::from_vec(children),
            drag_capture: RwLock::new(None),
        }
    }

    /// Adds an element.
    pub fn push(&mut self, element: ElementPtr) {
        self.inner.push(element);
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

        for (i, tile_slot) in tiles.iter_mut().enumerate().take(count) {
            *tile_slot = x;
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
        // Recomputed on every call -- see `VTile::bounds_of`'s comment for
        // why a cache keyed only on the container's own outer width can't
        // notice a child's `limits()` changing on its own (e.g. a
        // `Splitter` calling `TreeView::set_width` on a sibling).
        let count = self.inner.len();
        if count == 0 {
            return Rect::zero();
        }
        let width = ctx.bounds.width();
        let basic_ctx = BasicContext::new(ctx.view, ctx.canvas);
        let tiles = self.compute_layout(&basic_ctx, width);

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

        // Second pass: overlays (e.g. an expanded dropdown) always draw last
        // so a later sibling's normal content never paints over them.
        for i in 0..self.inner.len() {
            if let Some(child) = self.inner.at(i) {
                let bounds = self.bounds_of(ctx, i);
                let child_ctx = ctx.with_bounds(bounds);
                child.draw_overlay(&child_ctx);
            }
        }
    }

    fn hit_test(&self, ctx: &Context, p: Point, leaf: bool, control: bool) -> Option<&dyn Element> {
        // First check all children - some may have popups extending beyond bounds
        for i in 0..self.inner.len() {
            let bounds = self.bounds_of(ctx, i);
            if let Some(child) = self.inner.at(i) {
                let child_ctx = ctx.with_bounds(bounds);
                if let Some(hit) = child.hit_test(&child_ctx, p, leaf, control) {
                    return Some(hit);
                }
            }
        }

        // If point is within our bounds but no child handled it
        if ctx.bounds.contains(p) {
            if leaf {
                None
            } else {
                Some(self)
            }
        } else {
            None
        }
    }

    fn handle_click(&self, ctx: &Context, btn: crate::view::MouseButton) -> bool {
        // On release, deliver to whichever child captured the drag, regardless
        // of whether the pointer is still within that child's bounds.
        if !btn.down {
            if let Some(index) = self.drag_capture.write().unwrap().take() {
                if let Some(child) = self.inner.at(index) {
                    let bounds = self.bounds_of(ctx, index);
                    let child_ctx = ctx.with_bounds(bounds);
                    return child.handle_click(&child_ctx, btn);
                }
            }
        }

        // Only forward to child that passes hit_test for this position
        for i in 0..self.inner.len() {
            let bounds = self.bounds_of(ctx, i);
            if let Some(child) = self.inner.at(i) {
                let child_ctx = ctx.with_bounds(bounds);
                // Check if this child wants the click via hit_test
                if child.hit_test(&child_ctx, btn.pos, false, false).is_some()
                    && child.handle_click(&child_ctx, btn)
                {
                    if btn.down {
                        *self.drag_capture.write().unwrap() = Some(i);
                    }
                    return true;
                }
            }
        }
        false
    }

    fn handle_drag(&self, ctx: &Context, btn: crate::view::MouseButton) {
        // Route to the child that captured the drag on mouse-down, even if the
        // pointer has since moved outside that child's bounds (e.g. dragging a
        // small dial well beyond its own hit region).
        if let Some(index) = *self.drag_capture.read().unwrap() {
            if let Some(child) = self.inner.at(index) {
                let bounds = self.bounds_of(ctx, index);
                let child_ctx = ctx.with_bounds(bounds);
                child.handle_drag(&child_ctx, btn);
                return;
            }
        }

        for i in 0..self.inner.len() {
            let bounds = self.bounds_of(ctx, i);
            if let Some(child) = self.inner.at(i) {
                let child_ctx = ctx.with_bounds(bounds);
                if child.hit_test(&child_ctx, btn.pos, false, false).is_some() {
                    child.handle_drag(&child_ctx, btn);
                    return;
                }
            }
        }
    }

    fn handle_scroll(
        &self,
        ctx: &Context,
        dir: crate::support::point::Point,
        p: crate::support::point::Point,
    ) -> bool {
        for i in 0..self.inner.len() {
            let bounds = self.bounds_of(ctx, i);
            if let Some(child) = self.inner.at(i) {
                let child_ctx = ctx.with_bounds(bounds);
                if child.hit_test(&child_ctx, p, false, false).is_some()
                    && child.handle_scroll(&child_ctx, dir, p)
                {
                    return true;
                }
            }
        }
        false
    }

    fn handle_key(&self, ctx: &Context, k: crate::view::KeyInfo) -> bool {
        for i in 0..self.inner.len() {
            let bounds = self.bounds_of(ctx, i);
            if let Some(child) = self.inner.at(i) {
                let child_ctx = ctx.with_bounds(bounds);
                if child.handle_key(&child_ctx, k) {
                    return true;
                }
            }
        }
        false
    }

    fn handle_text(&self, ctx: &Context, info: crate::view::TextInfo) -> bool {
        for i in 0..self.inner.len() {
            let bounds = self.bounds_of(ctx, i);
            if let Some(child) = self.inner.at(i) {
                let child_ctx = ctx.with_bounds(bounds);
                if child.handle_text(&child_ctx, info) {
                    return true;
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

    fn clear_focus(&self) {
        for i in 0..self.inner.len() {
            if let Some(child) = self.inner.at(i) {
                child.clear_focus();
            }
        }
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::element::{share, ViewStretch};
    use crate::support::canvas::Canvas;

    /// An element whose reported min-size can be changed at runtime via
    /// `&self` (mirroring `CodeEditor::set_height`/`TreeView::set_width`),
    /// used to reproduce a bug where `VTile`/`HTile::bounds_of` cached
    /// layout keyed only on the *container's* own outer size -- so a
    /// child's own `limits()` changing on its own (exactly what dragging a
    /// `Splitter` does to a sibling) was invisible to the cache and never
    /// took effect.
    struct Resizable {
        min: RwLock<Point>,
        stretch: ViewStretch,
    }
    impl Element for Resizable {
        fn limits(&self, _ctx: &BasicContext) -> ViewLimits {
            let min = *self.min.read().unwrap();
            ViewLimits::min_size(min.x, min.y)
        }
        fn stretch(&self) -> ViewStretch {
            self.stretch
        }
        fn as_any(&self) -> &dyn Any {
            self
        }
        fn as_any_mut(&mut self) -> &mut dyn Any {
            self
        }
    }

    #[test]
    fn vtile_bounds_of_reflects_a_childs_min_size_change_without_a_resize() {
        let a = share(Resizable {
            min: RwLock::new(Point::new(100.0, 90.0)),
            stretch: ViewStretch::new(0.0, 0.0),
        });
        let b = share(Resizable {
            min: RwLock::new(Point::new(100.0, 90.0)),
            stretch: ViewStretch::new(0.0, 0.0),
        });
        let vtile = VTile::from_vec(vec![a.clone(), b.clone()]);

        let view = crate::view::View::new(crate::support::point::Extent::new(200.0, 400.0));
        let canvas = std::cell::RefCell::new(Canvas::new(200, 400).unwrap());
        let bounds = Rect::new(0.0, 0.0, 200.0, 400.0);
        let ctx = Context::new(&view, &canvas, bounds);

        let before = vtile.bounds_of(&ctx, 1);
        assert_eq!(
            before.top, 90.0,
            "second child should start right after the first's 90pt height"
        );

        // Same window/container height as before -- only the first
        // child's own min-height changed, the way `Splitter::on_drag`
        // calling `CodeEditor::set_height` does to a sibling.
        *a.as_any()
            .downcast_ref::<Resizable>()
            .unwrap()
            .min
            .write()
            .unwrap() = Point::new(100.0, 200.0);

        let after = vtile.bounds_of(&ctx, 1);
        assert_eq!(
            after.top, 200.0,
            "second child's position should follow the first child's new height on the very next call, \
             not stay frozen at the old layout"
        );
    }

    #[test]
    fn htile_bounds_of_reflects_a_childs_min_size_change_without_a_resize() {
        let a = share(Resizable {
            min: RwLock::new(Point::new(240.0, 400.0)),
            stretch: ViewStretch::new(0.0, 0.0),
        });
        let b = share(Resizable {
            min: RwLock::new(Point::new(100.0, 400.0)),
            stretch: ViewStretch::new(0.0, 0.0),
        });
        let htile = HTile::from_vec(vec![a.clone(), b.clone()]);

        let view = crate::view::View::new(crate::support::point::Extent::new(900.0, 400.0));
        let canvas = std::cell::RefCell::new(Canvas::new(900, 400).unwrap());
        let bounds = Rect::new(0.0, 0.0, 900.0, 400.0);
        let ctx = Context::new(&view, &canvas, bounds);

        let before = htile.bounds_of(&ctx, 1);
        assert_eq!(before.left, 240.0);

        *a.as_any()
            .downcast_ref::<Resizable>()
            .unwrap()
            .min
            .write()
            .unwrap() = Point::new(300.0, 400.0);

        let after = htile.bounds_of(&ctx, 1);
        assert_eq!(
            after.left, 300.0,
            "second child's position should follow the first (sidebar-like) child's new width \
             immediately, matching what a Splitter drag needs"
        );
    }
}
