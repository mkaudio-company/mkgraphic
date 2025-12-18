//! Toggle switch element (iOS/Android style switches).

use std::any::Any;
use std::sync::RwLock;
use super::{Element, ViewLimits, ViewStretch};
use super::context::{BasicContext, Context};
use crate::support::point::Point;
use crate::support::color::Color;
use crate::support::theme::get_theme;
use crate::view::{MouseButton, MouseButtonKind, CursorTracking};

/// Switch state.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum SwitchState {
    #[default]
    Normal,
    Hover,
    Pressed,
    Disabled,
}

/// Callback type for switch changes.
pub type SwitchCallback = Box<dyn Fn(bool) + Send + Sync>;

/// A toggle switch element (similar to iOS/Android toggle switches).
pub struct SlideSwitch {
    on: RwLock<bool>,
    state: RwLock<SwitchState>,
    track_on_color: Color,
    track_off_color: Color,
    thumb_color: Color,
    width: f32,
    height: f32,
    enabled: bool,
    on_change: Option<SwitchCallback>,
    /// Animation progress (0.0 = off, 1.0 = on)
    animation_progress: RwLock<f32>,
}

impl SlideSwitch {
    /// Creates a new slide switch.
    pub fn new() -> Self {
        let theme = get_theme();
        Self {
            on: RwLock::new(false),
            state: RwLock::new(SwitchState::Normal),
            track_on_color: theme.indicator_bright_color,
            track_off_color: theme.frame_color,
            thumb_color: Color::new(1.0, 1.0, 1.0, 1.0),
            width: 44.0,
            height: 24.0,
            enabled: true,
            on_change: None,
            animation_progress: RwLock::new(0.0),
        }
    }

    /// Sets the initial on/off state.
    pub fn on(self, on: bool) -> Self {
        *self.on.write().unwrap() = on;
        *self.animation_progress.write().unwrap() = if on { 1.0 } else { 0.0 };
        self
    }

    /// Sets the track color when on.
    pub fn track_on_color(mut self, color: Color) -> Self {
        self.track_on_color = color;
        self
    }

    /// Sets the track color when off.
    pub fn track_off_color(mut self, color: Color) -> Self {
        self.track_off_color = color;
        self
    }

    /// Sets the thumb color.
    pub fn thumb_color(mut self, color: Color) -> Self {
        self.thumb_color = color;
        self
    }

    /// Sets the switch dimensions.
    pub fn size(mut self, width: f32, height: f32) -> Self {
        self.width = width;
        self.height = height;
        self
    }

