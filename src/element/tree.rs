//! A collapsible hierarchical list ("tree view"): expandable/collapsible
//! nodes with indentation and a disclosure arrow, e.g. for a project
//! file-tree panel. Unlike `List` (flat items), nodes can have children
//! that are shown or hidden by clicking their disclosure arrow (or the row
//! itself, for a branch node with no `data`).
//!
//! Self-contained like `List` (built-in vertical scrolling), rather than
//! requiring the caller to wrap it in a `ScrollView`, so a sidebar can use
//! it directly.

use super::context::{BasicContext, Context};
use super::{Element, ViewLimits, ViewStretch};
use crate::support::color::Color;
use crate::support::point::Point;
use crate::support::rect::Rect;
use crate::support::theme::get_theme;
use crate::view::{CursorTracking, MouseButton, MouseButtonKind};
use std::any::Any;
use std::collections::HashSet;
use std::sync::RwLock;

/// One node in a `TreeView`. Leaf vs. branch is purely a function of
/// whether `children` is empty -- there's no separate "is a directory"
/// flag. A branch node with `data` set still fires `on_select` when its
/// label (not its disclosure arrow) is clicked, in addition to toggling
/// expansion; leave `data` unset for pure containers that should only
/// toggle.
#[derive(Debug, Clone)]
pub struct TreeNode {
    pub label: String,
    pub data: Option<String>,
    pub children: Vec<TreeNode>,
}

impl TreeNode {
    /// Creates a leaf node with no children and no associated data.
    pub fn new(label: impl Into<String>) -> Self {
        Self {
            label: label.into(),
            data: None,
            children: Vec::new(),
        }
    }

    /// Attaches an opaque payload (e.g. a file path) delivered to
    /// `on_select` when this node is clicked.
    pub fn with_data(mut self, data: impl Into<String>) -> Self {
        self.data = Some(data.into());
        self
    }

    /// Attaches child nodes, making this a branch with a disclosure arrow.
    pub fn children(mut self, children: Vec<TreeNode>) -> Self {
        self.children = children;
        self
    }
}

/// A flattened, currently-visible row: one entry per node whose every
/// ancestor is expanded. Recomputed by `refresh_rows` whenever `nodes` or
/// `expanded` change shape.
struct Row {
    path: Vec<usize>,
    depth: usize,
    has_children: bool,
    label: String,
    data: Option<String>,
}

/// Callback type fired with a clicked node's `data`.
pub type TreeSelectCallback = Box<dyn Fn(&str) + Send + Sync>;

/// A hierarchical, collapsible list.
pub struct TreeView {
    nodes: RwLock<Vec<TreeNode>>,
    /// Index-path (root-relative) of every currently-expanded branch node.
    expanded: RwLock<HashSet<Vec<usize>>>,
    rows: RwLock<Vec<Row>>,
    selected: RwLock<Option<Vec<usize>>>,
    hovered_row: RwLock<Option<usize>>,
    scroll_offset: RwLock<f32>,
    background_color: Color,
    selected_color: Color,
    hover_color: Color,
    text_color: Color,
    disclosure_color: Color,
    row_height: f32,
    indent: f32,
    width: RwLock<f32>,
    height: f32,
    enabled: bool,
    on_select: Option<TreeSelectCallback>,
}

impl TreeView {
    /// Creates a new, empty tree view.
    pub fn new() -> Self {
        let theme = get_theme();
        Self {
            nodes: RwLock::new(Vec::new()),
            expanded: RwLock::new(HashSet::new()),
            rows: RwLock::new(Vec::new()),
            selected: RwLock::new(None),
            hovered_row: RwLock::new(None),
            scroll_offset: RwLock::new(0.0),
            background_color: theme.input_box_color,
            selected_color: theme.selection_hilite_color,
            hover_color: theme.frame_hilite_color.with_alpha(0.3),
            text_color: theme.label_font_color,
            disclosure_color: theme.label_font_color.with_alpha(0.7),
            row_height: 24.0,
            indent: 14.0,
            width: RwLock::new(200.0),
            height: 400.0,
            enabled: true,
            on_select: None,
        }
    }

    /// Sets the root-level nodes.
    pub fn nodes(self, nodes: Vec<TreeNode>) -> Self {
        *self.nodes.write().unwrap() = nodes;
        self
    }

    /// Replaces the node list at runtime (e.g. the project's file tree
    /// changed on disk), preserving existing expand state for paths that
    /// still exist.
    pub fn set_nodes(&self, nodes: Vec<TreeNode>) {
        *self.nodes.write().unwrap() = nodes;
    }

    /// Sets the view size.
    pub fn size(mut self, width: f32, height: f32) -> Self {
        self.width = RwLock::new(width);
        self.height = height;
        self
    }

    /// Returns the current width (see `set_width`).
    pub fn get_width(&self) -> f32 {
        *self.width.read().unwrap()
    }

    /// Adjusts the width at runtime, e.g. from a `Splitter`'s drag
    /// callback. Clamped to a small minimum so a drag can't collapse the
    /// sidebar to nothing.
    pub fn set_width(&self, width: f32) {
        *self.width.write().unwrap() = width.max(80.0);
    }

