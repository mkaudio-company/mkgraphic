//! Slider elements for selecting values within a range.

use std::any::Any;
use std::sync::RwLock;
use super::{Element, ViewLimits, ViewStretch};
use super::context::{BasicContext, Context};
use crate::support::point::Point;
use crate::support::rect::Rect;
use crate::support::color::Color;
use crate::support::theme::get_theme;
use crate::view::{MouseButton, MouseButtonKind, CursorTracking};

/// Slider state.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum SliderState {
    #[default]
    Normal,
    Hover,
    Dragging,
    Disabled,
}

/// Slider orientation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum SliderOrientation {
    #[default]
    Horizontal,
    Vertical,
}

/// Callback type for value changes.
pub type ValueChangeCallback = Box<dyn Fn(f64) + Send + Sync>;

/// A basic slider element for selecting a value within a range.
pub struct Slider {
    value: RwLock<f64>,
    min_value: f64,
    max_value: f64,
    step: Option<f64>,
    orientation: SliderOrientation,
    state: RwLock<SliderState>,
    track_color: Color,
    thumb_color: Color,
    active_color: Color,
    thumb_size: f32,
    track_height: f32,
    length: f32,
    enabled: bool,
    on_change: Option<ValueChangeCallback>,
    drag_start_value: RwLock<f64>,
}

impl Slider {
    /// Creates a new horizontal slider with default range [0.0, 1.0].
    pub fn new() -> Self {
        let theme = get_theme();
        Self {
            value: RwLock::new(0.0),
            min_value: 0.0,
            max_value: 1.0,
            step: None,
            orientation: SliderOrientation::Horizontal,
            state: RwLock::new(SliderState::Normal),
            track_color: theme.slider_slot_color,
            thumb_color: theme.slider_thumb_color,
            active_color: theme.indicator_bright_color,
            thumb_size: 16.0,
            track_height: 4.0,
            length: 150.0,
            enabled: true,
            on_change: None,
            drag_start_value: RwLock::new(0.0),
        }
    }

    /// Creates a slider with specified range.
    pub fn with_range(min: f64, max: f64) -> Self {
        let mut slider = Self::new();
        slider.min_value = min;
        slider.max_value = max;
        slider.set_value(min);
        slider
    }

    /// Sets the orientation.
    pub fn orientation(mut self, orientation: SliderOrientation) -> Self {
        self.orientation = orientation;
        self
    }

    /// Sets the initial value.
    pub fn value(self, value: f64) -> Self {
        self.set_value(value);
        self
    }

    /// Sets the step increment (for discrete values).
    pub fn step(mut self, step: f64) -> Self {
        self.step = Some(step);
        self
    }

    /// Sets the track color.
    pub fn track_color(mut self, color: Color) -> Self {
        self.track_color = color;
        self
    }

    /// Sets the thumb color.
    pub fn thumb_color(mut self, color: Color) -> Self {
        self.thumb_color = color;
        self
    }

    /// Sets the active (filled) track color.
    pub fn active_color(mut self, color: Color) -> Self {
        self.active_color = color;
        self
    }

    /// Sets the thumb size.
    pub fn thumb_size(mut self, size: f32) -> Self {
        self.thumb_size = size;
        self
    }

    /// Sets the slider length.
    pub fn length(mut self, length: f32) -> Self {
        self.length = length;
        self
    }

