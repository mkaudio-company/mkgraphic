//! Button elements for user interaction.

use std::any::Any;
use std::sync::RwLock;
use super::{Element, ViewLimits};
use super::context::{BasicContext, Context};
use crate::support::point::Point;
use crate::support::rect::Rect;
use crate::support::color::Color;
use crate::support::canvas::CornerRadii;
use crate::support::theme::get_theme;
use crate::view::{MouseButton, CursorTracking};

/// Button state.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ButtonState {
    #[default]
    Normal,
    Hover,
    Pressed,
    Disabled,
}

/// Callback type for button clicks.
pub type ClickCallback = Box<dyn Fn() + Send + Sync>;

/// A basic button element.
pub struct BasicButton {
    label: String,
    state: RwLock<ButtonState>,
    body_color: Color,
    text_color: Color,
    corner_radius: f32,
    enabled: bool,
    on_click: Option<ClickCallback>,
    value: bool, // For toggle buttons
}

impl BasicButton {
    /// Creates a new button with the given label.
    pub fn new(label: impl Into<String>) -> Self {
        let theme = get_theme();
        Self {
            label: label.into(),
            state: RwLock::new(ButtonState::Normal),
            body_color: theme.default_button_color,
            text_color: theme.label_font_color,
            corner_radius: theme.button_corner_radius,
            enabled: true,
            on_click: None,
            value: false,
        }
    }

    /// Sets the click callback.
    pub fn on_click<F: Fn() + Send + Sync + 'static>(mut self, callback: F) -> Self {
        self.on_click = Some(Box::new(callback));
        self
    }

    /// Sets the body color.
    pub fn with_body_color(mut self, color: Color) -> Self {
        self.body_color = color;
        self
    }

    /// Sets the text color.
    pub fn with_text_color(mut self, color: Color) -> Self {
        self.text_color = color;
        self
    }

    /// Sets the corner radius.
    pub fn with_corner_radius(mut self, radius: f32) -> Self {
        self.corner_radius = radius;
        self
    }

    /// Returns the label.
    pub fn label(&self) -> &str {
        &self.label
    }

    /// Sets the label.
    pub fn set_label(&mut self, label: impl Into<String>) {
        self.label = label.into();
    }

    /// Returns the current state.
    pub fn state(&self) -> ButtonState {
        *self.state.read().unwrap()
    }

    /// Returns whether the button is pressed (for toggle buttons).
    pub fn value(&self) -> bool {
        self.value
    }

    /// Sets the value (for toggle buttons).
    pub fn set_value(&mut self, value: bool) {
        self.value = value;
    }

    fn draw_background(&self, ctx: &Context) {
        let state = *self.state.read().unwrap();
        let color = match state {
            ButtonState::Normal => self.body_color,
            ButtonState::Hover => self.body_color.level(1.2),
            ButtonState::Pressed => self.body_color.level(0.8),
            ButtonState::Disabled => self.body_color.with_alpha(0.5),
        };

        let mut canvas = ctx.canvas.borrow_mut();
        canvas.fill_style(color);
        canvas.fill_round_rect(ctx.bounds, self.corner_radius);
    }

    fn draw_label(&self, ctx: &Context) {
        let color = if self.enabled {
            self.text_color
        } else {
            self.text_color.with_alpha(0.5)
        };

        let theme = get_theme();
        let mut canvas = ctx.canvas.borrow_mut();
        canvas.fill_style(color);
        canvas.font_size(theme.label_font_size);

        // Center the text
        let text_width = self.label.len() as f32 * theme.label_font_size * 0.6;
        let text_height = theme.label_font_size;
        let x = ctx.bounds.left + (ctx.bounds.width() - text_width) / 2.0;
        let y = ctx.bounds.top + (ctx.bounds.height() - text_height) / 2.0 + text_height * 0.8;

        canvas.fill_text(&self.label, Point::new(x, y));
    }
}

impl Element for BasicButton {
    fn limits(&self, ctx: &BasicContext) -> ViewLimits {
        let theme = get_theme();
        let text_width = self.label.len() as f32 * theme.label_font_size * 0.6;
        let text_height = theme.label_font_size * 1.2;

        let margin = &theme.button_margin;
        let width = text_width + margin.left + margin.right;
        let height = text_height + margin.top + margin.bottom;

        ViewLimits::fixed(width, height)
    }

    fn stretch(&self) -> super::ViewStretch {
        // Button has fixed size, so no stretch
        super::ViewStretch::new(0.0, 0.0)
    }