    /// Sets the per-level indent, in points.
    pub fn indent(mut self, indent: f32) -> Self {
        self.indent = indent;
        self
    }

    /// Sets the selection callback, fired with a clicked node's `data`
    /// (branch nodes with no `data` set only toggle, never fire this).
    pub fn on_select<F: Fn(&str) + Send + Sync + 'static>(mut self, callback: F) -> Self {
        self.on_select = Some(Box::new(callback));
        self
    }

    /// Expands every ancestor of `path` (but not `path` itself), so a node
    /// reached programmatically -- e.g. "reveal the file that was just
    /// opened" -- is actually visible without the caller needing to expand
    /// each level by hand.
    pub fn reveal(&self, path: &[usize]) {
        let mut expanded = self.expanded.write().unwrap();
        for i in 1..path.len() {
            expanded.insert(path[..i].to_vec());
        }
    }

    fn refresh_rows(&self) {
        fn walk(
            nodes: &[TreeNode],
            path: &mut Vec<usize>,
            expanded: &HashSet<Vec<usize>>,
            depth: usize,
            out: &mut Vec<Row>,
        ) {
            for (i, node) in nodes.iter().enumerate() {
                path.push(i);
                out.push(Row {
                    path: path.clone(),
                    depth,
                    has_children: !node.children.is_empty(),
                    label: node.label.clone(),
                    data: node.data.clone(),
                });
                if !node.children.is_empty() && expanded.contains(path) {
                    walk(&node.children, path, expanded, depth + 1, out);
                }
                path.pop();
            }
        }

        let nodes = self.nodes.read().unwrap();
        let expanded = self.expanded.read().unwrap();
        let mut rows = Vec::new();
        let mut path = Vec::new();
        walk(&nodes, &mut path, &expanded, 0, &mut rows);
        drop(nodes);
        drop(expanded);
        *self.rows.write().unwrap() = rows;
    }

    fn total_content_height(&self) -> f32 {
        self.rows.read().unwrap().len() as f32 * self.row_height + 8.0
    }

    fn row_rect(&self, ctx: &Context, index: usize) -> Rect {
        let scroll = *self.scroll_offset.read().unwrap();
        let y = ctx.bounds.top + 4.0 + index as f32 * self.row_height - scroll;
        Rect::new(ctx.bounds.left, y, ctx.bounds.right, y + self.row_height)
    }

    fn row_at_point(&self, ctx: &Context, p: Point) -> Option<usize> {
        let count = self.rows.read().unwrap().len();
        for i in 0..count {
            let rect = self.row_rect(ctx, i);
            if rect.contains(p) && rect.top >= ctx.bounds.top && rect.bottom <= ctx.bounds.bottom {
                return Some(i);
            }
        }
        None
    }

    fn disclosure_rect(&self, row_rect: Rect, depth: usize) -> Rect {
        let size = self.row_height * 0.5;
        let x = row_rect.left + 4.0 + depth as f32 * self.indent;
        let y = row_rect.center().y - size / 2.0;
        Rect::new(x, y, x + size, y + size)
    }

    fn draw_background(&self, ctx: &Context) {
        let mut canvas = ctx.canvas.borrow_mut();
        canvas.fill_style(self.background_color);
        canvas.fill_rect(ctx.bounds);
    }

    fn draw_rows(&self, ctx: &Context) {
        let rows = self.rows.read().unwrap();
        let selected = self.selected.read().unwrap();
        let hovered = *self.hovered_row.read().unwrap();
        let theme = get_theme();
        let mut canvas = ctx.canvas.borrow_mut();

        for (i, row) in rows.iter().enumerate() {
            let rect = self.row_rect(ctx, i);
            if rect.bottom < ctx.bounds.top || rect.top > ctx.bounds.bottom {
                continue;
            }

            let is_selected = selected.as_deref() == Some(row.path.as_slice());
            let is_hovered = hovered == Some(i) && !is_selected;

            if is_selected {
                canvas.fill_style(self.selected_color);
                canvas.fill_round_rect(rect, 3.0);
            } else if is_hovered {
                canvas.fill_style(self.hover_color);
                canvas.fill_round_rect(rect, 3.0);
            }

            if row.has_children {
                let expanded = self.expanded.read().unwrap().contains(&row.path);
                let disclosure = self.disclosure_rect(rect, row.depth);
                canvas.fill_style(self.disclosure_color);
                canvas.font_size(theme.label_font_size * 0.8);
                let glyph = if expanded { "\u{25be}" } else { "\u{25b8}" };
                canvas.fill_text(
                    glyph,
                    Point::new(
                        disclosure.left,
                        disclosure.center().y + theme.label_font_size * 0.28,
                    ),
                );
            }

            let text_color = if !self.enabled {
                self.text_color.with_alpha(0.5)
            } else {
                self.text_color
            };
            canvas.fill_style(text_color);
            canvas.font_size(theme.label_font_size);

            let text_x = rect.left + 4.0 + row.depth as f32 * self.indent + self.row_height * 0.5;
            let text_y = rect.center().y + theme.label_font_size * 0.35;
            canvas.fill_text(&row.label, Point::new(text_x, text_y));
        }
    }

    fn draw_scrollbar(&self, ctx: &Context) {
        let total_height = self.total_content_height();
        let visible_height = ctx.bounds.height();
        if total_height <= visible_height {
            return;
        }

        let theme = get_theme();
        let scroll = *self.scroll_offset.read().unwrap();
        let scrollbar_height = (visible_height / total_height * visible_height).max(20.0);
        let scrollbar_y =
            scroll / (total_height - visible_height) * (visible_height - scrollbar_height);

        let rect = Rect::new(
            ctx.bounds.right - 8.0,
            ctx.bounds.top + scrollbar_y,
            ctx.bounds.right - 2.0,
            ctx.bounds.top + scrollbar_y + scrollbar_height,
        );
        let mut canvas = ctx.canvas.borrow_mut();
        canvas.fill_style(theme.scrollbar_color);
        canvas.fill_round_rect(rect, 3.0);
    }

    /// Toggles expansion for the branch node at `path`.
    fn toggle(&self, path: &[usize]) {
        let mut expanded = self.expanded.write().unwrap();
        if !expanded.remove(path) {
            expanded.insert(path.to_vec());
        }
    }
}

