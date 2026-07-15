//! A scrollable, read-only rendering of a full Markdown document -- the
//! standalone viewer this workspace's Markdown support
//! (`crate::support::markdown`) was always meant to grow into once there
//! was a second consumer beyond `ChatHistory`'s message bubbles. Built on
//! the exact same `markdown_to_runs`/`wrap_runs`/`draw_runs` pipeline,
//! generalized from "one bubble's text" to "one flowing document": no
//! bubble background, no sender alignment, just left-aligned wrapped text
//! filling the element's width. Scroll/clip/draw structure mirrors
//! `ChatHistory`'s own (`src/element/chat_history.rs`) -- recomputes
//! layout on every call rather than caching against content/width, for
//! the same reasoning documented there (a cache keyed on the wrong
//! invalidation condition is a real, already-hit bug class in this
//! codebase).
//!
//! Display-only, like `ChatHistory`/`List`/`Dropdown` -- no text
//! selection/copy of the rendered content.

use super::context::{BasicContext, Context};
use super::{Element, ViewLimits, ViewStretch};
use crate::support::color::Color;
use crate::support::markdown::{self, StyledRun};
use crate::support::point::Point;
use crate::support::rect::Rect;
use crate::support::theme::get_theme;
use std::any::Any;
use std::sync::RwLock;

/// A scrollable Markdown document viewer.
pub struct MarkdownView {
    text: RwLock<String>,
    scroll_offset: RwLock<f32>,
    width: f32,
    height: f32,
    font_size: f32,
    padding: f32,
    background_color: Color,
    text_color: Color,
    enabled: bool,
}

impl MarkdownView {
    /// Creates a new, empty Markdown view.
    pub fn new() -> Self {
        let theme = get_theme();
        Self {
            text: RwLock::new(String::new()),
            scroll_offset: RwLock::new(0.0),
            width: 400.0,
            height: 300.0,
            font_size: theme.label_font_size,
            padding: 12.0,
            background_color: theme.input_box_color,
            text_color: theme.label_font_color,
            enabled: true,
        }
    }

    /// Sets the initial text (builder form).
    pub fn text(self, text: impl Into<String>) -> Self {
        *self.text.write().unwrap() = text.into();
        self
    }

    /// Sets the width.
    pub fn width(mut self, width: f32) -> Self {
        self.width = width;
        self
    }

    /// Sets the height.
    pub fn height(mut self, height: f32) -> Self {
        self.height = height;
        self
    }

    /// Replaces the rendered text -- for a live preview, called on every
    /// source edit. Deliberately does *not* reset scroll: jumping back to
    /// the top on every keystroke while the user is reading further down
    /// would be actively hostile, and `layout`'s own clamp already keeps
    /// `scroll_offset` valid if the content shrinks.
    pub fn set_text(&self, text: impl Into<String>) {
        *self.text.write().unwrap() = text.into();
    }

    fn line_height(&self) -> f32 {
        self.font_size * 1.3
    }

    fn max_text_width(&self) -> f32 {
        (self.width - 2.0 * self.padding).max(20.0)
    }

    /// Markdown-parses and word-wraps the current text, returning
    /// (wrapped lines, total content height) in local (unscrolled, un-
    /// translated) coordinates. As a side effect, self-corrects
    /// `scroll_offset` against the fresh height -- same convention as
    /// `ChatHistory::layout_messages`.
    fn layout(&self, ctx: &Context) -> (Vec<Vec<StyledRun>>, f32) {
        let text = self.text.read().unwrap().clone();
        let line_height = self.line_height();
        let max_width = self.max_text_width();

        let lines = {
            let mut canvas = ctx.canvas.borrow_mut();
            canvas.font_size(self.font_size);
            if text.is_empty() {
                Vec::new()
            } else {
                let runs = markdown::markdown_to_runs(&text);
                markdown::wrap_runs(&mut canvas, &runs, max_width)
            }
        };

        let total_height = lines.len() as f32 * line_height + 2.0 * self.padding;

        let visible_height = ctx.bounds.height();
        let mut scroll = self.scroll_offset.write().unwrap();
        *scroll = if total_height <= visible_height {
            0.0
        } else {
            (*scroll).min(total_height - visible_height).max(0.0)
        };

        (lines, total_height)
    }

    fn draw_background(&self, ctx: &Context) {
        let mut canvas = ctx.canvas.borrow_mut();
        canvas.fill_style(self.background_color);
        canvas.fill_rect(ctx.bounds);
    }

    fn draw_content(&self, ctx: &Context, lines: &[Vec<StyledRun>], scroll: f32) {
        let mut canvas = ctx.canvas.borrow_mut();
        canvas.font_size(self.font_size);
        let line_height = self.line_height();

        let origin = Point::new(ctx.bounds.left + self.padding, ctx.bounds.top + self.padding - scroll + self.font_size * 0.85);
        markdown::draw_runs(&mut canvas, lines, origin, line_height, self.text_color);
    }

    fn draw_scrollbar(&self, ctx: &Context, total_height: f32, visible_height: f32) {
        if total_height <= visible_height {
            return;
        }

        let theme = get_theme();
        let scroll = *self.scroll_offset.read().unwrap();

        let scrollbar_height = (visible_height / total_height * visible_height).max(20.0);
        let scrollbar_y =
            scroll / (total_height - visible_height) * (visible_height - scrollbar_height);

        let scrollbar_rect = Rect::new(
            ctx.bounds.right - 8.0,
            ctx.bounds.top + scrollbar_y,
            ctx.bounds.right - 2.0,
            ctx.bounds.top + scrollbar_y + scrollbar_height,
        );

        let mut canvas = ctx.canvas.borrow_mut();
        canvas.fill_style(theme.scrollbar_color);
        canvas.fill_round_rect(scrollbar_rect, 3.0);
    }
}

