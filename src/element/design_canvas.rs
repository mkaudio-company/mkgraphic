//! A free-form container for visually laying out other elements: absolute
//! (not flow-based) positioning, click-to-select, drag-to-move,
//! drag-to-resize via corner/edge handles, and snap/alignment guides against
//! sibling edges while dragging.
//!
//! Modeled directly on [`super::floating::Floating`] (same
//! position/size-as-`RwLock<Point>` + drag-offset shape for a single free-
//! positioned child), extended to hold many children plus one "selected"
//! child with resize handles. Deliberately has no mkapk/plugin-specific
//! concepts -- it's a general MKGraphic primitive; MKIDE's plugin-UI
//! designer (a separate crate) builds the component-palette/parameter-
//! binding/codegen layer on top of this.

use std::any::Any;
use std::sync::RwLock;

use super::context::{BasicContext, Context};
use super::{share, Element, ElementPtr, ViewLimits, ViewStretch};
use crate::support::color::Color;
use crate::support::point::Point;
use crate::support::rect::Rect;
use crate::support::theme::get_theme;
use crate::view::{MouseButton, MouseButtonKind};

/// One child positioned on the canvas.
struct CanvasChild {
    content: ElementPtr,
    rect: RwLock<Rect>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum DragKind {
    Move,
    ResizeLeft,
    ResizeRight,
    ResizeTop,
    ResizeBottom,
    ResizeTopLeft,
    ResizeTopRight,
    ResizeBottomLeft,
    ResizeBottomRight,
}

#[derive(Debug, Clone, Copy)]
struct DragState {
    child_index: usize,
    kind: DragKind,
    start_pointer: Point,
    start_rect: Rect,
}

/// A snapped alignment guide to draw: `Vertical(x)` spans the canvas height
/// at `x`; `Horizontal(y)` spans the canvas width at `y`.
#[derive(Debug, Clone, Copy)]
enum Guide {
    Vertical(f32),
    Horizontal(f32),
}

type SelectionCallback = Box<dyn Fn(Option<usize>) + Send + Sync>;
type LayoutChangeCallback = Box<dyn Fn() + Send + Sync>;

/// A design surface: children positioned by absolute rect, with selection,
/// drag-move, drag-resize, and snap guides.
pub struct DesignCanvas {
    // Plain field (not RwLock<Vec<_>>), matching `Composite`'s convention:
    // structural changes (add/remove) need `&mut self`, so `hit_test`'s
    // `&self`-elided return of `&dyn Element` can borrow straight through
    // `self.children[i].content` without a lock guard whose lifetime would
    // be shorter than `&self`'s. Each child's own `rect` is still an
    // interior-mutable `RwLock` (see `CanvasChild`), since drag/resize only
    // ever need to update that, not the Vec's shape.
    children: Vec<CanvasChild>,
    selected: RwLock<Option<usize>>,
    drag: RwLock<Option<DragState>>,
    guides: RwLock<Vec<Guide>>,
    background_color: Color,
    selection_color: Color,
    guide_color: Color,
    handle_size: f32,
    snap_threshold: f32,
    width: f32,
    height: f32,
    on_selection_changed: Option<SelectionCallback>,
    on_layout_changed: Option<LayoutChangeCallback>,
}

impl DesignCanvas {
    /// Creates a new, empty design canvas of the given size.
    pub fn new(width: f32, height: f32) -> Self {
        let theme = get_theme();
        Self {
            children: Vec::new(),
            selected: RwLock::new(None),
            drag: RwLock::new(None),
            guides: RwLock::new(Vec::new()),
            background_color: theme.element_background_color,
            selection_color: theme.frame_hilite_color,
            guide_color: Color::from_rgb_u8(255, 100, 100),
            handle_size: 8.0,
            snap_threshold: 6.0,
            width,
            height,
            on_selection_changed: None,
            on_layout_changed: None,
        }
    }

