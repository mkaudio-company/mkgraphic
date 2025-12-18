//! Thumbwheel element for fine value adjustment.

use std::any::Any;
use std::sync::RwLock;
use super::{Element, ViewLimits, ViewStretch};
use super::context::{BasicContext, Context};
use crate::support::point::Point;
use crate::support::color::Color;
use crate::support::theme::get_theme;
use crate::view::{MouseButton, MouseButtonKind, CursorTracking};

/// Thumbwheel orientation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ThumbwheelOrientation {
    #[default]
    Horizontal,
    Vertical,
}

/// Thumbwheel state.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ThumbwheelState {
    #[default]
    Normal,
    Hover,
    Dragging,
    Disabled,
}

/// Callback type for value changes.
pub type ThumbwheelCallback = Box<dyn Fn(f64) + Send + Sync>;

/// A thumbwheel element for fine value adjustment (like a mouse scroll wheel).
pub struct Thumbwheel {
    value: RwLock<f64>,
    min_value: f64,
    max_value: f64,
    step: f64,
    orientation: ThumbwheelOrientation,
    state: RwLock<ThumbwheelState>,
    background_color: Color,
    groove_color: Color,
    tick_color: Color,
    width: f32,
    height: f32,
    enabled: bool,
    on_change: Option<ThumbwheelCallback>,
    drag_start: RwLock<f32>,
    drag_start_value: RwLock<f64>,
}

impl Thumbwheel {
    /// Creates a new thumbwheel.
    pub fn new() -> Self {
        let theme = get_theme();
        Self {
            value: RwLock::new(0.0),
            min_value: 0.0,
            max_value: 100.0,
            step: 1.0,
            orientation: ThumbwheelOrientation::Horizontal,
            state: RwLock::new(ThumbwheelState::Normal),
            background_color: theme.frame_color,
            groove_color: theme.frame_color.level(0.7),
            tick_color: theme.label_font_color.with_alpha(0.5),
            width: 80.0,
            height: 24.0,
            enabled: true,
            on_change: None,
            drag_start: RwLock::new(0.0),
            drag_start_value: RwLock::new(0.0),
        }
    }

    /// Sets the range.
    pub fn range(mut self, min: f64, max: f64) -> Self {
        self.min_value = min;
        self.max_value = max;
        self
    }

    /// Sets the step increment.
    pub fn step(mut self, step: f64) -> Self {
        self.step = step;
        self
    }

    /// Sets the initial value.
    pub fn value(self, value: f64) -> Self {
        self.set_value(value);
        self
    }

    /// Sets the orientation.
    pub fn orientation(mut self, orientation: ThumbwheelOrientation) -> Self {
        self.orientation = orientation;
        if orientation == ThumbwheelOrientation::Vertical {
            std::mem::swap(&mut self.width, &mut self.height);
        }
        self
    }

    /// Sets the dimensions.
    pub fn size(mut self, width: f32, height: f32) -> Self {
        self.width = width;
        self.height = height;
        self
    }

    /// Sets the background color.
    pub fn background_color(mut self, color: Color) -> Self {
        self.background_color = color;
        self
    }

    /// Sets the change callback.
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
        let stepped = (clamped / self.step).round() * self.step;
        *self.value.write().unwrap() = stepped.clamp(self.min_value, self.max_value);
    }

    fn draw_background(&self, ctx: &Context) {
        let mut canvas = ctx.canvas.borrow_mut();
        let state = *self.state.read().unwrap();

        let color = match state {
            ThumbwheelState::Normal => self.background_color,
            ThumbwheelState::Hover => self.background_color.level(1.1),
            ThumbwheelState::Dragging => self.background_color.level(1.2),
            ThumbwheelState::Disabled => self.background_color.with_alpha(0.5),
        };

        canvas.fill_style(color);
        canvas.fill_round_rect(ctx.bounds, 4.0);

        // Inner groove
        let groove_inset = 4.0;
        let groove_rect = ctx.bounds.inset(groove_inset, groove_inset);
        canvas.fill_style(self.groove_color);
        canvas.fill_round_rect(groove_rect, 2.0);
    }

    fn draw_ticks(&self, ctx: &Context) {
        let mut canvas = ctx.canvas.borrow_mut();
        let state = *self.state.read().unwrap();

        let tick_color = if state == ThumbwheelState::Disabled {
            self.tick_color.with_alpha(0.3)
        } else {
            self.tick_color
        };

        canvas.stroke_style(tick_color);
        canvas.line_width(1.0);

        // Draw tick marks that move with the value
        let value = self.get_value();
        let range = self.max_value - self.min_value;
        let offset = if range > 0.0 {
            ((value - self.min_value) / range * 20.0) % 10.0
        } else {
            0.0
        } as f32;

        match self.orientation {
            ThumbwheelOrientation::Horizontal => {
                let tick_spacing = 10.0;
                let start_x = ctx.bounds.left + 6.0 - offset;
                let mut x = start_x;

                while x < ctx.bounds.right - 6.0 {
                    if x >= ctx.bounds.left + 6.0 {
                        canvas.begin_path();
                        canvas.move_to(Point::new(x, ctx.bounds.top + 6.0));
                        canvas.line_to(Point::new(x, ctx.bounds.bottom - 6.0));
                        canvas.stroke();
                    }
                    x += tick_spacing;
                }
            }
            ThumbwheelOrientation::Vertical => {
                let tick_spacing = 10.0;
                let start_y = ctx.bounds.top + 6.0 + offset;
                let mut y = start_y;

                while y < ctx.bounds.bottom - 6.0 {
                    if y >= ctx.bounds.top + 6.0 {
                        canvas.begin_path();
                        canvas.move_to(Point::new(ctx.bounds.left + 6.0, y));
                        canvas.line_to(Point::new(ctx.bounds.right - 6.0, y));
                        canvas.stroke();
                    }
                    y += tick_spacing;
                }
            }
        }
    }
}

