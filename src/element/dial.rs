//! Dial/knob elements for rotary value selection.

use std::any::Any;
use std::sync::RwLock;
use std::f32::consts::PI;
use super::{Element, ViewLimits, ViewStretch};
use super::context::{BasicContext, Context};
use crate::support::point::Point;
use crate::support::color::Color;
use crate::support::theme::get_theme;
use crate::view::{MouseButton, MouseButtonKind, CursorTracking};

/// Dial state.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum DialState {
    #[default]
    Normal,
    Hover,
    Dragging,
    Disabled,
}

/// Callback type for dial value changes.
pub type DialChangeCallback = Box<dyn Fn(f64) + Send + Sync>;

/// A rotary dial/knob element for selecting values.
pub struct Dial {
    value: RwLock<f64>,
    min_value: f64,
    max_value: f64,
    state: RwLock<DialState>,
    dial_color: Color,
    indicator_color: Color,
    gauge_color: Color,
    gauge_width: f32,
    size: f32,
    /// Start angle in radians (measured from top, clockwise positive)
    start_angle: f32,
    /// End angle in radians
    end_angle: f32,
    enabled: bool,
    on_change: Option<DialChangeCallback>,
    drag_start_y: RwLock<f32>,
    drag_start_value: RwLock<f64>,
    /// Center position for angular calculations (set during click)
    dial_center: RwLock<Point>,
    /// Starting angle when drag began
    drag_start_angle: RwLock<f32>,
}

impl Dial {
    /// Creates a new dial with default range [0.0, 1.0].
    pub fn new() -> Self {
        let theme = get_theme();
        Self {
            value: RwLock::new(0.0),
            min_value: 0.0,
            max_value: 1.0,
            state: RwLock::new(DialState::Normal),
            dial_color: theme.dial_color,
            indicator_color: theme.dial_indicator_color,
            gauge_color: theme.dial_gauge_color,
            gauge_width: theme.dial_gauge_width,
            size: 50.0,
            start_angle: -135.0 * PI / 180.0,  // -135 degrees from top
            end_angle: 135.0 * PI / 180.0,     // 135 degrees from top
            enabled: true,
            on_change: None,
            drag_start_y: RwLock::new(0.0),
            drag_start_value: RwLock::new(0.0),
            dial_center: RwLock::new(Point::new(0.0, 0.0)),
            drag_start_angle: RwLock::new(0.0),
        }
    }

    /// Calculates the angle from dial center to a point.
    /// Returns angle in radians, measured clockwise from top (12 o'clock).
    fn angle_to_point(&self, center: Point, p: Point) -> f32 {
        let dx = p.x - center.x;
        let dy = p.y - center.y;
        // atan2 gives angle from positive x-axis, counterclockwise
        // We want angle from top (negative y-axis), clockwise
        // So we use atan2(dx, -dy) to rotate the reference
        dx.atan2(-dy)
    }

    /// Converts an angle to a normalized value (0.0 to 1.0).
    fn angle_to_normalized(&self, angle: f32) -> f64 {
        let angle_range = self.end_angle - self.start_angle;
        let normalized = ((angle - self.start_angle) / angle_range) as f64;
        normalized.clamp(0.0, 1.0)
    }

    /// Creates a dial with specified range.
    pub fn with_range(min: f64, max: f64) -> Self {
        let mut dial = Self::new();
        dial.min_value = min;
        dial.max_value = max;
        dial.set_value(min);
        dial
    }

    /// Sets the initial value.
    pub fn value(self, value: f64) -> Self {
        self.set_value(value);
        self
    }

    /// Sets the dial color.
    pub fn dial_color(mut self, color: Color) -> Self {
        self.dial_color = color;
        self
    }

    /// Sets the indicator color.
    pub fn indicator_color(mut self, color: Color) -> Self {
        self.indicator_color = color;
        self
    }

    /// Sets the gauge color.
    pub fn gauge_color(mut self, color: Color) -> Self {
        self.gauge_color = color;
        self
    }