    /// Sets the callback invoked (with the newly selected child index, or
    /// `None` if selection was cleared) whenever selection changes.
    pub fn on_selection_changed<F: Fn(Option<usize>) + Send + Sync + 'static>(
        mut self,
        callback: F,
    ) -> Self {
        self.on_selection_changed = Some(Box::new(callback));
        self
    }

    /// Sets the callback invoked after any child is moved or resized.
    pub fn on_layout_changed<F: Fn() + Send + Sync + 'static>(mut self, callback: F) -> Self {
        self.on_layout_changed = Some(Box::new(callback));
        self
    }

    /// Adds a child at `rect`, returning its index (stable for the child's
    /// lifetime; used by [`Self::child_rect`]/[`Self::set_child_rect`]/
    /// [`Self::remove_child`]). Build up a canvas's children before sharing
    /// it (e.g. via [`share`]) -- structural changes need `&mut self`, while
    /// every other interaction (drag, select, draw, hit-test) only needs
    /// `&self` and works fine behind `Arc`.
    pub fn add_child<E: Element + 'static>(&mut self, content: E, rect: Rect) -> usize {
        self.children.push(CanvasChild {
            content: share(content),
            rect: RwLock::new(rect),
        });
        self.children.len() - 1
    }

    /// Removes the child at `index`. Indices of children after it shift down
    /// by one; re-fetch any index you were holding onto after calling this.
    pub fn remove_child(&mut self, index: usize) {
        if index < self.children.len() {
            self.children.remove(index);
        }
        let mut selected = self.selected.write().unwrap();
        if *selected == Some(index) {
            *selected = None;
        }
    }

    /// Returns the number of children currently on the canvas.
    pub fn child_count(&self) -> usize {
        self.children.len()
    }

    /// Returns child `index`'s current rect, if it exists.
    pub fn child_rect(&self, index: usize) -> Option<Rect> {
        self.children.get(index).map(|c| *c.rect.read().unwrap())
    }

    /// Sets child `index`'s rect directly (e.g. from a property-inspector
    /// panel, rather than a drag gesture).
    pub fn set_child_rect(&self, index: usize, rect: Rect) {
        if let Some(child) = self.children.get(index) {
            *child.rect.write().unwrap() = rect;
        }
    }

    /// Returns the currently selected child's index, if any.
    pub fn selected_index(&self) -> Option<usize> {
        *self.selected.read().unwrap()
    }

    /// Selects child `index` (or clears selection if `None`), invoking
    /// [`Self::on_selection_changed`] if it changed.
    pub fn select(&self, index: Option<usize>) {
        let mut selected = self.selected.write().unwrap();
        if *selected != index {
            *selected = index;
            drop(selected);
            if let Some(ref cb) = self.on_selection_changed {
                cb(index);
            }
        }
    }

    fn canvas_bounds(&self, ctx: &Context) -> Rect {
        Rect::new(
            ctx.bounds.left,
            ctx.bounds.top,
            ctx.bounds.left + self.width,
            ctx.bounds.top + self.height,
        )
    }

    /// Hit-tests children topmost-first (last added = drawn last = on top).
    fn hit_child(&self, _ctx: &Context, p: Point) -> Option<usize> {
        for (i, child) in self.children.iter().enumerate().rev() {
            if child.rect.read().unwrap().contains(p) {
                return Some(i);
            }
        }
        None
    }

    fn handle_rects(&self, rect: Rect) -> [(DragKind, Rect); 8] {
        let h = self.handle_size;
        let half = h / 2.0;
        let mk = |cx: f32, cy: f32| Rect::new(cx - half, cy - half, cx + half, cy + half);
        [
            (DragKind::ResizeTopLeft, mk(rect.left, rect.top)),
            (DragKind::ResizeTop, mk(rect.center().x, rect.top)),
            (DragKind::ResizeTopRight, mk(rect.right, rect.top)),
            (DragKind::ResizeLeft, mk(rect.left, rect.center().y)),
            (DragKind::ResizeRight, mk(rect.right, rect.center().y)),
            (DragKind::ResizeBottomLeft, mk(rect.left, rect.bottom)),
            (DragKind::ResizeBottom, mk(rect.center().x, rect.bottom)),
            (DragKind::ResizeBottomRight, mk(rect.right, rect.bottom)),
        ]
    }

    /// Applies a drag gesture at `pointer` to `state.start_rect`, snapping
    /// the moved/resized edges against sibling edges within
    /// `snap_threshold`, and returns the resulting rect plus any guides that
    /// were snapped to (for [`Self::draw`] to render).
    fn apply_drag(&self, state: &DragState, pointer: Point) -> (Rect, Vec<Guide>) {
        let dx = pointer.x - state.start_pointer.x;
        let dy = pointer.y - state.start_pointer.y;
        let r = state.start_rect;

        let mut rect = match state.kind {
            DragKind::Move => r.translate(dx, dy),
            DragKind::ResizeLeft => Rect::new(r.left + dx, r.top, r.right, r.bottom),
            DragKind::ResizeRight => Rect::new(r.left, r.top, r.right + dx, r.bottom),
            DragKind::ResizeTop => Rect::new(r.left, r.top + dy, r.right, r.bottom),
            DragKind::ResizeBottom => Rect::new(r.left, r.top, r.right, r.bottom + dy),
            DragKind::ResizeTopLeft => Rect::new(r.left + dx, r.top + dy, r.right, r.bottom),
            DragKind::ResizeTopRight => Rect::new(r.left, r.top + dy, r.right + dx, r.bottom),
            DragKind::ResizeBottomLeft => Rect::new(r.left + dx, r.top, r.right, r.bottom + dy),
            DragKind::ResizeBottomRight => Rect::new(r.left, r.top, r.right + dx, r.bottom + dy),
        };

        let mut guides = Vec::new();
        let siblings: Vec<Rect> = self
            .children
            .iter()
            .enumerate()
            .filter(|(i, _)| *i != state.child_index)
            .map(|(_, c)| *c.rect.read().unwrap())
            .collect();

        for sib in &siblings {
            for &edge in &[sib.left, sib.right] {
                if (rect.left - edge).abs() <= self.snap_threshold {
                    let shift = edge - rect.left;
                    rect = rect.translate(shift, 0.0);
                    guides.push(Guide::Vertical(edge));
                } else if (rect.right - edge).abs() <= self.snap_threshold {
                    let shift = edge - rect.right;
                    rect = rect.translate(shift, 0.0);
                    guides.push(Guide::Vertical(edge));
                }
            }
            for &edge in &[sib.top, sib.bottom] {
                if (rect.top - edge).abs() <= self.snap_threshold {
                    let shift = edge - rect.top;
                    rect = rect.translate(0.0, shift);
                    guides.push(Guide::Horizontal(edge));
                } else if (rect.bottom - edge).abs() <= self.snap_threshold {
                    let shift = edge - rect.bottom;
                    rect = rect.translate(0.0, shift);
                    guides.push(Guide::Horizontal(edge));
                }
            }
        }

        (rect, guides)
    }
}