impl Default for Thumbwheel {
    fn default() -> Self {
        Self::new()
    }
}

impl Element for Thumbwheel {
    fn limits(&self, _ctx: &BasicContext) -> ViewLimits {
        ViewLimits::fixed(self.width, self.height)
    }

    fn stretch(&self) -> ViewStretch {
        ViewStretch::new(0.0, 0.0)
    }

    fn draw(&self, ctx: &Context) {
        self.draw_background(ctx);
        self.draw_ticks(ctx);
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
            *state = ThumbwheelState::Dragging;
            match self.orientation {
                ThumbwheelOrientation::Horizontal => {
                    *self.drag_start.write().unwrap() = btn.pos.x;
                }
                ThumbwheelOrientation::Vertical => {
                    *self.drag_start.write().unwrap() = btn.pos.y;
                }
            }
            *self.drag_start_value.write().unwrap() = self.get_value();
        } else {
            *state = if ctx.bounds.contains(btn.pos) {
                ThumbwheelState::Hover
            } else {
                ThumbwheelState::Normal
            };
        }

        true
    }

    fn drag(&mut self, _ctx: &Context, btn: MouseButton) {
        if !self.enabled {
            return;
        }

        let drag_start = *self.drag_start.read().unwrap();
        let start_value = *self.drag_start_value.read().unwrap();

        let delta = match self.orientation {
            ThumbwheelOrientation::Horizontal => btn.pos.x - drag_start,
            ThumbwheelOrientation::Vertical => drag_start - btn.pos.y,
        };

        let sensitivity = (self.max_value - self.min_value) / 200.0;
        let new_value = start_value + delta as f64 * sensitivity;
        self.set_value(new_value);

        if let Some(ref callback) = self.on_change {
            callback(self.get_value());
        }
    }

    fn scroll(&mut self, _ctx: &Context, dir: Point, _p: Point) -> bool {
        if !self.enabled {
            return false;
        }

        let delta = match self.orientation {
            ThumbwheelOrientation::Horizontal => dir.x,
            ThumbwheelOrientation::Vertical => dir.y,
        };

        let new_value = self.get_value() + delta as f64 * self.step;
        self.set_value(new_value);

        if let Some(ref callback) = self.on_change {
            callback(self.get_value());
        }

        true
    }

    fn cursor(&mut self, ctx: &Context, _p: Point, status: CursorTracking) -> bool {
        if !self.enabled {
            return false;
        }

        let mut state = self.state.write().unwrap();
        if *state == ThumbwheelState::Dragging {
            return true;
        }

        match status {
            CursorTracking::Entering | CursorTracking::Hovering => {
                *state = ThumbwheelState::Hover;
            }
            CursorTracking::Leaving => {
                *state = ThumbwheelState::Normal;
            }
        }

        true
    }

    fn enable(&mut self, state: bool) {
        self.enabled = state;
        let mut wheel_state = self.state.write().unwrap();
        if !state {
            *wheel_state = ThumbwheelState::Disabled;
        } else if *wheel_state == ThumbwheelState::Disabled {
            *wheel_state = ThumbwheelState::Normal;
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

/// Creates a thumbwheel.
pub fn thumbwheel() -> Thumbwheel {
    Thumbwheel::new()
}

/// Creates a vertical thumbwheel.
pub fn vthumbwheel() -> Thumbwheel {
    Thumbwheel::new().orientation(ThumbwheelOrientation::Vertical)
}
