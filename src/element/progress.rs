//! Progress bar element.

use std::any::Any;
use std::sync::RwLock;
use super::{Element, ViewLimits, ViewStretch};
use super::context::{BasicContext, Context};
use crate::support::point::Point;
use crate::support::rect::Rect;
use crate::support::color::Color;
use crate::support::theme::get_theme;

/// Progress bar style.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ProgressStyle {
    #[default]
    Linear,
    Circular,
}

/// A progress bar element.
pub struct ProgressBar {
    value: RwLock<f32>,
    style: ProgressStyle,
    background_color: Color,
    fill_color: Color,
    text_color: Color,
    show_percentage: bool,
    width: f32,
    height: f32,
    corner_radius: f32,
    indeterminate: bool,
    animation_offset: RwLock<f32>,
}

impl ProgressBar {
    /// Creates a new progress bar.
    pub fn new() -> Self {
        let theme = get_theme();
        Self {
            value: RwLock::new(0.0),
            style: ProgressStyle::Linear,
            background_color: theme.slider_slot_color,
            fill_color: theme.indicator_bright_color,
            text_color: theme.label_font_color,
            show_percentage: false,
            width: 200.0,
            height: 8.0,
            corner_radius: 4.0,
            indeterminate: false,
            animation_offset: RwLock::new(0.0),
        }
    }

    /// Sets the progress value (0.0 to 1.0).
    pub fn value(self, value: f32) -> Self {
        self.set_value(value);
        self
    }

    /// Sets the style.
    pub fn style(mut self, style: ProgressStyle) -> Self {
        self.style = style;
        if style == ProgressStyle::Circular {
            self.height = self.width; // Make it square
        }
        self
    }

    /// Sets the background color.
    pub fn background_color(mut self, color: Color) -> Self {
        self.background_color = color;
        self
    }

    /// Sets the fill color.
    pub fn fill_color(mut self, color: Color) -> Self {
        self.fill_color = color;
        self
    }

    /// Sets whether to show percentage text.
    pub fn show_percentage(mut self, show: bool) -> Self {
        self.show_percentage = show;
        if show && self.style == ProgressStyle::Linear {
            self.height = 20.0; // Make room for text
        }
        self
    }

    /// Sets indeterminate mode.
    pub fn indeterminate(mut self, indeterminate: bool) -> Self {
        self.indeterminate = indeterminate;
        self
    }

    /// Sets the dimensions.
    pub fn size(mut self, width: f32, height: f32) -> Self {
        self.width = width;
        self.height = height;
        self
    }

    /// Returns the current value.
    pub fn get_value(&self) -> f32 {
        *self.value.read().unwrap()
    }

    /// Sets the current value.
    pub fn set_value(&self, value: f32) {
        *self.value.write().unwrap() = value.clamp(0.0, 1.0);
    }

    /// Increments the value.
    pub fn increment(&self, delta: f32) {
        let current = self.get_value();
        self.set_value(current + delta);
    }

    fn draw_linear(&self, ctx: &Context) {
        let mut canvas = ctx.canvas.borrow_mut();
        let value = self.get_value();

        // Background
        canvas.fill_style(self.background_color);
        canvas.fill_round_rect(ctx.bounds, self.corner_radius);

        if self.indeterminate {
            // Animated indeterminate bar
            let offset = *self.animation_offset.read().unwrap();
            let bar_width = ctx.bounds.width() * 0.3;
            let x = ctx.bounds.left + (ctx.bounds.width() - bar_width) * offset;

            let fill_rect = Rect::new(
                x.max(ctx.bounds.left),
                ctx.bounds.top,
                (x + bar_width).min(ctx.bounds.right),
                ctx.bounds.bottom,
            );

            canvas.fill_style(self.fill_color);
            canvas.fill_round_rect(fill_rect, self.corner_radius);
        } else {
            // Determinate progress bar
            if value > 0.0 {
                let fill_width = ctx.bounds.width() * value;
                let fill_rect = Rect::new(
                    ctx.bounds.left,
                    ctx.bounds.top,
                    ctx.bounds.left + fill_width,
                    ctx.bounds.bottom,
                );

                canvas.fill_style(self.fill_color);
                canvas.fill_round_rect(fill_rect, self.corner_radius);
            }

            // Percentage text
            if self.show_percentage {
                let text = format!("{}%", (value * 100.0) as i32);
                let theme = get_theme();

                canvas.fill_style(self.text_color);
                canvas.font_size(theme.label_font_size * 0.8);

                let x = ctx.bounds.center().x - text.len() as f32 * theme.label_font_size * 0.2;
                let y = ctx.bounds.center().y + theme.label_font_size * 0.25;
                canvas.fill_text(&text, Point::new(x, y));
            }
        }
    }

