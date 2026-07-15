//! Text input elements.

use super::context::{BasicContext, Context};
use super::{Element, FocusRequest, ViewLimits, ViewStretch};
use crate::support::color::Color;
use crate::support::point::Point;
use crate::support::rect::Rect;
use crate::support::theme::get_theme;
use crate::view::{CursorTracking, KeyCode, KeyInfo, MouseButton, MouseButtonKind, TextInfo};
use std::any::Any;
use std::sync::RwLock;

/// Text box state.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum TextBoxState {
    #[default]
    Idle,
    Hover,
    Focused,
    Disabled,
}

/// Callback type for text changes.
pub type TextChangeCallback = Box<dyn Fn(&str) + Send + Sync>;
/// Callback type for enter key.
pub type EnterCallback = Box<dyn Fn(&str) + Send + Sync>;

/// A text input element. Single-line by default; content only spans
/// multiple visual rows if a caller's own `on_enter` callback is wired
/// up so that Shift+Enter can insert a newline (see `handle_key`) --
/// there's no automatic word-wrapping.
pub struct TextBox {
    text: RwLock<String>,
    placeholder: String,
    state: RwLock<TextBoxState>,
    cursor_pos: RwLock<usize>,
    selection_start: RwLock<Option<usize>>,
    background_color: Color,
    text_color: Color,
    placeholder_color: Color,
    highlight_color: Color,
    caret_color: Color,
    font_size: f32,
    width: f32,
    height: f32,
    padding: f32,
    corner_radius: f32,
    enabled: bool,
    password_mode: bool,
    on_change: Option<TextChangeCallback>,
    on_enter: Option<EnterCallback>,
    scroll_offset: RwLock<f32>,
}

impl TextBox {
    /// Creates a new text box.
    pub fn new() -> Self {
        let theme = get_theme();
        Self {
            text: RwLock::new(String::new()),
            placeholder: String::new(),
            state: RwLock::new(TextBoxState::Idle),
            cursor_pos: RwLock::new(0),
            selection_start: RwLock::new(None),
            background_color: theme.input_box_color,
            text_color: theme.text_box_font_color,
            placeholder_color: theme.text_box_idle_color,
            highlight_color: theme.text_box_hilite_color,
            caret_color: theme.text_box_caret_color,
            font_size: theme.text_box_font_size,
            width: 150.0,
            height: theme.text_box_font_size * 2.0,
            padding: 8.0,
            corner_radius: 4.0,
            enabled: true,
            password_mode: false,
            on_change: None,
            on_enter: None,
            scroll_offset: RwLock::new(0.0),
        }
    }

    /// Sets the initial text.
    pub fn text(self, text: impl Into<String>) -> Self {
        let s: String = text.into();
        let len = s.len();
        *self.text.write().unwrap() = s;
        *self.cursor_pos.write().unwrap() = len;
        self
    }

    /// Sets the placeholder text.
    pub fn placeholder(mut self, placeholder: impl Into<String>) -> Self {
        self.placeholder = placeholder.into();
        self
    }

    /// Sets the width.
    pub fn width(mut self, width: f32) -> Self {
        self.width = width;
        self
    }

    /// Sets the height. Useful for growing past the default single-line
    /// height so multiple wrapped/inserted lines (see `on_enter`'s
    /// Shift+Enter handling) are actually visible at once instead of just
    /// scrolling past the box's own clip region.
    pub fn height(mut self, height: f32) -> Self {
        self.height = height;
        self
    }

    /// Sets password mode (displays dots instead of text).
    pub fn password(mut self, password: bool) -> Self {
        self.password_mode = password;
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

    /// Sets the change callback.
    pub fn on_change<F: Fn(&str) + Send + Sync + 'static>(mut self, callback: F) -> Self {
        self.on_change = Some(Box::new(callback));
        self
    }