    /// Sets the dial size.
    pub fn size(mut self, size: f32) -> Self {
        self.size = size;
        self
    }

    /// Sets the gauge width.
    pub fn gauge_width(mut self, width: f32) -> Self {
        self.gauge_width = width;
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
        *self.value.write().unwrap() = value.clamp(self.min_value, self.max_value);
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
        let value = self.min_value + normalized.clamp(0.0, 1.0) * (self.max_value - self.min_value);
        self.set_value(value);
    }

    /// Returns the angle for the current value.
    fn value_to_angle(&self) -> f32 {
        let norm = self.normalized_value() as f32;
        self.start_angle + norm * (self.end_angle - self.start_angle)
    }

    fn draw_gauge_background(&self, ctx: &Context) {
        let mut canvas = ctx.canvas.borrow_mut();
        let center = ctx.bounds.center();
        let radius = self.size / 2.0 - self.gauge_width / 2.0;

        // Draw background arc
        let state = *self.state.read().unwrap();
        let color = match state {
            DialState::Disabled => self.dial_color.with_alpha(0.3),
            _ => self.dial_color.with_alpha(0.3),
        };

        canvas.stroke_style(color);
        canvas.line_width(self.gauge_width);

        // Draw arc from start to end angle
        let segments = 32;
        let angle_range = self.end_angle - self.start_angle;

        canvas.begin_path();
        for i in 0..=segments {
            let t = i as f32 / segments as f32;
            let angle = self.start_angle + t * angle_range - PI / 2.0; // Adjust for coordinate system
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

    fn draw_gauge_value(&self, ctx: &Context) {
        let mut canvas = ctx.canvas.borrow_mut();
        let center = ctx.bounds.center();
        let radius = self.size / 2.0 - self.gauge_width / 2.0;
        let current_angle = self.value_to_angle();

        let state = *self.state.read().unwrap();
        let color = match state {
            DialState::Normal => self.gauge_color,
            DialState::Hover => self.gauge_color.level(1.2),
            DialState::Dragging => self.gauge_color.level(1.3),
            DialState::Disabled => self.gauge_color.with_alpha(0.5),
        };

        canvas.stroke_style(color);
        canvas.line_width(self.gauge_width);

        // Draw arc from start to current value
        let segments = 32;
        let angle_range = current_angle - self.start_angle;

        if angle_range.abs() > 0.01 {
            canvas.begin_path();
            let segment_count = ((segments as f32) * (angle_range.abs() / (self.end_angle - self.start_angle))).ceil() as i32;
            for i in 0..=segment_count.max(1) {
                let t = i as f32 / segment_count.max(1) as f32;
                let angle = self.start_angle + t * angle_range - PI / 2.0;
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
    }

    fn draw_indicator(&self, ctx: &Context) {
        let mut canvas = ctx.canvas.borrow_mut();
        let center = ctx.bounds.center();
        let current_angle = self.value_to_angle() - PI / 2.0;

        let state = *self.state.read().unwrap();
        let color = match state {
            DialState::Normal => self.indicator_color,
            DialState::Hover => self.indicator_color.level(1.2),
            DialState::Dragging => self.indicator_color.level(1.3),
            DialState::Disabled => self.indicator_color.with_alpha(0.5),
        };

        // Draw indicator line
        let inner_radius = self.size / 2.0 - self.gauge_width - 4.0;
        let outer_radius = self.size / 2.0 - self.gauge_width / 2.0;

        let inner_x = center.x + inner_radius * current_angle.cos();
        let inner_y = center.y + inner_radius * current_angle.sin();
        let outer_x = center.x + outer_radius * current_angle.cos();
        let outer_y = center.y + outer_radius * current_angle.sin();

        canvas.stroke_style(color);
        canvas.line_width(2.0);
        canvas.begin_path();
        canvas.move_to(Point::new(inner_x, inner_y));
        canvas.line_to(Point::new(outer_x, outer_y));
        canvas.stroke();

        // Draw center dot
        canvas.fill_style(color);
        canvas.begin_path();
        canvas.add_circle(crate::support::circle::Circle::new(center, 4.0));
        canvas.fill();
    }
}

impl Default for Dial {
    fn default() -> Self {
        Self::new()
    }
}

impl Element for Dial {
    fn limits(&self, _ctx: &BasicContext) -> ViewLimits {
        ViewLimits::fixed(self.size, self.size)
    }

    fn stretch(&self) -> ViewStretch {
        ViewStretch::new(0.0, 0.0)
    }

    fn draw(&self, ctx: &Context) {
        self.draw_gauge_background(ctx);
        self.draw_gauge_value(ctx);
        self.draw_indicator(ctx);
    }

    fn hit_test(&self, ctx: &Context, p: Point, _leaf: bool, _control: bool) -> Option<&dyn Element> {
        if ctx.bounds.contains(p) && self.enabled {
            // Check if within the circular dial area
            let center = ctx.bounds.center();
            let dx = p.x - center.x;
            let dy = p.y - center.y;
            let dist = (dx * dx + dy * dy).sqrt();
            if dist <= self.size / 2.0 {
                return Some(self);
            }
        }
        None
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
            *state = DialState::Dragging;
            // Store dial center for angular calculations
            let center = ctx.bounds.center();
            *self.dial_center.write().unwrap() = center;
            *self.drag_start_y.write().unwrap() = btn.pos.y;
            *self.drag_start_value.write().unwrap() = self.get_value();
            // Store initial angle for relative angular movement
            *self.drag_start_angle.write().unwrap() = self.angle_to_point(center, btn.pos);
        } else {
            *state = if ctx.bounds.contains(btn.pos) {
                DialState::Hover
            } else {
                DialState::Normal
            };
        }

        true
    }

    fn drag(&mut self, ctx: &Context, btn: MouseButton) {
        self.handle_drag(ctx, btn);
    }

    fn handle_drag(&self, _ctx: &Context, btn: MouseButton) {
        if !self.enabled {
            return;
        }

        let center = *self.dial_center.read().unwrap();
        let drag_start_angle = *self.drag_start_angle.read().unwrap();
        let drag_start_value = *self.drag_start_value.read().unwrap();

        // Calculate current angle from center to mouse position
        let current_angle = self.angle_to_point(center, btn.pos);

        // Calculate angular delta
        let mut angle_delta = current_angle - drag_start_angle;

        // Handle wrap-around at ±π
        if angle_delta > PI {
            angle_delta -= 2.0 * PI;
        } else if angle_delta < -PI {
            angle_delta += 2.0 * PI;
        }

        // Convert angle delta to normalized value change
        let angle_range = self.end_angle - self.start_angle;
        let delta_normalized = (angle_delta / angle_range) as f64;

        let start_normalized = (drag_start_value - self.min_value) / (self.max_value - self.min_value);
        let new_normalized = (start_normalized + delta_normalized).clamp(0.0, 1.0);

        self.set_normalized_value(new_normalized);

        if let Some(ref callback) = self.on_change {
            callback(self.get_value());
        }
    }

    fn cursor(&mut self, _ctx: &Context, _p: Point, status: CursorTracking) -> bool {
        if !self.enabled {
            return false;
        }

        let mut state = self.state.write().unwrap();
        if *state == DialState::Dragging {
            return true;
        }

        match status {
            CursorTracking::Entering | CursorTracking::Hovering => {
                *state = DialState::Hover;
            }
            CursorTracking::Leaving => {
                *state = DialState::Normal;
            }
        }

        true
    }

    fn enable(&mut self, state: bool) {
        self.enabled = state;
        let mut dial_state = self.state.write().unwrap();
        if !state {
            *dial_state = DialState::Disabled;
        } else if *dial_state == DialState::Disabled {
            *dial_state = DialState::Normal;
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

/// Creates a dial.
pub fn dial() -> Dial {
    Dial::new()
}

/// Creates a dial with range.
pub fn dial_with_range(min: f64, max: f64) -> Dial {
    Dial::with_range(min, max)
}
