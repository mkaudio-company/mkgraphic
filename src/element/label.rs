//! Label elements for displaying text.

use std::any::Any;
use super::{Element, ViewLimits};
use super::context::{BasicContext, Context};
use crate::support::color::Color;
use crate::support::font::Font;
use crate::support::theme::get_theme;

/// A simple text label element.
pub struct Label {
    text: String,
    font: Font,
    font_size: f32,
    color: Color,
}

impl Label {
    /// Creates a new label with the given text.
    pub fn new(text: impl Into<String>) -> Self {
        let theme = get_theme();
        Self {
            text: text.into(),
            font: theme.label_font.clone(),
            font_size: theme.label_font_size,
            color: theme.label_font_color,
        }
    }

    /// Sets the text.
    pub fn set_text(&mut self, text: impl Into<String>) {
        self.text = text.into();
    }

    /// Returns the text.
    pub fn text(&self) -> &str {
        &self.text
    }

    /// Sets the font.
    pub fn with_font(mut self, font: Font) -> Self {
        self.font = font;
        self
    }

    /// Sets the font size.
    pub fn with_font_size(mut self, size: f32) -> Self {
        self.font_size = size;
        self
    }

    /// Sets the color.
    pub fn with_color(mut self, color: Color) -> Self {
        self.color = color;
        self
    }

    /// Returns the font.
    pub fn font(&self) -> &Font {
        &self.font
    }

    /// Returns the font size.
    pub fn font_size(&self) -> f32 {
        self.font_size
    }

    /// Returns the color.
    pub fn color(&self) -> Color {
        self.color
    }
}

impl Element for Label {
    fn limits(&self, ctx: &BasicContext) -> ViewLimits {
        // Estimate text size based on font size and character count
        // In a real implementation, this would use proper text measurement
        let estimated_width = self.text.len() as f32 * self.font_size * 0.6;
        let estimated_height = self.font_size * 1.2;

        ViewLimits::fixed(estimated_width, estimated_height)
    }

    fn draw(&self, ctx: &Context) {
        let mut canvas = ctx.canvas.borrow_mut();
        canvas.fill_style(self.color);
        canvas.font(self.font.clone());
        canvas.font_size(self.font_size);
        canvas.fill_text(&self.text, ctx.bounds.top_left());
    }

    fn as_any(&self) -> &dyn Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }
}

/// Heading label with larger font size.
pub struct Heading {
    label: Label,
}

impl Heading {
    /// Creates a new heading.
    pub fn new(text: impl Into<String>) -> Self {
        let theme = get_theme();
        Self {
            label: Label::new(text)
                .with_font(theme.heading_font.clone())
                .with_font_size(theme.heading_font_size)
                .with_color(theme.heading_font_color),
        }
    }

    /// Sets the text.
    pub fn set_text(&mut self, text: impl Into<String>) {
        self.label.set_text(text);
    }

    /// Returns the text.
    pub fn text(&self) -> &str {
        self.label.text()
    }
}

impl Element for Heading {
    fn limits(&self, ctx: &BasicContext) -> ViewLimits {
        self.label.limits(ctx)
    }

    fn draw(&self, ctx: &Context) {
        self.label.draw(ctx);
    }

    fn as_any(&self) -> &dyn Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }
}

/// A static text element (label that doesn't change).
pub struct StaticText {
    text: &'static str,
    font_size: f32,
    color: Color,
}

impl StaticText {
    /// Creates a new static text element.
    pub const fn new(text: &'static str) -> Self {
        Self {
            text,
            font_size: 14.0,
            color: Color::new(0.8, 0.8, 0.8, 1.0),
        }
    }

    /// Sets the font size.
    pub const fn with_font_size(mut self, size: f32) -> Self {
        self.font_size = size;
        self
    }

    /// Sets the color.
    pub const fn with_color(mut self, color: Color) -> Self {
        self.color = color;
        self
    }
}

impl Element for StaticText {
    fn limits(&self, ctx: &BasicContext) -> ViewLimits {
        let estimated_width = self.text.len() as f32 * self.font_size * 0.6;
        let estimated_height = self.font_size * 1.2;
        ViewLimits::fixed(estimated_width, estimated_height)
    }

    fn draw(&self, ctx: &Context) {
        let mut canvas = ctx.canvas.borrow_mut();
        canvas.fill_style(self.color);
        canvas.font_size(self.font_size);
        canvas.fill_text(self.text, ctx.bounds.top_left());
    }

    fn as_any(&self) -> &dyn Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }
}

// Convenience functions

/// Creates a label.
pub fn label(text: impl Into<String>) -> Label {
    Label::new(text)
}

/// Creates a heading.
pub fn heading(text: impl Into<String>) -> Heading {
    Heading::new(text)
}

/// Creates static text.
pub const fn static_text(text: &'static str) -> StaticText {
    StaticText::new(text)
}