    fn draw_circular(&self, ctx: &Context) {
        let mut canvas = ctx.canvas.borrow_mut();
        let value = self.get_value();
        let theme = get_theme();

        let center = ctx.bounds.center();
        let radius = (ctx.bounds.width().min(ctx.bounds.height()) / 2.0) - 4.0;
        let stroke_width = 6.0;

        // Background circle
        canvas.stroke_style(self.background_color);
        canvas.line_width(stroke_width);
        canvas.begin_path();
        canvas.add_circle(crate::support::circle::Circle::new(center, radius));
        canvas.stroke();

        if self.indeterminate {
            // Animated arc
            let offset = *self.animation_offset.read().unwrap();
            let start_angle = offset * std::f32::consts::PI * 2.0 - std::f32::consts::PI / 2.0;
            let end_angle = start_angle + std::f32::consts::PI * 0.75;

            canvas.stroke_style(self.fill_color);
            canvas.line_width(stroke_width);
            canvas.begin_path();

            let segments = 20;
            for i in 0..=segments {
                let t = i as f32 / segments as f32;
                let angle = start_angle + t * (end_angle - start_angle);
                let x = center.x + radius * angle.cos();
                let y = center.y + radius * angle.sin();

                if i == 0 {
                    canvas.move_to(Point::new(x, y));
                } else {
                    canvas.line_to(Point::new(x, y));
                }
            }
            canvas.stroke();
        } else {
            // Progress arc
            if value > 0.0 {
                let start_angle = -std::f32::consts::PI / 2.0;
                let end_angle = start_angle + value * std::f32::consts::PI * 2.0;

                canvas.stroke_style(self.fill_color);
                canvas.line_width(stroke_width);
                canvas.begin_path();

                let segments = (value * 40.0).max(2.0) as i32;
                for i in 0..=segments {
                    let t = i as f32 / segments as f32;
                    let angle = start_angle + t * (end_angle - start_angle);
                    let x = center.x + radius * angle.cos();
                    let y = center.y + radius * angle.sin();

                    if i == 0 {
                        canvas.move_to(Point::new(x, y));
                    } else {
                        canvas.line_to(Point::new(x, y));
                    }
                }
                canvas.stroke();
            }

            // Percentage text in center
            if self.show_percentage {
                let text = format!("{}%", (value * 100.0) as i32);

                canvas.fill_style(self.text_color);
                canvas.font_size(theme.label_font_size);

                let x = center.x - text.len() as f32 * theme.label_font_size * 0.25;
                let y = center.y + theme.label_font_size * 0.35;
                canvas.fill_text(&text, Point::new(x, y));
            }
        }
    }
}

impl Default for ProgressBar {
    fn default() -> Self {
        Self::new()
    }
}

impl Element for ProgressBar {
    fn limits(&self, _ctx: &BasicContext) -> ViewLimits {
        ViewLimits::fixed(self.width, self.height)
    }

    fn stretch(&self) -> ViewStretch {
        match self.style {
            ProgressStyle::Linear => ViewStretch::new(1.0, 0.0),
            ProgressStyle::Circular => ViewStretch::new(0.0, 0.0),
        }
    }

    fn draw(&self, ctx: &Context) {
        match self.style {
            ProgressStyle::Linear => self.draw_linear(ctx),
            ProgressStyle::Circular => self.draw_circular(ctx),
        }
    }

    fn as_any(&self) -> &dyn Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }
}

/// Creates a progress bar.
pub fn progress_bar() -> ProgressBar {
    ProgressBar::new()
}

/// Creates a progress bar with initial value.
pub fn progress_bar_with_value(value: f32) -> ProgressBar {
    ProgressBar::new().value(value)
}

/// Creates a circular progress indicator.
pub fn circular_progress() -> ProgressBar {
    ProgressBar::new()
        .style(ProgressStyle::Circular)
        .size(50.0, 50.0)
}

/// Creates an indeterminate progress bar.
pub fn indeterminate_progress() -> ProgressBar {
    ProgressBar::new().indeterminate(true)
}