impl Element for DesignCanvas {
    fn limits(&self, _ctx: &BasicContext) -> ViewLimits {
        ViewLimits::fixed(self.width, self.height)
    }

    fn stretch(&self) -> ViewStretch {
        ViewStretch::new(0.0, 0.0)
    }

    fn draw(&self, ctx: &Context) {
        let bounds = self.canvas_bounds(ctx);
        {
            let mut canvas = ctx.canvas.borrow_mut();
            canvas.fill_style(self.background_color);
            canvas.fill_rect(bounds);
        }

        for child in self.children.iter() {
            let rect = *child.rect.read().unwrap();
            let child_ctx = ctx.with_bounds(rect);
            child.content.draw(&child_ctx);
        }

        if let Some(index) = self.selected_index() {
            if let Some(rect) = self.child_rect(index) {
                let mut canvas = ctx.canvas.borrow_mut();
                canvas.stroke_style(self.selection_color);
                canvas.line_width(1.5);
                canvas.begin_path();
                canvas.add_rect(rect);
                canvas.stroke();
                drop(canvas);

                let mut canvas = ctx.canvas.borrow_mut();
                canvas.fill_style(self.selection_color);
                for (_, handle_rect) in self.handle_rects(rect) {
                    canvas.fill_rect(handle_rect);
                }
            }
        }

        let guides = self.guides.read().unwrap();
        if !guides.is_empty() {
            let mut canvas = ctx.canvas.borrow_mut();
            canvas.stroke_style(self.guide_color);
            canvas.line_width(1.0);
            for guide in guides.iter() {
                canvas.begin_path();
                match *guide {
                    Guide::Vertical(x) => {
                        canvas.move_to(Point::new(x, bounds.top));
                        canvas.line_to(Point::new(x, bounds.bottom));
                    }
                    Guide::Horizontal(y) => {
                        canvas.move_to(Point::new(bounds.left, y));
                        canvas.line_to(Point::new(bounds.right, y));
                    }
                }
                canvas.stroke();
            }
        }
    }