impl Default for TreeView {
    fn default() -> Self {
        Self::new()
    }
}

impl Element for TreeView {
    fn limits(&self, _ctx: &BasicContext) -> ViewLimits {
        // `min_size`, not `fixed` -- see `CodeEditor::limits` for why: a
        // sidebar tree should stay at least this size, not be capped at
        // exactly this size forever regardless of how much room its
        // container actually has.
        ViewLimits::min_size(*self.width.read().unwrap(), self.height)
    }

    fn stretch(&self) -> ViewStretch {
        // No horizontal stretch: a sidebar tree should stay close to its
        // constructed width and let a sibling with an actual claim on
        // extra space (e.g. an editor) take it, not split it 50/50 just
        // because both have a non-zero `max`. Vertical stretch stays on so
        // it still fills the height of whatever column/row it's in.
        ViewStretch::new(0.0, 1.0)
    }

    fn draw(&self, ctx: &Context) {
        self.refresh_rows();
        self.draw_background(ctx);

        {
            let mut canvas = ctx.canvas.borrow_mut();
            canvas.save();
            canvas.clip(ctx.bounds);
        }
        self.draw_rows(ctx);
        ctx.canvas.borrow_mut().restore();

        self.draw_scrollbar(ctx);
    }

    fn hit_test(
        &self,
        ctx: &Context,
        p: Point,
        _leaf: bool,
        _control: bool,
    ) -> Option<&dyn Element> {
        if ctx.bounds.contains(p) && self.enabled {
            Some(self)
        } else {
            None
        }
    }

    fn wants_control(&self) -> bool {
        self.enabled
    }

    fn handle_click(&self, ctx: &Context, btn: MouseButton) -> bool {
        if !self.enabled || btn.button != MouseButtonKind::Left || !btn.down {
            return btn.button == MouseButtonKind::Left;
        }

        let Some(index) = self.row_at_point(ctx, btn.pos) else {
            return true;
        };

        let (path, has_children, data) = {
            let rows = self.rows.read().unwrap();
            let Some(row) = rows.get(index) else {
                return true;
            };
            (row.path.clone(), row.has_children, row.data.clone())
        };

        if has_children {
            let rect = self.row_rect(ctx, index);
            let disclosure = self.disclosure_rect(rect, self.rows.read().unwrap()[index].depth);
            if disclosure.contains(btn.pos) || data.is_none() {
                self.toggle(&path);
                self.refresh_rows();
                return true;
            }
        }

        if let Some(data) = data {
            *self.selected.write().unwrap() = Some(path);
            if let Some(ref callback) = self.on_select {
                callback(&data);
            }
        }

        true
    }

    fn handle_scroll(&self, ctx: &Context, dir: Point, _p: Point) -> bool {
        if !self.enabled {
            return false;
        }
        let total_height = self.total_content_height();
        let visible_height = ctx.bounds.height();
        if total_height <= visible_height {
            return false;
        }
        let mut scroll = self.scroll_offset.write().unwrap();
        *scroll = (*scroll - dir.y * 20.0).clamp(0.0, total_height - visible_height);
        true
    }

    fn cursor(&mut self, ctx: &Context, p: Point, status: CursorTracking) -> bool {
        if !self.enabled {
            return false;
        }
        match status {
            CursorTracking::Leaving => {
                *self.hovered_row.write().unwrap() = None;
            }
            _ => {
                *self.hovered_row.write().unwrap() = self.row_at_point(ctx, p);
            }
        }
        true
    }

    fn enable(&mut self, state: bool) {
        self.enabled = state;
    }

    fn is_enabled(&self) -> bool {
        self.enabled
    }

    fn as_any(&self) -> &dyn Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }
}

/// Creates a tree view.
pub fn tree_view() -> TreeView {
    TreeView::new()
}

/// Creates a tree node.
pub fn tree_node(label: impl Into<String>) -> TreeNode {
    TreeNode::new(label)
}