    /// Sets the value change callback.
    pub fn on_change<F: Fn(f64) + Send + Sync + 'static>(mut self, callback: F) -> Self {
        self.on_change = Some(Box::new(callback));
        self
    }

    /// Returns the current value.
    pub fn get_value(&self) -> f64 {
        *self.value.read().unwrap()
    }

    /// Sets the current value.
    pub fn set_value(&self, value: f64) {
        let clamped = value.clamp(self.min_value, self.max_value);
        let stepped = if let Some(step) = self.step {
            let steps = ((clamped - self.min_value) / step).round();
            self.min_value + steps * step
        } else {
            clamped
        };
        *self.value.write().unwrap() = stepped.clamp(self.min_value, self.max_value);
    }

    /// Returns the normalized value (0.0 to 1.0).
    fn normalized_value(&self) -> f64 {
        let value = self.get_value();
        if (self.max_value - self.min_value).abs() < f64::EPSILON {
            0.0
        } else {
            (value - self.min_value) / (self.max_value - self.min_value)
        }
    }

    /// Sets value from normalized (0.0 to 1.0).
    fn set_normalized_value(&self, normalized: f64) {
        let value = self.min_value + normalized * (self.max_value - self.min_value);
        self.set_value(value);
    }

    /// Returns the thumb position based on bounds.
    fn thumb_position(&self, bounds: &Rect) -> Point {
        let norm = self.normalized_value() as f32;
        match self.orientation {
            SliderOrientation::Horizontal => {
                let track_start = bounds.left + self.thumb_size / 2.0;
                let track_end = bounds.right - self.thumb_size / 2.0;
                let x = track_start + norm * (track_end - track_start);
                Point::new(x, bounds.center().y)
            }
            SliderOrientation::Vertical => {
                let track_start = bounds.bottom - self.thumb_size / 2.0;
                let track_end = bounds.top + self.thumb_size / 2.0;
                let y = track_start - norm * (track_start - track_end);
                Point::new(bounds.center().x, y)
            }
        }
    }

    /// Converts a point to a normalized value.
    fn point_to_normalized(&self, bounds: &Rect, p: Point) -> f64 {
        match self.orientation {
            SliderOrientation::Horizontal => {
                let track_start = bounds.left + self.thumb_size / 2.0;
                let track_end = bounds.right - self.thumb_size / 2.0;
                ((p.x - track_start) / (track_end - track_start)).clamp(0.0, 1.0) as f64
            }
            SliderOrientation::Vertical => {
                let track_start = bounds.bottom - self.thumb_size / 2.0;
                let track_end = bounds.top + self.thumb_size / 2.0;
                ((track_start - p.y) / (track_start - track_end)).clamp(0.0, 1.0) as f64
            }
        }
    }

    fn draw_track(&self, ctx: &Context) {
        let mut canvas = ctx.canvas.borrow_mut();
        let bounds = ctx.bounds;
        let theme = get_theme();

        let (track_rect, active_rect) = match self.orientation {
            SliderOrientation::Horizontal => {
                let track_top = bounds.center().y - self.track_height / 2.0;
                let track_bottom = track_top + self.track_height;
                let thumb_x = self.thumb_position(&bounds).x;

                let track = Rect::new(
                    bounds.left + self.thumb_size / 2.0,
                    track_top,
                    bounds.right - self.thumb_size / 2.0,
                    track_bottom,
                );
                let active = Rect::new(
                    bounds.left + self.thumb_size / 2.0,
                    track_top,
                    thumb_x,
                    track_bottom,
                );
                (track, active)
            }
            SliderOrientation::Vertical => {
                let track_left = bounds.center().x - self.track_height / 2.0;
                let track_right = track_left + self.track_height;
                let thumb_y = self.thumb_position(&bounds).y;

                let track = Rect::new(
                    track_left,
                    bounds.top + self.thumb_size / 2.0,
                    track_right,
                    bounds.bottom - self.thumb_size / 2.0,
                );
                let active = Rect::new(
                    track_left,
                    thumb_y,
                    track_right,
                    bounds.bottom - self.thumb_size / 2.0,
                );
                (track, active)
            }
        };

        // Draw background track
        canvas.fill_style(self.track_color);
        canvas.fill_round_rect(track_rect, theme.slider_slot_corner_radius);

        // Draw active portion
        if active_rect.width() > 0.0 && active_rect.height() > 0.0 {
            canvas.fill_style(self.active_color);
            canvas.fill_round_rect(active_rect, theme.slider_slot_corner_radius);
        }
    }

    fn draw_thumb(&self, ctx: &Context) {
        let mut canvas = ctx.canvas.borrow_mut();
        let state = *self.state.read().unwrap();

        let color = match state {
            SliderState::Normal => self.thumb_color,
            SliderState::Hover => self.thumb_color.level(1.2),
            SliderState::Dragging => self.thumb_color.level(0.8),
            SliderState::Disabled => self.thumb_color.with_alpha(0.5),
        };

        let thumb_pos = self.thumb_position(&ctx.bounds);
        let thumb_radius = self.thumb_size / 2.0;

        canvas.fill_style(color);
        canvas.begin_path();
        canvas.add_circle(crate::support::circle::Circle::new(thumb_pos, thumb_radius));
        canvas.fill();
    }
}