    fn hit_test(&self, ctx: &Context, p: Point, leaf: bool, control: bool) -> Option<&dyn Element> {
        let bounds = self.canvas_bounds(ctx);
        if !bounds.contains(p) {
            return None;
        }
        if let Some(index) = self.hit_child(ctx, p) {
            if let Some(child_rect) = self.child_rect(index) {
                let child_ctx = ctx.with_bounds(child_rect);
                if let Some(hit) =
                    self.children[index].content.hit_test(&child_ctx, p, leaf, control)
                {
                    return Some(hit);
                }
            }
        }
        Some(self)
    }

    fn wants_control(&self) -> bool {
        true
    }

    fn handle_click(&self, ctx: &Context, btn: MouseButton) -> bool {
        if btn.button != MouseButtonKind::Left {
            return false;
        }

        if !btn.down {
            // Mouse-up: finalize the drag (already live-applied in
            // handle_drag) and notify listeners.
            let had_drag = self.drag.write().unwrap().take().is_some();
            self.guides.write().unwrap().clear();
            if had_drag {
                if let Some(ref cb) = self.on_layout_changed {
                    cb();
                }
            }
            return true;
        }

        // Resize handle on the already-selected child takes priority over
        // re-selecting/moving.
        if let Some(index) = self.selected_index() {
            if let Some(rect) = self.child_rect(index) {
                for (kind, handle_rect) in self.handle_rects(rect) {
                    if handle_rect.contains(btn.pos) {
                        *self.drag.write().unwrap() = Some(DragState {
                            child_index: index,
                            kind,
                            start_pointer: btn.pos,
                            start_rect: rect,
                        });
                        return true;
                    }
                }
            }
        }

        match self.hit_child(ctx, btn.pos) {
            Some(index) => {
                self.select(Some(index));
                let rect = self.child_rect(index).unwrap();
                *self.drag.write().unwrap() = Some(DragState {
                    child_index: index,
                    kind: DragKind::Move,
                    start_pointer: btn.pos,
                    start_rect: rect,
                });
            }
            None => self.select(None),
        }
        true
    }

    fn drag(&mut self, ctx: &Context, btn: MouseButton) {
        self.handle_drag(ctx, btn);
    }

    fn handle_drag(&self, _ctx: &Context, btn: MouseButton) {
        let Some(state) = *self.drag.read().unwrap() else {
            return;
        };
        let (rect, guides) = self.apply_drag(&state, btn.pos);
        self.set_child_rect(state.child_index, rect);
        *self.guides.write().unwrap() = guides;
    }

    fn as_any(&self) -> &dyn Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }
}

/// Creates a design canvas of the given size.
pub fn design_canvas(width: f32, height: f32) -> DesignCanvas {
    DesignCanvas::new(width, height)
}