    /// Sets the enter callback.
    pub fn on_enter<F: Fn(&str) + Send + Sync + 'static>(mut self, callback: F) -> Self {
        self.on_enter = Some(Box::new(callback));
        self
    }

    /// Returns the current text.
    pub fn get_text(&self) -> String {
        self.text.read().unwrap().clone()
    }

    /// Sets the text.
    pub fn set_text(&self, text: impl Into<String>) {
        let s: String = text.into();
        let len = s.len();
        *self.text.write().unwrap() = s;
        *self.cursor_pos.write().unwrap() = len;
        *self.selection_start.write().unwrap() = None;
    }

    /// Returns the display text (masked if password mode).
    fn display_text(&self) -> String {
        let text = self.text.read().unwrap();
        if self.password_mode {
            "•".repeat(text.chars().count())
        } else {
            text.clone()
        }
    }

    /// Vertical spacing between rows once `text` contains a newline (see
    /// `Shift+Enter` in `handle_key`). Single-line content never uses
    /// this -- it stays vertically centered exactly as before.
    fn line_height(&self) -> f32 {
        self.font_size * 1.3
    }

    /// Splits a flat char-index cursor position into (row, col) against a
    /// possibly multi-line `text`, for caret/selection placement.
    fn row_col_for_pos(text: &str, pos: usize) -> (usize, usize) {
        let mut row = 0;
        let mut col = 0;
        for (i, c) in text.chars().enumerate() {
            if i >= pos {
                break;
            }
            if c == '\n' {
                row += 1;
                col = 0;
            } else {
                col += 1;
            }
        }
        (row, col)
    }

    /// (row_top, row_bottom, baseline_y) for one row of multi-line text,
    /// matching the row layout `draw_text`'s multi-line branch uses.
    fn multiline_row_bounds(&self, ctx: &Context, row: usize) -> (f32, f32, f32) {
        let line_height = self.line_height();
        let row_top = ctx.bounds.top + self.padding + row as f32 * line_height;
        (
            row_top,
            row_top + line_height,
            row_top + self.font_size * 0.85,
        )
    }

    /// Inserts text at cursor position.
    fn insert_text(&self, s: &str) {
        let mut text = self.text.write().unwrap();
        let mut cursor_pos = self.cursor_pos.write().unwrap();
        let mut selection_start = self.selection_start.write().unwrap();

        // Delete selection if any
        if let Some(sel_start) = *selection_start {
            let start = sel_start.min(*cursor_pos);
            let end = sel_start.max(*cursor_pos);

            // Find byte indices
            let start_byte = text
                .char_indices()
                .nth(start)
                .map(|(i, _)| i)
                .unwrap_or(text.len());
            let end_byte = text
                .char_indices()
                .nth(end)
                .map(|(i, _)| i)
                .unwrap_or(text.len());

            text.replace_range(start_byte..end_byte, "");
            *cursor_pos = start;
            *selection_start = None;
        }

        // Insert new text
        let byte_pos = text
            .char_indices()
            .nth(*cursor_pos)
            .map(|(i, _)| i)
            .unwrap_or(text.len());
        text.insert_str(byte_pos, s);
        *cursor_pos += s.chars().count();
    }

    /// Deletes character before cursor.
    fn delete_backward(&self) {
        let mut text = self.text.write().unwrap();
        let mut cursor_pos = self.cursor_pos.write().unwrap();
        let mut selection_start = self.selection_start.write().unwrap();

        if let Some(sel_start) = *selection_start {
            // Delete selection
            let start = sel_start.min(*cursor_pos);
            let end = sel_start.max(*cursor_pos);

            let start_byte = text
                .char_indices()
                .nth(start)
                .map(|(i, _)| i)
                .unwrap_or(text.len());
            let end_byte = text
                .char_indices()
                .nth(end)
                .map(|(i, _)| i)
                .unwrap_or(text.len());

            text.replace_range(start_byte..end_byte, "");
            *cursor_pos = start;
            *selection_start = None;
        } else if *cursor_pos > 0 {
            let prev_pos = *cursor_pos - 1;
            let start_byte = text
                .char_indices()
                .nth(prev_pos)
                .map(|(i, _)| i)
                .unwrap_or(0);
            let end_byte = text
                .char_indices()
                .nth(*cursor_pos)
                .map(|(i, _)| i)
                .unwrap_or(text.len());

            text.replace_range(start_byte..end_byte, "");
            *cursor_pos = prev_pos;
        }
    }

    /// Deletes character after cursor.
    fn delete_forward(&self) {
        let mut text = self.text.write().unwrap();
        let mut cursor_pos = self.cursor_pos.write().unwrap();
        let mut selection_start = self.selection_start.write().unwrap();

        if let Some(sel_start) = *selection_start {
            // Delete selection
            let start = sel_start.min(*cursor_pos);
            let end = sel_start.max(*cursor_pos);

            let start_byte = text
                .char_indices()
                .nth(start)
                .map(|(i, _)| i)
                .unwrap_or(text.len());
            let end_byte = text
                .char_indices()
                .nth(end)
                .map(|(i, _)| i)
                .unwrap_or(text.len());

            text.replace_range(start_byte..end_byte, "");
            *cursor_pos = start;
            *selection_start = None;
        } else {
            let char_count = text.chars().count();
            if *cursor_pos < char_count {
                let start_byte = text
                    .char_indices()
                    .nth(*cursor_pos)
                    .map(|(i, _)| i)
                    .unwrap_or(text.len());
                let end_byte = text
                    .char_indices()
                    .nth(*cursor_pos + 1)
                    .map(|(i, _)| i)
                    .unwrap_or(text.len());

                text.replace_range(start_byte..end_byte, "");
            }
        }
    }

    /// Moves cursor left.
    fn move_left(&self, select: bool) {
        let mut cursor_pos = self.cursor_pos.write().unwrap();
        let mut selection_start = self.selection_start.write().unwrap();

        if select {
            if selection_start.is_none() {
                *selection_start = Some(*cursor_pos);
            }
        } else {
            *selection_start = None;
        }

        if *cursor_pos > 0 {
            *cursor_pos -= 1;
        }
    }

    /// Moves cursor right.
    fn move_right(&self, select: bool) {
        let text = self.text.read().unwrap();
        let char_count = text.chars().count();
        drop(text);

        let mut cursor_pos = self.cursor_pos.write().unwrap();
        let mut selection_start = self.selection_start.write().unwrap();

        if select {
            if selection_start.is_none() {
                *selection_start = Some(*cursor_pos);
            }
        } else {
            *selection_start = None;
        }

        if *cursor_pos < char_count {
            *cursor_pos += 1;
        }
    }

    /// Moves cursor to start.
    fn move_home(&self, select: bool) {
        let mut cursor_pos = self.cursor_pos.write().unwrap();
        let mut selection_start = self.selection_start.write().unwrap();

        if select {
            if selection_start.is_none() {
                *selection_start = Some(*cursor_pos);
            }
        } else {
            *selection_start = None;
        }

        *cursor_pos = 0;
    }

    /// Moves cursor to end.
    fn move_end(&self, select: bool) {
        let text = self.text.read().unwrap();
        let char_count = text.chars().count();
        drop(text);

        let mut cursor_pos = self.cursor_pos.write().unwrap();
        let mut selection_start = self.selection_start.write().unwrap();

        if select {
            if selection_start.is_none() {
                *selection_start = Some(*cursor_pos);
            }
        } else {
            *selection_start = None;
        }

        *cursor_pos = char_count;
    }

    /// Handles an Enter keypress: Shift+Enter inserts a literal newline
    /// (lets a chat-style input grow to multiple lines); plain Enter
    /// fires `on_enter` (typically "submit") instead, leaving the text
    /// untouched. Split out from `handle_key` so it's testable without
    /// needing a real `Context`.
    fn handle_enter(&self, shift: bool) {
        if shift {
            self.insert_text("\n");
            if let Some(ref callback) = self.on_change {
                callback(&self.get_text());
            }
        } else if let Some(ref callback) = self.on_enter {
            callback(&self.get_text());
        }
    }

    /// Selects all text.
    fn select_all(&self) {
        let text = self.text.read().unwrap();
        let char_count = text.chars().count();
        drop(text);

        *self.selection_start.write().unwrap() = Some(0);
        *self.cursor_pos.write().unwrap() = char_count;
    }

    fn draw_background(&self, ctx: &Context) {
        let mut canvas = ctx.canvas.borrow_mut();
        let state = *self.state.read().unwrap();

        let color = match state {
            TextBoxState::Idle => self.background_color,
            TextBoxState::Hover => self.background_color.level(1.1),
            TextBoxState::Focused => self.background_color.level(1.2),
            TextBoxState::Disabled => self.background_color.with_alpha(0.5),
        };

        canvas.fill_style(color);
        canvas.fill_round_rect(ctx.bounds, self.corner_radius);

        // Draw focus border
        if state == TextBoxState::Focused {
            let theme = get_theme();
            canvas.stroke_style(theme.frame_hilite_color);
            canvas.line_width(1.0);
            canvas.begin_path();
            canvas.add_round_rect(ctx.bounds, self.corner_radius);
            canvas.stroke();
        }
    }

    fn draw_text(&self, ctx: &Context) {
        let mut canvas = ctx.canvas.borrow_mut();
        let state = *self.state.read().unwrap();
        let display = self.display_text();

        let text_area = Rect::new(
            ctx.bounds.left + self.padding,
            ctx.bounds.top,
            ctx.bounds.right - self.padding,
            ctx.bounds.bottom,
        );

        canvas.font_size(self.font_size);

        // Without this, text/placeholder content wider than the box's own
        // bounds (e.g. a long placeholder on a narrow box, or typed text
        // past the visible width -- this box has no internal horizontal
        // scroll) paints straight through whatever sits next to it in the
        // layout, rather than just getting cut off at the box's own edge.
        // Matches `CodeEditor`'s own `canvas.save()`/`clip()`/`restore()`
        // convention for its text drawing.
        canvas.save();
        canvas.clip(ctx.bounds);

        if display.is_empty() && !self.placeholder.is_empty() {
            // Draw placeholder (always single-line, vertically centered)
            let color = if state == TextBoxState::Disabled {
                self.placeholder_color.with_alpha(0.3)
            } else {
                self.placeholder_color
            };
            canvas.fill_style(color);
            let y = text_area.center().y + self.font_size * 0.35;
            canvas.fill_text(&self.placeholder, Point::new(text_area.left, y));
        } else {
            // Draw text
            let color = if state == TextBoxState::Disabled {
                self.text_color.with_alpha(0.5)
            } else {
                self.text_color
            };
            canvas.fill_style(color);

            if !display.contains('\n') {
                // Unchanged single-line layout: vertically centered.
                let y = text_area.center().y + self.font_size * 0.35;
                canvas.fill_text(&display, Point::new(text_area.left, y));
            } else {
                // Content contains a newline (see `Shift+Enter` in
                // `handle_key`) -- lay rows out top-aligned instead of
                // centering the whole (unbounded-height) block.
                let line_height = self.line_height();
                let mut y = text_area.top + self.padding + self.font_size * 0.85;
                for line in display.split('\n') {
                    canvas.fill_text(line, Point::new(text_area.left, y));
                    y += line_height;
                }
            }
        }

        canvas.restore();
    }

    fn draw_selection(&self, ctx: &Context) {
        let selection_start = *self.selection_start.read().unwrap();
        let cursor_pos = *self.cursor_pos.read().unwrap();

        if selection_start.is_none() {
            return;
        }

        let sel_start = selection_start.unwrap();
        if sel_start == cursor_pos {
            return;
        }

        let mut canvas = ctx.canvas.borrow_mut();
        let display = self.display_text();

        let start = sel_start.min(cursor_pos);
        let end = sel_start.max(cursor_pos);

        // Measure text width up to start and end positions
        canvas.font_size(self.font_size);

        // Same overflow risk as `draw_text` -- a selection extending past
        // the box's own (unscrolled) visible width would otherwise paint
        // the highlight rect over neighboring widgets too.
        canvas.save();
        canvas.clip(ctx.bounds);
        canvas.fill_style(self.highlight_color);

        if !display.contains('\n') {
            let x1 =
                ctx.bounds.left + self.padding + canvas.text_width_to_position(&display, start);
            let x2 = ctx.bounds.left + self.padding + canvas.text_width_to_position(&display, end);
            let sel_rect = Rect::new(x1, ctx.bounds.top + 4.0, x2, ctx.bounds.bottom - 4.0);
            canvas.fill_rect(sel_rect);
        } else {
            // Selection spans (possibly) multiple rows -- draw one rect
            // per row: partial on the start/end rows, full-width on any
            // row strictly between them.
            let lines: Vec<&str> = display.split('\n').collect();
            let (start_row, start_col) = Self::row_col_for_pos(&display, start);
            let (end_row, end_col) = Self::row_col_for_pos(&display, end);

            for row in start_row..=end_row {
                let line = lines.get(row).copied().unwrap_or("");
                let col_start = if row == start_row { start_col } else { 0 };
                let col_end = if row == end_row {
                    end_col
                } else {
                    line.chars().count()
                };

                let x1 =
                    ctx.bounds.left + self.padding + canvas.text_width_to_position(line, col_start);
                let x2 =
                    ctx.bounds.left + self.padding + canvas.text_width_to_position(line, col_end);
                let (row_top, row_bottom, _) = self.multiline_row_bounds(ctx, row);
                canvas.fill_rect(Rect::new(x1, row_top, x2, row_bottom));
            }
        }

        canvas.restore();
    }

    fn draw_caret(&self, ctx: &Context) {
        let state = *self.state.read().unwrap();
        if state != TextBoxState::Focused {
            return;
        }

        let mut canvas = ctx.canvas.borrow_mut();
        let cursor_pos = *self.cursor_pos.read().unwrap();
        let display = self.display_text();

        // Measure text width up to cursor position
        canvas.font_size(self.font_size);
        let (x, y1, y2) = if !display.contains('\n') {
            let x = ctx.bounds.left
                + self.padding
                + canvas.text_width_to_position(&display, cursor_pos);
            (x, ctx.bounds.top + 4.0, ctx.bounds.bottom - 4.0)
        } else {
            let (row, col) = Self::row_col_for_pos(&display, cursor_pos);
            let line = display.split('\n').nth(row).unwrap_or("");
            let x = ctx.bounds.left + self.padding + canvas.text_width_to_position(line, col);
            let (row_top, row_bottom, _) = self.multiline_row_bounds(ctx, row);
            (x, row_top, row_bottom)
        };

        // Same overflow risk as `draw_text` -- a caret past the box's own
        // (unscrolled) visible width would otherwise draw outside it.
        canvas.save();
        canvas.clip(ctx.bounds);
        canvas.stroke_style(self.caret_color);
        canvas.line_width(1.5);
        canvas.begin_path();
        canvas.move_to(Point::new(x, y1));
        canvas.line_to(Point::new(x, y2));
        canvas.stroke();
        canvas.restore();
    }
}

