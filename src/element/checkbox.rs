//! Checkbox and radio button elements.

use std::any::Any;
use std::sync::RwLock;
use super::{Element, ViewLimits, ViewStretch};
use super::context::{BasicContext, Context};
use crate::support::point::Point;
use crate::support::rect::Rect;
use crate::support::color::Color;
use crate::support::theme::get_theme;
use crate::view::{MouseButton, MouseButtonKind, CursorTracking};

/// Checkbox state.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum CheckboxState {
    #[default]
    Normal,
    Hover,
    Pressed,
    Disabled,
}

/// Callback type for checkbox changes.
pub type CheckCallback = Box<dyn Fn(bool) + Send + Sync>;

/// A checkbox element for boolean values.
pub struct Checkbox {
    label: String,
    checked: RwLock<bool>,
    state: RwLock<CheckboxState>,
    box_color: Color,
    check_color: Color,
    text_color: Color,
    box_size: f32,
    corner_radius: f32,
    enabled: bool,
    on_change: Option<CheckCallback>,
}

impl Checkbox {
    /// Creates a new checkbox with optional label.
    pub fn new(label: impl Into<String>) -> Self {
        let theme = get_theme();
        Self {
            label: label.into(),
            checked: RwLock::new(false),
            state: RwLock::new(CheckboxState::Normal),
            box_color: theme.frame_color,
            check_color: theme.indicator_bright_color,
            text_color: theme.label_font_color,
            box_size: 18.0,
            corner_radius: 3.0,
            enabled: true,
            on_change: None,
        }
    }

    /// Sets the initial checked state.
    pub fn checked(self, checked: bool) -> Self {
        *self.checked.write().unwrap() = checked;
        self
    }

    /// Sets the box color.
    pub fn box_color(mut self, color: Color) -> Self {
        self.box_color = color;
        self
    }

    /// Sets the check mark color.
    pub fn check_color(mut self, color: Color) -> Self {
        self.check_color = color;
        self
    }

    /// Sets the text color.
    pub fn text_color(mut self, color: Color) -> Self {
        self.text_color = color;
        self
    }

    /// Sets the box size.
    pub fn box_size(mut self, size: f32) -> Self {
        self.box_size = size;
        self
    }

    /// Sets the change callback.
    pub fn on_change<F: Fn(bool) + Send + Sync + 'static>(mut self, callback: F) -> Self {
        self.on_change = Some(Box::new(callback));
        self
    }

    /// Returns whether the checkbox is checked.
    pub fn is_checked(&self) -> bool {
        *self.checked.read().unwrap()
    }

    /// Sets the checked state.
    pub fn set_checked(&self, checked: bool) {
        *self.checked.write().unwrap() = checked;
    }

    /// Toggles the checked state.
    pub fn toggle(&self) {
        let mut checked = self.checked.write().unwrap();
        *checked = !*checked;
    }

    fn box_rect(&self, bounds: &Rect) -> Rect {
        Rect::new(
            bounds.left,
            bounds.top + (bounds.height() - self.box_size) / 2.0,
            bounds.left + self.box_size,
            bounds.top + (bounds.height() - self.box_size) / 2.0 + self.box_size,
        )
    }

    fn draw_box(&self, ctx: &Context) {
        let mut canvas = ctx.canvas.borrow_mut();
        let state = *self.state.read().unwrap();
        let box_rect = self.box_rect(&ctx.bounds);

        let color = match state {
            CheckboxState::Normal => self.box_color,
            CheckboxState::Hover => self.box_color.level(1.2),
            CheckboxState::Pressed => self.box_color.level(0.8),
            CheckboxState::Disabled => self.box_color.with_alpha(0.5),
        };

        canvas.fill_style(color);
        canvas.fill_round_rect(box_rect, self.corner_radius);
    }

    fn draw_check(&self, ctx: &Context) {
        if !self.is_checked() {
            return;
        }

        let mut canvas = ctx.canvas.borrow_mut();
        let box_rect = self.box_rect(&ctx.bounds);
        let state = *self.state.read().unwrap();

        let color = if state == CheckboxState::Disabled {
            self.check_color.with_alpha(0.5)
        } else {
            self.check_color
        };

        // Draw checkmark
        let padding = self.box_size * 0.25;
        let x1 = box_rect.left + padding;
        let y1 = box_rect.center().y;
        let x2 = box_rect.center().x - padding * 0.3;
        let y2 = box_rect.bottom - padding;
        let x3 = box_rect.right - padding;
        let y3 = box_rect.top + padding;

        canvas.stroke_style(color);
        canvas.line_width(2.0);
        canvas.begin_path();
        canvas.move_to(Point::new(x1, y1));
        canvas.line_to(Point::new(x2, y2));
        canvas.line_to(Point::new(x3, y3));
        canvas.stroke();
    }

    fn draw_label(&self, ctx: &Context) {
        if self.label.is_empty() {
            return;
        }

        let mut canvas = ctx.canvas.borrow_mut();
        let theme = get_theme();
        let state = *self.state.read().unwrap();

        let color = if state == CheckboxState::Disabled {
            self.text_color.with_alpha(0.5)
        } else {
            self.text_color
        };

        canvas.fill_style(color);
        canvas.font_size(theme.label_font_size);

        let x = ctx.bounds.left + self.box_size + 8.0;
        let y = ctx.bounds.center().y + theme.label_font_size * 0.35;

        canvas.fill_text(&self.label, Point::new(x, y));
    }
}

