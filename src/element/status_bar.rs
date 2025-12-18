//! Status bar element.

use std::any::Any;
use std::sync::RwLock;
use super::{Element, ViewLimits, ViewStretch};
use super::context::{BasicContext, Context};
use crate::support::point::Point;
use crate::support::color::Color;
use crate::support::theme::get_theme;

/// A status bar segment.
#[derive(Debug, Clone)]
pub struct StatusSegment {
    pub text: String,
    pub flex: f32, // Relative width (0.0 for fixed width based on text)
}

impl StatusSegment {
    pub fn new(text: impl Into<String>) -> Self {
        Self {
            text: text.into(),
            flex: 0.0,
        }
    }

    pub fn flex(text: impl Into<String>, flex: f32) -> Self {
        Self {
            text: text.into(),
            flex,
        }
    }
}

/// A status bar element typically shown at the bottom of a window.
pub struct StatusBar {
    segments: RwLock<Vec<StatusSegment>>,
    background_color: Color,
    text_color: Color,
    separator_color: Color,
    height: f32,
    padding: f32,
}

impl StatusBar {
    /// Creates a new status bar.
    pub fn new() -> Self {
        let theme = get_theme();
        Self {
            segments: RwLock::new(Vec::new()),
            background_color: theme.panel_color,
            text_color: theme.label_font_color.with_alpha(0.8),
            separator_color: theme.frame_color,
            height: 24.0,
            padding: 8.0,
        }
    }

    /// Sets the segments.
    pub fn segments(self, segments: Vec<StatusSegment>) -> Self {
        *self.segments.write().unwrap() = segments;
        self
    }

    /// Sets a single text.
    pub fn text(self, text: impl Into<String>) -> Self {
        *self.segments.write().unwrap() = vec![StatusSegment::flex(text, 1.0)];
        self
    }

    /// Sets the background color.
    pub fn background_color(mut self, color: Color) -> Self {
        self.background_color = color;
        self
    }

    /// Sets the text color.
    pub fn text_color(mut self, color: Color) -> Self {
        self.text_color = color;
        self
    }

    /// Sets the height.
    pub fn height(mut self, height: f32) -> Self {
        self.height = height;
        self
    }

    /// Updates a segment's text.
    pub fn set_segment_text(&self, index: usize, text: impl Into<String>) {
        let mut segments = self.segments.write().unwrap();
        if let Some(segment) = segments.get_mut(index) {
            segment.text = text.into();
        }
    }

    /// Sets the main text (first segment).
    pub fn set_text(&self, text: impl Into<String>) {
        self.set_segment_text(0, text);
    }

    fn calculate_segment_widths(&self, total_width: f32) -> Vec<f32> {
        let segments = self.segments.read().unwrap();
        let theme = get_theme();

        if segments.is_empty() {
            return Vec::new();
        }

        let mut widths = Vec::with_capacity(segments.len());
        let mut fixed_width = 0.0f32;
        let mut total_flex = 0.0f32;

        // Calculate fixed widths and total flex
        for segment in segments.iter() {
            if segment.flex == 0.0 {
                let w = segment.text.len() as f32 * theme.label_font_size * 0.6 + self.padding * 2.0;
                widths.push(w);
                fixed_width += w;
            } else {
                widths.push(0.0);
                total_flex += segment.flex;
            }
        }

        // Distribute remaining space to flex segments
        let remaining = (total_width - fixed_width).max(0.0);
        for (i, segment) in segments.iter().enumerate() {
            if segment.flex > 0.0 {
                widths[i] = remaining * (segment.flex / total_flex);
            }
        }

        widths
    }
}

impl Default for StatusBar {
    fn default() -> Self {
        Self::new()
    }
}

impl Element for StatusBar {
    fn limits(&self, _ctx: &BasicContext) -> ViewLimits {
        ViewLimits {
            min: Point::new(100.0, self.height),
            max: Point::new(super::FULL_EXTENT, self.height),
        }
    }

    fn stretch(&self) -> ViewStretch {
        ViewStretch::new(1.0, 0.0)
    }

    fn draw(&self, ctx: &Context) {
        let mut canvas = ctx.canvas.borrow_mut();
        let theme = get_theme();

        // Background
        canvas.fill_style(self.background_color);
        canvas.fill_rect(ctx.bounds);

        // Top border
        canvas.stroke_style(self.separator_color);
        canvas.line_width(1.0);
        canvas.begin_path();
        canvas.move_to(Point::new(ctx.bounds.left, ctx.bounds.top));
        canvas.line_to(Point::new(ctx.bounds.right, ctx.bounds.top));
        canvas.stroke();

        // Draw segments
        let segments = self.segments.read().unwrap();
        let widths = self.calculate_segment_widths(ctx.bounds.width());

        let mut x = ctx.bounds.left;
        for (i, segment) in segments.iter().enumerate() {
            let width = widths.get(i).copied().unwrap_or(0.0);

            // Text
            canvas.fill_style(self.text_color);
            canvas.font_size(theme.label_font_size * 0.9);

            let text_x = x + self.padding;
            let text_y = ctx.bounds.center().y + theme.label_font_size * 0.3;

            // Clip text if too long
            let max_chars = ((width - self.padding * 2.0) / (theme.label_font_size * 0.5)) as usize;
            let display_text = if segment.text.len() > max_chars && max_chars > 3 {
                format!("{}...", &segment.text[..max_chars - 3])
            } else {
                segment.text.clone()
            };

            canvas.fill_text(&display_text, Point::new(text_x, text_y));

            x += width;

            // Separator (except for last segment)
            if i < segments.len() - 1 {
                canvas.stroke_style(self.separator_color);
                canvas.line_width(1.0);
                canvas.begin_path();
                canvas.move_to(Point::new(x, ctx.bounds.top + 4.0));
                canvas.line_to(Point::new(x, ctx.bounds.bottom - 4.0));
                canvas.stroke();
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

/// Creates a status bar.
pub fn status_bar() -> StatusBar {
    StatusBar::new()
}

/// Creates a status bar with text.
pub fn status_bar_with_text(text: impl Into<String>) -> StatusBar {
    StatusBar::new().text(text)
}