impl Default for TextBox {
    fn default() -> Self {
        Self::new()
    }
}

impl Element for TextBox {
    /// `self.width` is a *minimum*, not a fixed size -- `max.x` is left
    /// unbounded so a genuinely stretchy sibling (see `stretch` below)
    /// actually receives extra horizontal space from its container
    /// instead of being clamped straight back down to `self.width`
    /// regardless of how much room is available. Height stays fixed,
    /// matching `stretch`'s `(1.0, 0.0)` -- this is a single-line box,
    /// not vertically resizable.
    fn limits(&self, _ctx: &BasicContext) -> ViewLimits {
        ViewLimits::new(
            Point::new(self.width, self.height),
            Point::new(super::FULL_EXTENT, self.height),
        )
    }

    fn stretch(&self) -> ViewStretch {
        ViewStretch::new(1.0, 0.0)
    }

    fn draw(&self, ctx: &Context) {
        self.draw_background(ctx);
        self.draw_selection(ctx);
        self.draw_text(ctx);
        self.draw_caret(ctx);
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

    fn wants_focus(&self) -> bool {
        self.enabled
    }

    fn begin_focus(&mut self, _req: FocusRequest) {
        *self.state.write().unwrap() = TextBoxState::Focused;
    }

    fn end_focus(&mut self) -> bool {
        *self.state.write().unwrap() = TextBoxState::Idle;
        true
    }

    fn clear_focus(&self) {
        let mut state = self.state.write().unwrap();
        if *state == TextBoxState::Focused {
            *state = TextBoxState::Idle;
        }
    }

    fn handle_click(&self, ctx: &Context, btn: MouseButton) -> bool {
        if !self.enabled || btn.button != MouseButtonKind::Left {
            return false;
        }

        if btn.down {
            *self.state.write().unwrap() = TextBoxState::Focused;

            // Set cursor position based on click location
            let char_width = self.font_size * 0.6;
            let text = self.text.read().unwrap();
            let char_count = text.chars().count();
            drop(text);

            let rel_x = btn.pos.x - ctx.bounds.left - self.padding;
            let pos = ((rel_x / char_width).round() as usize).min(char_count);

            *self.cursor_pos.write().unwrap() = pos;
            *self.selection_start.write().unwrap() = None;
        }

        true
    }

    fn key(&mut self, _ctx: &Context, k: KeyInfo) -> bool {
        self.handle_key(_ctx, k)
    }

    fn handle_key(&self, _ctx: &Context, k: KeyInfo) -> bool {
        if !self.enabled {
            return false;
        }

        let state = *self.state.read().unwrap();
        if state != TextBoxState::Focused {
            return false;
        }

        if k.action != crate::view::KeyAction::Press && k.action != crate::view::KeyAction::Repeat {
            return true;
        }

        let shift = k.modifiers & crate::view::modifiers::SHIFT != 0;
        let ctrl =
            k.modifiers & (crate::view::modifiers::CONTROL | crate::view::modifiers::SUPER) != 0;

        match k.key {
            KeyCode::Left => {
                self.move_left(shift);
                return true;
            }
            KeyCode::Right => {
                self.move_right(shift);
                return true;
            }
            KeyCode::Home => {
                self.move_home(shift);
                return true;
            }
            KeyCode::End => {
                self.move_end(shift);
                return true;
            }
            KeyCode::Backspace => {
                self.delete_backward();
                if let Some(ref callback) = self.on_change {
                    callback(&self.get_text());
                }
                return true;
            }
            KeyCode::Delete => {
                self.delete_forward();
                if let Some(ref callback) = self.on_change {
                    callback(&self.get_text());
                }
                return true;
            }
            KeyCode::Enter => {
                self.handle_enter(shift);
                return true;
            }
            KeyCode::A if ctrl => {
                self.select_all();
                return true;
            }
            _ => {}
        }

        false
    }

    fn text(&mut self, _ctx: &Context, info: TextInfo) -> bool {
        self.handle_text(_ctx, info)
    }

    fn handle_text(&self, _ctx: &Context, info: TextInfo) -> bool {
        if !self.enabled {
            return false;
        }

        let state = *self.state.read().unwrap();
        if state != TextBoxState::Focused {
            return false;
        }

        // Filter control characters
        let c = info.codepoint;
        if !c.is_control() {
            let s = c.to_string();
            self.insert_text(&s);
            if let Some(ref callback) = self.on_change {
                callback(&self.get_text());
            }
        }

        true
    }

    fn cursor(&mut self, _ctx: &Context, _p: Point, status: CursorTracking) -> bool {
        if !self.enabled {
            return false;
        }

        let mut state = self.state.write().unwrap();
        if *state == TextBoxState::Focused {
            return true;
        }

        match status {
            CursorTracking::Entering | CursorTracking::Hovering => {
                *state = TextBoxState::Hover;
            }
            CursorTracking::Leaving => {
                *state = TextBoxState::Idle;
            }
        }

        true
    }

    fn enable(&mut self, state: bool) {
        self.enabled = state;
        let mut box_state = self.state.write().unwrap();
        if !state {
            *box_state = TextBoxState::Disabled;
        } else if *box_state == TextBoxState::Disabled {
            *box_state = TextBoxState::Idle;
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

/// Creates a text box.
pub fn text_box() -> TextBox {
    TextBox::new()
}

/// Creates a text box with initial text.
pub fn text_box_with_text(text: impl Into<String>) -> TextBox {
    TextBox::new().text(text)
}

/// Creates a password input field.
pub fn password_box() -> TextBox {
    TextBox::new().password(true)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicBool, Ordering};
    use std::sync::{Arc, Mutex};

    #[test]
    fn shift_enter_inserts_a_newline_without_firing_on_enter() {
        let fired = Arc::new(AtomicBool::new(false));
        let fired_clone = fired.clone();
        let tb = TextBox::new()
            .text("hello")
            .on_enter(move |_| fired_clone.store(true, Ordering::SeqCst));

        tb.handle_enter(true);

        assert_eq!(tb.get_text(), "hello\n");
        assert!(!fired.load(Ordering::SeqCst));
    }

    #[test]
    fn plain_enter_fires_on_enter_and_leaves_text_untouched() {
        let captured = Arc::new(Mutex::new(String::new()));
        let captured_clone = captured.clone();
        let tb = TextBox::new()
            .text("hello")
            .on_enter(move |text| *captured_clone.lock().unwrap() = text.to_string());

        tb.handle_enter(false);

        assert_eq!(tb.get_text(), "hello");
        assert_eq!(*captured.lock().unwrap(), "hello");
    }

    #[test]
    fn row_col_for_pos_tracks_newlines_correctly() {
        let text = "ab\ncde\nf";
        assert_eq!(TextBox::row_col_for_pos(text, 0), (0, 0));
        assert_eq!(TextBox::row_col_for_pos(text, 2), (0, 2));
        assert_eq!(TextBox::row_col_for_pos(text, 3), (1, 0));
        assert_eq!(TextBox::row_col_for_pos(text, 7), (2, 0));
        assert_eq!(TextBox::row_col_for_pos(text, 8), (2, 1));
    }
}