impl Element for Checkbox {
    fn limits(&self, _ctx: &BasicContext) -> ViewLimits {
        let theme = get_theme();
        let text_width = if self.label.is_empty() {
            0.0
        } else {
            self.label.len() as f32 * theme.label_font_size * 0.6 + 8.0
        };
        let width = self.box_size + text_width;
        let height = self.box_size.max(theme.label_font_size * 1.2);
        ViewLimits::fixed(width, height)
    }

    fn stretch(&self) -> ViewStretch {
        ViewStretch::new(0.0, 0.0)
    }

    fn draw(&self, ctx: &Context) {
        self.draw_box(ctx);
        self.draw_check(ctx);
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

    fn handle_click(&self, ctx: &Context, btn: MouseButton) -> bool {
        if !self.enabled || btn.button != MouseButtonKind::Left {
            return false;
        }

        let mut state = self.state.write().unwrap();
        if btn.down {
            *state = CheckboxState::Pressed;
        } else {
            if *state == CheckboxState::Pressed && ctx.bounds.contains(btn.pos) {
                drop(state);
                self.toggle();
                if let Some(ref callback) = self.on_change {
                    callback(self.is_checked());
                }
                let mut state = self.state.write().unwrap();
                *state = CheckboxState::Hover;
            } else {
                *state = if ctx.bounds.contains(btn.pos) {
                    CheckboxState::Hover
                } else {
                    CheckboxState::Normal
                };
            }
        }

        true
    }

    fn cursor(&mut self, ctx: &Context, _p: Point, status: CursorTracking) -> bool {
        if !self.enabled {
            return false;
        }

        let mut state = self.state.write().unwrap();
        if *state == CheckboxState::Pressed {
            return true;
        }

        match status {
            CursorTracking::Entering | CursorTracking::Hovering => {
                *state = CheckboxState::Hover;
            }
            CursorTracking::Leaving => {
                *state = CheckboxState::Normal;
            }
        }

        true
    }

    fn enable(&mut self, state: bool) {
        self.enabled = state;
        let mut checkbox_state = self.state.write().unwrap();
        if !state {
            *checkbox_state = CheckboxState::Disabled;
        } else if *checkbox_state == CheckboxState::Disabled {
            *checkbox_state = CheckboxState::Normal;
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

/// A radio button element for selecting one option from a group.
pub struct RadioButton {
    label: String,
    selected: RwLock<bool>,
    state: RwLock<CheckboxState>,
    circle_color: Color,
    indicator_color: Color,
    text_color: Color,
    circle_size: f32,
    enabled: bool,
    on_select: Option<Box<dyn Fn() + Send + Sync>>,
}

impl RadioButton {
    /// Creates a new radio button with label.
    pub fn new(label: impl Into<String>) -> Self {
        let theme = get_theme();
        Self {
            label: label.into(),
            selected: RwLock::new(false),
            state: RwLock::new(CheckboxState::Normal),
            circle_color: theme.frame_color,
            indicator_color: theme.indicator_bright_color,
            text_color: theme.label_font_color,
            circle_size: 18.0,
            enabled: true,
            on_select: None,
        }
    }

    /// Sets the initial selected state.
    pub fn selected(self, selected: bool) -> Self {
        *self.selected.write().unwrap() = selected;
        self
    }

    /// Sets the circle color.
    pub fn circle_color(mut self, color: Color) -> Self {
        self.circle_color = color;
        self
    }

    /// Sets the indicator color.
    pub fn indicator_color(mut self, color: Color) -> Self {
        self.indicator_color = color;
        self
    }

    /// Sets the text color.
    pub fn text_color(mut self, color: Color) -> Self {
        self.text_color = color;
        self
    }

    /// Sets the select callback.
    pub fn on_select<F: Fn() + Send + Sync + 'static>(mut self, callback: F) -> Self {
        self.on_select = Some(Box::new(callback));
        self
    }

    /// Returns whether the radio button is selected.
    pub fn is_selected(&self) -> bool {
        *self.selected.read().unwrap()
    }

    /// Sets the selected state.
    pub fn set_selected(&self, selected: bool) {
        *self.selected.write().unwrap() = selected;
    }

    fn draw_circle(&self, ctx: &Context) {
        let mut canvas = ctx.canvas.borrow_mut();
        let state = *self.state.read().unwrap();

        let center = Point::new(
            ctx.bounds.left + self.circle_size / 2.0,
            ctx.bounds.center().y,
        );

        let color = match state {
            CheckboxState::Normal => self.circle_color,
            CheckboxState::Hover => self.circle_color.level(1.2),
            CheckboxState::Pressed => self.circle_color.level(0.8),
            CheckboxState::Disabled => self.circle_color.with_alpha(0.5),
        };

        canvas.fill_style(color);
        canvas.begin_path();
        canvas.add_circle(crate::support::circle::Circle::new(center, self.circle_size / 2.0));
        canvas.fill();
    }

    fn draw_indicator(&self, ctx: &Context) {
        if !self.is_selected() {
            return;
        }

        let mut canvas = ctx.canvas.borrow_mut();
        let state = *self.state.read().unwrap();

        let center = Point::new(
            ctx.bounds.left + self.circle_size / 2.0,
            ctx.bounds.center().y,
        );

        let color = if state == CheckboxState::Disabled {
            self.indicator_color.with_alpha(0.5)
        } else {
            self.indicator_color
        };

        canvas.fill_style(color);
        canvas.begin_path();
        canvas.add_circle(crate::support::circle::Circle::new(center, self.circle_size / 4.0));
        canvas.fill();
    }

    fn draw_label(&self, ctx: &Context) {
        if self.label.is_empty() {
            return;
        }

        let mut canvas = ctx.canvas.borrow_mut();
        let theme = get_theme();
        let state = *self.state.read().unwrap();

        let color = if state == CheckboxState::Disabled {
            self.text_color.with_alpha(0.5)
        } else {
            self.text_color
        };

        canvas.fill_style(color);
        canvas.font_size(theme.label_font_size);

        let x = ctx.bounds.left + self.circle_size + 8.0;
        let y = ctx.bounds.center().y + theme.label_font_size * 0.35;

        canvas.fill_text(&self.label, Point::new(x, y));
    }
}

impl Element for RadioButton {
    fn limits(&self, _ctx: &BasicContext) -> ViewLimits {
        let theme = get_theme();
        let text_width = if self.label.is_empty() {
            0.0
        } else {
            self.label.len() as f32 * theme.label_font_size * 0.6 + 8.0
        };
        let width = self.circle_size + text_width;
        let height = self.circle_size.max(theme.label_font_size * 1.2);
        ViewLimits::fixed(width, height)
    }

    fn stretch(&self) -> ViewStretch {
        ViewStretch::new(0.0, 0.0)
    }

    fn draw(&self, ctx: &Context) {
        self.draw_circle(ctx);
        self.draw_indicator(ctx);
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

    fn handle_click(&self, ctx: &Context, btn: MouseButton) -> bool {
        if !self.enabled || btn.button != MouseButtonKind::Left {
            return false;
        }

        let mut state = self.state.write().unwrap();
        if btn.down {
            *state = CheckboxState::Pressed;
        } else {
            if *state == CheckboxState::Pressed && ctx.bounds.contains(btn.pos) {
                drop(state);
                // Radio buttons can only be selected, not deselected by clicking
                if !self.is_selected() {
                    self.set_selected(true);
                    if let Some(ref callback) = self.on_select {
                        callback();
                    }
                }
                let mut state = self.state.write().unwrap();
                *state = CheckboxState::Hover;
            } else {
                *state = if ctx.bounds.contains(btn.pos) {
                    CheckboxState::Hover
                } else {
                    CheckboxState::Normal
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
        if *state == CheckboxState::Pressed {
            return true;
        }

        match status {
            CursorTracking::Entering | CursorTracking::Hovering => {
                *state = CheckboxState::Hover;
            }
            CursorTracking::Leaving => {
                *state = CheckboxState::Normal;
            }
        }

        true
    }

    fn enable(&mut self, state: bool) {
        self.enabled = state;
        let mut radio_state = self.state.write().unwrap();
        if !state {
            *radio_state = CheckboxState::Disabled;
        } else if *radio_state == CheckboxState::Disabled {
            *radio_state = CheckboxState::Normal;
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

/// Creates a checkbox.
pub fn checkbox(label: impl Into<String>) -> Checkbox {
    Checkbox::new(label)
}

/// Creates a radio button.
pub fn radio_button(label: impl Into<String>) -> RadioButton {
    RadioButton::new(label)
}