    fn draw(&self, ctx: &Context) {
        self.draw_background(ctx);
        self.draw_label(ctx);
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

    fn click(&mut self, ctx: &Context, btn: MouseButton) -> bool {
        self.handle_click(ctx, btn)
    }

    fn handle_click(&self, ctx: &Context, btn: MouseButton) -> bool {
        if !self.enabled || btn.button != crate::view::MouseButtonKind::Left {
            return false;
        }

        let mut state = self.state.write().unwrap();
        if btn.down {
            *state = ButtonState::Pressed;
        } else {
            if *state == ButtonState::Pressed {
                // Button was clicked - call callback outside of lock
                drop(state);
                if let Some(ref callback) = self.on_click {
                    callback();
                }
                let mut state = self.state.write().unwrap();
                *state = if ctx.bounds.contains(btn.pos) {
                    ButtonState::Hover
                } else {
                    ButtonState::Normal
                };
            } else {
                *state = if ctx.bounds.contains(btn.pos) {
                    ButtonState::Hover
                } else {
                    ButtonState::Normal
                };
            }
        }

        true
    }

    fn cursor(&mut self, ctx: &Context, p: Point, status: CursorTracking) -> bool {
        if !self.enabled {
            return false;
        }

        let mut state = self.state.write().unwrap();
        match status {
            CursorTracking::Entering | CursorTracking::Hovering => {
                if *state != ButtonState::Pressed {
                    *state = ButtonState::Hover;
                }
                // Would set cursor to hand
            }
            CursorTracking::Leaving => {
                if *state != ButtonState::Pressed {
                    *state = ButtonState::Normal;
                }
            }
        }

        true
    }

    fn enable(&mut self, state: bool) {
        self.enabled = state;
        let mut btn_state = self.state.write().unwrap();
        if !state {
            *btn_state = ButtonState::Disabled;
        } else if *btn_state == ButtonState::Disabled {
            *btn_state = ButtonState::Normal;
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

/// A toggle button that maintains its state.
pub struct ToggleButton {
    inner: BasicButton,
    active_color: Color,
}

impl ToggleButton {
    /// Creates a new toggle button.
    pub fn new(label: impl Into<String>) -> Self {
        let theme = get_theme();
        Self {
            inner: BasicButton::new(label),
            active_color: theme.indicator_bright_color,
        }
    }

    /// Sets the active color.
    pub fn with_active_color(mut self, color: Color) -> Self {
        self.active_color = color;
        self
    }

    /// Returns whether the button is toggled on.
    pub fn value(&self) -> bool {
        self.inner.value
    }

    /// Sets the toggle state.
    pub fn set_value(&mut self, value: bool) {
        self.inner.value = value;
    }

    /// Toggles the state.
    pub fn toggle(&mut self) {
        self.inner.value = !self.inner.value;
    }
}

impl Element for ToggleButton {
    fn limits(&self, ctx: &BasicContext) -> ViewLimits {
        self.inner.limits(ctx)
    }

    fn draw(&self, ctx: &Context) {
        // Modify color if toggled
        let original_color = self.inner.body_color;
        if self.inner.value {
            // Would need interior mutability here
            // For now, just draw with current color
        }
        self.inner.draw(ctx);
    }

    fn wants_control(&self) -> bool {
        self.inner.wants_control()
    }

    fn click(&mut self, ctx: &Context, btn: MouseButton) -> bool {
        if !self.inner.enabled || btn.button != crate::view::MouseButtonKind::Left {
            return false;
        }

        let mut state = self.inner.state.write().unwrap();
        if btn.down {
            *state = ButtonState::Pressed;
        } else {
            let was_pressed = *state == ButtonState::Pressed;
            if was_pressed && ctx.bounds.contains(btn.pos) {
                // Toggle on release
                drop(state);
                self.toggle();
                let mut state = self.inner.state.write().unwrap();
                *state = ButtonState::Hover;
            } else {
                *state = if ctx.bounds.contains(btn.pos) {
                    ButtonState::Hover
                } else {
                    ButtonState::Normal
                };
            }
        }

        true
    }

    fn cursor(&mut self, ctx: &Context, p: Point, status: CursorTracking) -> bool {
        self.inner.cursor(ctx, p, status)
    }

    fn enable(&mut self, state: bool) {
        self.inner.enable(state);
    }

    fn is_enabled(&self) -> bool {
        self.inner.is_enabled()
    }

    fn as_any(&self) -> &dyn Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }
}

// Convenience functions

/// Creates a momentary button.
pub fn button(label: impl Into<String>) -> BasicButton {
    BasicButton::new(label)
}

/// Creates a toggle button.
pub fn toggle_button(label: impl Into<String>) -> ToggleButton {
    ToggleButton::new(label)
}

/// Draws a button background (utility function).
pub fn draw_button_base(
    ctx: &Context,
    bounds: Rect,
    color: Color,
    enabled: bool,
    corner_radii: CornerRadii,
) {
    let actual_color = if enabled {
        color
    } else {
        color.with_alpha(color.alpha * 0.5)
    };

    let mut canvas = ctx.canvas.borrow_mut();
    canvas.fill_style(actual_color);
    canvas.begin_path();
    canvas.add_round_rect_varying(bounds, corner_radii);
    canvas.fill();
}