impl Default for Slider {
    fn default() -> Self {
        Self::new()
    }
}

impl Element for Slider {
    fn limits(&self, _ctx: &BasicContext) -> ViewLimits {
        match self.orientation {
            SliderOrientation::Horizontal => {
                ViewLimits::fixed(self.length, self.thumb_size)
            }
            SliderOrientation::Vertical => {
                ViewLimits::fixed(self.thumb_size, self.length)
            }
        }
    }

    fn stretch(&self) -> ViewStretch {
        match self.orientation {
            SliderOrientation::Horizontal => ViewStretch::new(1.0, 0.0),
            SliderOrientation::Vertical => ViewStretch::new(0.0, 1.0),
        }
    }

    fn draw(&self, ctx: &Context) {
        self.draw_track(ctx);
        self.draw_thumb(ctx);
    }

    fn hit_test(&self, ctx: &Context, p: Point, _leaf: bool, _control: bool) -> Option<&dyn Element> {
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
        if !self.enabled || btn.button != MouseButtonKind::Left {
            return false;
        }

        let mut state = self.state.write().unwrap();
        if btn.down {
            *state = SliderState::Dragging;
            *self.drag_start_value.write().unwrap() = self.get_value();

            // Jump to click position
            let normalized = self.point_to_normalized(&ctx.bounds, btn.pos);
            drop(state);
            self.set_normalized_value(normalized);
            if let Some(ref callback) = self.on_change {
                callback(self.get_value());
            }
        } else {
            *state = if ctx.bounds.contains(btn.pos) {
                SliderState::Hover
            } else {
                SliderState::Normal
            };
        }

        true
    }

    fn drag(&mut self, ctx: &Context, btn: MouseButton) {
        if !self.enabled {
            return;
        }

        let normalized = self.point_to_normalized(&ctx.bounds, btn.pos);
        self.set_normalized_value(normalized);
        if let Some(ref callback) = self.on_change {
            callback(self.get_value());
        }
    }

    fn cursor(&mut self, ctx: &Context, p: Point, status: CursorTracking) -> bool {
        if !self.enabled {
            return false;
        }

        let mut state = self.state.write().unwrap();
        if *state == SliderState::Dragging {
            return true;
        }

        match status {
            CursorTracking::Entering | CursorTracking::Hovering => {
                *state = SliderState::Hover;
            }
            CursorTracking::Leaving => {
                *state = SliderState::Normal;
            }
        }

        true
    }

    fn enable(&mut self, state: bool) {
        self.enabled = state;
        let mut slider_state = self.state.write().unwrap();
        if !state {
            *slider_state = SliderState::Disabled;
        } else if *slider_state == SliderState::Disabled {
            *slider_state = SliderState::Normal;
        }
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

/// Creates a horizontal slider.
pub fn slider() -> Slider {
    Slider::new()
}

/// Creates a horizontal slider with range.
pub fn slider_with_range(min: f64, max: f64) -> Slider {
    Slider::with_range(min, max)
}

/// Creates a vertical slider.
pub fn vslider() -> Slider {
    Slider::new().orientation(SliderOrientation::Vertical)
}

/// Creates a vertical slider with range.
pub fn vslider_with_range(min: f64, max: f64) -> Slider {
    Slider::with_range(min, max).orientation(SliderOrientation::Vertical)
}