    /// Sets the change callback.
    pub fn on_change<F: Fn(bool) + Send + Sync + 'static>(mut self, callback: F) -> Self {
        self.on_change = Some(Box::new(callback));
        self
    }

    /// Returns whether the switch is on.
    pub fn is_on(&self) -> bool {
        *self.on.read().unwrap()
    }

    /// Sets the on/off state.
    pub fn set_on(&self, on: bool) {
        *self.on.write().unwrap() = on;
        *self.animation_progress.write().unwrap() = if on { 1.0 } else { 0.0 };
    }

    /// Toggles the switch.
    pub fn toggle(&self) {
        let mut on = self.on.write().unwrap();
        *on = !*on;
        *self.animation_progress.write().unwrap() = if *on { 1.0 } else { 0.0 };
    }

    fn draw_track(&self, ctx: &Context) {
        let mut canvas = ctx.canvas.borrow_mut();
        let state = *self.state.read().unwrap();
        let progress = *self.animation_progress.read().unwrap();

        // Interpolate between off and on colors
        let track_color = Color::new(
            self.track_off_color.red + (self.track_on_color.red - self.track_off_color.red) * progress,
            self.track_off_color.green + (self.track_on_color.green - self.track_off_color.green) * progress,
            self.track_off_color.blue + (self.track_on_color.blue - self.track_off_color.blue) * progress,
            self.track_off_color.alpha + (self.track_on_color.alpha - self.track_off_color.alpha) * progress,
        );

        let color = match state {
            SwitchState::Normal => track_color,
            SwitchState::Hover => track_color.level(1.1),
            SwitchState::Pressed => track_color.level(0.9),
            SwitchState::Disabled => track_color.with_alpha(0.5),
        };

        let corner_radius = self.height / 2.0;
        canvas.fill_style(color);
        canvas.fill_round_rect(ctx.bounds, corner_radius);
    }

    fn draw_thumb(&self, ctx: &Context) {
        let mut canvas = ctx.canvas.borrow_mut();
        let state = *self.state.read().unwrap();
        let progress = *self.animation_progress.read().unwrap();

        let thumb_radius = (self.height - 4.0) / 2.0;
        let thumb_padding = 2.0;

        // Calculate thumb position
        let left_x = ctx.bounds.left + thumb_padding + thumb_radius;
        let right_x = ctx.bounds.right - thumb_padding - thumb_radius;
        let thumb_x = left_x + progress * (right_x - left_x);
        let thumb_y = ctx.bounds.center().y;

        let color = match state {
            SwitchState::Normal => self.thumb_color,
            SwitchState::Hover => self.thumb_color.level(0.95),
            SwitchState::Pressed => self.thumb_color.level(0.9),
            SwitchState::Disabled => self.thumb_color.with_alpha(0.7),
        };

        canvas.fill_style(color);
        canvas.begin_path();
        canvas.add_circle(crate::support::circle::Circle::new(
            Point::new(thumb_x, thumb_y),
            thumb_radius,
        ));
        canvas.fill();

        // Add subtle shadow/highlight
        if state != SwitchState::Disabled {
            canvas.stroke_style(Color::new(0.0, 0.0, 0.0, 0.1));
            canvas.line_width(1.0);
            canvas.begin_path();
            canvas.add_circle(crate::support::circle::Circle::new(
                Point::new(thumb_x, thumb_y),
                thumb_radius,
            ));
            canvas.stroke();
        }
    }
}

impl Default for SlideSwitch {
    fn default() -> Self {
        Self::new()
    }
}

impl Element for SlideSwitch {
    fn limits(&self, _ctx: &BasicContext) -> ViewLimits {
        ViewLimits::fixed(self.width, self.height)
    }

    fn stretch(&self) -> ViewStretch {
        ViewStretch::new(0.0, 0.0)
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
            *state = SwitchState::Pressed;
        } else {
            if *state == SwitchState::Pressed && ctx.bounds.contains(btn.pos) {
                drop(state);
                self.toggle();
                if let Some(ref callback) = self.on_change {
                    callback(self.is_on());
                }
                let mut state = self.state.write().unwrap();
                *state = SwitchState::Hover;
            } else {
                *state = if ctx.bounds.contains(btn.pos) {
                    SwitchState::Hover
                } else {
                    SwitchState::Normal
                };
            }
        }

        true
    }

    fn cursor(&mut self, _ctx: &Context, _p: Point, status: CursorTracking) -> bool {
        if !self.enabled {
            return false;
        }

        let mut state = self.state.write().unwrap();
        if *state == SwitchState::Pressed {
            return true;
        }

        match status {
            CursorTracking::Entering | CursorTracking::Hovering => {
                *state = SwitchState::Hover;
            }
            CursorTracking::Leaving => {
                *state = SwitchState::Normal;
            }
        }

        true
    }

    fn enable(&mut self, state: bool) {
        self.enabled = state;
        let mut switch_state = self.state.write().unwrap();
        if !state {
            *switch_state = SwitchState::Disabled;
        } else if *switch_state == SwitchState::Disabled {
            *switch_state = SwitchState::Normal;
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

/// Creates a toggle switch.
pub fn slide_switch() -> SlideSwitch {
    SlideSwitch::new()
}

/// Creates a toggle switch with initial state.
pub fn slide_switch_on(on: bool) -> SlideSwitch {
    SlideSwitch::new().on(on)
}