impl Default for MarkdownView {
    fn default() -> Self {
        Self::new()
    }
}

impl Element for MarkdownView {
    fn limits(&self, _ctx: &BasicContext) -> ViewLimits {
        ViewLimits::min_size(self.width, self.height)
    }

    fn stretch(&self) -> ViewStretch {
        ViewStretch::new(1.0, 1.0)
    }

    fn draw(&self, ctx: &Context) {
        self.draw_background(ctx);

        let (lines, total_height) = self.layout(ctx);
        let visible_height = ctx.bounds.height();
        let scroll = *self.scroll_offset.read().unwrap();

        {
            let mut canvas = ctx.canvas.borrow_mut();
            canvas.save();
            canvas.clip(ctx.bounds);
        }

        self.draw_content(ctx, &lines, scroll);

        {
            let mut canvas = ctx.canvas.borrow_mut();
            canvas.restore();
        }

        self.draw_scrollbar(ctx, total_height, visible_height);
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

    fn scroll(&mut self, ctx: &Context, dir: Point, p: Point) -> bool {
        self.handle_scroll(ctx, dir, p)
    }

    fn handle_scroll(&self, ctx: &Context, dir: Point, _p: Point) -> bool {
        if !self.enabled {
            return false;
        }

        // Also self-corrects `scroll_offset` against the fresh
        // `total_height` as a side effect -- see `layout`'s doc comment.
        let (_, total_height) = self.layout(ctx);
        let visible_height = ctx.bounds.height();

        if total_height <= visible_height {
            return false;
        }

        let mut scroll = self.scroll_offset.write().unwrap();
        *scroll = (*scroll - dir.y * 20.0).min(total_height - visible_height).max(0.0);

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

/// Creates a Markdown view.
pub fn markdown_view() -> MarkdownView {
    MarkdownView::new()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::support::canvas::Canvas;
    use crate::support::point::Extent;
    use crate::view::View;

    fn ctx_owned(width: f32, height: f32) -> (View, std::cell::RefCell<Canvas>, Rect) {
        let view = View::new(Extent::new(width, height));
        let canvas = std::cell::RefCell::new(Canvas::new(width as u32, height as u32).unwrap());
        let bounds = Rect::new(0.0, 0.0, width, height);
        (view, canvas, bounds)
    }

    #[test]
    fn empty_text_lays_out_with_zero_content_lines() {
        let view = MarkdownView::new().width(300.0).height(200.0);
        let (v, canvas, bounds) = ctx_owned(300.0, 200.0);
        let ctx = Context::new(&v, &canvas, bounds);

        let (lines, _) = view.layout(&ctx);
        assert!(lines.is_empty());
    }

    #[test]
    fn set_text_grows_total_content_height() {
        let view = MarkdownView::new().width(200.0).height(80.0);
        let (v, canvas, bounds) = ctx_owned(200.0, 80.0);
        let ctx = Context::new(&v, &canvas, bounds);

        let (_, height_before) = view.layout(&ctx);
        view.set_text("# Heading\n\nSome real paragraph text that will wrap across several lines given a narrow width.");
        let (_, height_after) = view.layout(&ctx);

        assert!(height_after > height_before);
    }

    #[test]
    fn set_text_does_not_reset_scroll() {
        let view = MarkdownView::new().width(200.0).height(80.0);
        view.set_text((0..40).map(|i| format!("line {i}")).collect::<Vec<_>>().join("\n\n"));
        let (v, canvas, bounds) = ctx_owned(200.0, 80.0);
        let ctx = Context::new(&v, &canvas, bounds);
        view.layout(&ctx);

        view.handle_scroll(&ctx, Point::new(0.0, -1000.0), Point::zero());
        let scrolled = *view.scroll_offset.read().unwrap();
        assert!(scrolled > 0.0, "expected scroll to have moved");

        view.set_text("same-length replacement content that also spans several lines\n\nmore\n\nmore\n\nmore");
        assert_eq!(*view.scroll_offset.read().unwrap(), scrolled, "set_text shouldn't reset scroll");
    }

    #[test]
    fn handle_scroll_is_a_noop_when_content_already_fits() {
        let view = MarkdownView::new().width(300.0).height(400.0).text("short");
        let (v, canvas, bounds) = ctx_owned(300.0, 400.0);
        let ctx = Context::new(&v, &canvas, bounds);

        let handled = view.handle_scroll(&ctx, Point::new(0.0, -5.0), Point::zero());

        assert!(!handled);
        assert_eq!(*view.scroll_offset.read().unwrap(), 0.0);
    }

    #[test]
    fn handle_scroll_clamps_into_a_valid_range_when_content_overflows() {
        let text = (0..40).map(|i| format!("paragraph number {i}")).collect::<Vec<_>>().join("\n\n");
        let view = MarkdownView::new().width(200.0).height(80.0).text(text);

        let (v, canvas, bounds) = ctx_owned(200.0, 80.0);
        let ctx = Context::new(&v, &canvas, bounds);

        let handled = view.handle_scroll(&ctx, Point::new(0.0, 1000.0), Point::zero());
        assert!(handled);
        assert_eq!(*view.scroll_offset.read().unwrap(), 0.0);

        let handled = view.handle_scroll(&ctx, Point::new(0.0, -100000.0), Point::zero());
        assert!(handled);
        let (_, total_height) = view.layout(&ctx);
        assert_eq!(*view.scroll_offset.read().unwrap(), total_height - 80.0);
    }
}
