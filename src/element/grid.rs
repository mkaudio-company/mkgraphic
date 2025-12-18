//! Grid layout element.

use std::any::Any;
use std::sync::RwLock;
use super::{Element, ElementPtr, ViewLimits, ViewStretch, FULL_EXTENT};
use super::context::{BasicContext, Context};
use super::composite::{Storage, CompositeBase, Composite};
use crate::support::point::Point;
use crate::support::rect::Rect;

/// A grid layout element that arranges children in rows and columns.
pub struct Grid {
    inner: Composite,
    columns: usize,
    row_heights: RwLock<Vec<f32>>,
    col_widths: RwLock<Vec<f32>>,
    h_gap: f32,
    v_gap: f32,
}

impl Grid {
    /// Creates a new grid with the specified number of columns.
    pub fn new(columns: usize) -> Self {
        Self {
            inner: Composite::new(),
            columns: columns.max(1),
            row_heights: RwLock::new(Vec::new()),
            col_widths: RwLock::new(Vec::new()),
            h_gap: 4.0,
            v_gap: 4.0,
        }
    }

    /// Creates a grid from a vector of elements.
    pub fn from_vec(columns: usize, children: Vec<ElementPtr>) -> Self {
        Self {
            inner: Composite::from_vec(children),
            columns: columns.max(1),
            row_heights: RwLock::new(Vec::new()),
            col_widths: RwLock::new(Vec::new()),
            h_gap: 4.0,
            v_gap: 4.0,
        }
    }

    /// Sets the horizontal gap between columns.
    pub fn h_gap(mut self, gap: f32) -> Self {
        self.h_gap = gap;
        self
    }

    /// Sets the vertical gap between rows.
    pub fn v_gap(mut self, gap: f32) -> Self {
        self.v_gap = gap;
        self
    }

    /// Sets both gaps.
    pub fn gap(mut self, gap: f32) -> Self {
        self.h_gap = gap;
        self.v_gap = gap;
        self
    }

    /// Adds an element.
    pub fn push(&mut self, element: ElementPtr) {
        self.inner.push(element);
    }

    /// Returns the number of rows.
    fn row_count(&self) -> usize {
        let count = self.inner.len();
        (count + self.columns - 1) / self.columns
    }

    fn compute_layout(&self, ctx: &BasicContext, bounds: &Rect) {
        let count = self.inner.len();
        if count == 0 {
            return;
        }

        let rows = self.row_count();

        // Calculate minimum sizes for each row and column
        let mut col_min_widths = vec![0.0f32; self.columns];
        let mut col_stretches = vec![0.0f32; self.columns];
        let mut row_min_heights = vec![0.0f32; rows];
        let mut row_stretches = vec![0.0f32; rows];

        for i in 0..count {
            let row = i / self.columns;
            let col = i % self.columns;

            if let Some(child) = self.inner.at(i) {
                let limits = child.limits(ctx);
                let stretch = child.stretch();

                col_min_widths[col] = col_min_widths[col].max(limits.min.x);
                col_stretches[col] = col_stretches[col].max(stretch.x);
                row_min_heights[row] = row_min_heights[row].max(limits.min.y);
                row_stretches[row] = row_stretches[row].max(stretch.y);
            }
        }

        // Calculate total minimum sizes
        let total_min_width: f32 = col_min_widths.iter().sum::<f32>() + self.h_gap * (self.columns - 1) as f32;
        let total_min_height: f32 = row_min_heights.iter().sum::<f32>() + self.v_gap * (rows - 1) as f32;

        // Calculate extra space
        let extra_width = (bounds.width() - total_min_width).max(0.0);
        let extra_height = (bounds.height() - total_min_height).max(0.0);

        let total_col_stretch: f32 = col_stretches.iter().sum();
        let total_row_stretch: f32 = row_stretches.iter().sum();

        // Distribute extra space
        let mut col_widths = col_min_widths.clone();
        let mut row_heights = row_min_heights.clone();

        if total_col_stretch > 0.0 {
            for (i, stretch) in col_stretches.iter().enumerate() {
                col_widths[i] += extra_width * (stretch / total_col_stretch);
            }
        }

        if total_row_stretch > 0.0 {
            for (i, stretch) in row_stretches.iter().enumerate() {
                row_heights[i] += extra_height * (stretch / total_row_stretch);
            }
        }

        *self.col_widths.write().unwrap() = col_widths;
        *self.row_heights.write().unwrap() = row_heights;
    }
}

impl Default for Grid {
    fn default() -> Self {
        Self::new(2)
    }
}

impl Storage for Grid {
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

impl CompositeBase for Grid {
    fn bounds_of(&self, ctx: &Context, index: usize) -> Rect {
        let count = self.inner.len();
        if index >= count {
            return Rect::zero();
        }

        // Ensure layout is computed
        {
            let col_widths = self.col_widths.read().unwrap();
            if col_widths.is_empty() || col_widths.len() != self.columns {
                drop(col_widths);
                let basic_ctx = BasicContext::new(ctx.view, ctx.canvas);
                self.compute_layout(&basic_ctx, &ctx.bounds);
            }
        }

        let col_widths = self.col_widths.read().unwrap();
        let row_heights = self.row_heights.read().unwrap();

        let row = index / self.columns;
        let col = index % self.columns;

        let mut x = ctx.bounds.left;
        for i in 0..col {
            x += col_widths.get(i).copied().unwrap_or(0.0) + self.h_gap;
        }

        let mut y = ctx.bounds.top;
        for i in 0..row {
            y += row_heights.get(i).copied().unwrap_or(0.0) + self.v_gap;
        }

        let width = col_widths.get(col).copied().unwrap_or(0.0);
        let height = row_heights.get(row).copied().unwrap_or(0.0);

        Rect::new(x, y, x + width, y + height)
    }
}

impl Element for Grid {
    fn limits(&self, ctx: &BasicContext) -> ViewLimits {
        let count = self.inner.len();
        if count == 0 {
            return ViewLimits::fixed(0.0, 0.0);
        }

        let rows = self.row_count();

        let mut col_min_widths = vec![0.0f32; self.columns];
        let mut col_max_widths = vec![0.0f32; self.columns];
        let mut row_min_heights = vec![0.0f32; rows];
        let mut row_max_heights = vec![0.0f32; rows];

        for i in 0..count {
            let row = i / self.columns;
            let col = i % self.columns;

            if let Some(child) = self.inner.at(i) {
                let limits = child.limits(ctx);
                col_min_widths[col] = col_min_widths[col].max(limits.min.x);
                col_max_widths[col] = col_max_widths[col].max(limits.max.x);
                row_min_heights[row] = row_min_heights[row].max(limits.min.y);
                row_max_heights[row] = row_max_heights[row].max(limits.max.y);
            }
        }

        let total_min_width: f32 = col_min_widths.iter().sum::<f32>() + self.h_gap * (self.columns.saturating_sub(1)) as f32;
        let total_max_width: f32 = col_max_widths.iter().sum::<f32>() + self.h_gap * (self.columns.saturating_sub(1)) as f32;
        let total_min_height: f32 = row_min_heights.iter().sum::<f32>() + self.v_gap * rows.saturating_sub(1) as f32;
        let total_max_height: f32 = row_max_heights.iter().sum::<f32>() + self.v_gap * rows.saturating_sub(1) as f32;

        ViewLimits {
            min: Point::new(total_min_width, total_min_height),
            max: Point::new(
                total_max_width.min(FULL_EXTENT),
                total_max_height.min(FULL_EXTENT),
            ),
        }
    }

    fn stretch(&self) -> ViewStretch {
        // Grid stretches if any child stretches
        let mut x_stretch = 0.0f32;
        let mut y_stretch = 0.0f32;

        for i in 0..self.inner.len() {
            if let Some(child) = self.inner.at(i) {
                let stretch = child.stretch();
                x_stretch = x_stretch.max(stretch.x);
                y_stretch = y_stretch.max(stretch.y);
            }
        }

        ViewStretch::new(x_stretch, y_stretch)
    }

    fn draw(&self, ctx: &Context) {
        // Ensure layout is computed
        {
            let col_widths = self.col_widths.read().unwrap();
            if col_widths.is_empty() {
                drop(col_widths);
                let basic_ctx = BasicContext::new(ctx.view, ctx.canvas);
                self.compute_layout(&basic_ctx, &ctx.bounds);
            }
        }

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

    fn as_any(&self) -> &dyn Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }
}

/// Creates a grid with the specified number of columns.
pub fn grid(columns: usize) -> Grid {
    Grid::new(columns)
}

/// Macro for creating grids.
#[macro_export]
macro_rules! grid {
    ($cols:expr; $($elem:expr),* $(,)?) => {{
        let mut g = $crate::element::grid::Grid::new($cols);
        $(
            g.push($crate::element::share($elem));
        )*
        g
    }};
}
