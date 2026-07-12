//! Multi-line code editor element with tree-sitter syntax highlighting.
//!
//! Follows the same interior-mutability shape as [`super::text_box::TextBox`]
//! (RwLock-guarded state, `handle_*` methods as the real logic so this works
//! behind `Arc<dyn Element>`), extended to a multi-line buffer with a line
//! number gutter, snapshot-based undo/redo, and tree-sitter-driven color.
//!
//! Scope for this first version: single cursor + one contiguous selection
//! (no multi-cursor), whole-buffer reparse per edit (not tree-sitter's
//! incremental `Tree::edit`), and Rust highlighting only. All three are
//! straightforward to extend later without changing the element's shape.

use std::any::Any;
use std::sync::RwLock;

use streaming_iterator::StreamingIterator;

use super::context::{BasicContext, Context};
use super::{Element, FocusRequest, ViewLimits, ViewStretch};
use crate::support::color::Color;
use crate::support::point::Point;
use crate::support::rect::Rect;
use crate::support::theme::get_theme;
use crate::view::{CursorTracking, KeyCode, KeyInfo, MouseButton, MouseButtonKind, TextInfo};

/// A (line, column) position in the buffer. `column` is a char index within
/// `line`'s `String`, not a byte offset. `PartialOrd`/`Ord` compare fields in
/// declaration order (line first), giving buffer order for free -- used by
/// `find_next`/`find_prev` to locate the nearest match relative to the
/// cursor.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Default)]
struct CursorPos {
    line: usize,
    column: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
enum EditorState {
    #[default]
    Idle,
    Hover,
    Focused,
}

/// One highlighted span within the buffer, in (line, column) coordinates so
/// it survives being recomputed each draw without byte-offset bookkeeping
/// leaking into rendering.
#[derive(Debug, Clone, Copy)]
struct Highlight {
    start: CursorPos,
    end: CursorPos,
    color: Color,
}

/// Severity of a [`Diagnostic`], matching the LSP's three-level scheme
/// (LSP's `Hint` folds into `Info` here -- one more color wouldn't add
/// anything a caller couldn't already convey via `message`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DiagnosticSeverity {
    Error,
    Warning,
    Info,
}

/// A single diagnostic (e.g. from `mkide-lsp`) attached to one line.
/// Whole-line rather than column-range, matching this editor's existing
/// preference for simplicity in its first version (see the module doc
/// comment) -- enough to show "something's wrong here," which is what the
/// gutter marker and line tint are for; the message itself carries the
/// specifics.
#[derive(Debug, Clone)]
pub struct Diagnostic {
    pub line: usize,
    pub severity: DiagnosticSeverity,
    pub message: String,
}

pub type TextChangeCallback = Box<dyn Fn(&str) + Send + Sync>;

/// A multi-line code editor with line numbers and syntax highlighting.
pub struct CodeEditor {
    lines: RwLock<Vec<String>>,
    cursor: RwLock<CursorPos>,
    selection_anchor: RwLock<Option<CursorPos>>,
    scroll_offset: RwLock<f32>,
    state: RwLock<EditorState>,
    undo_stack: RwLock<Vec<Vec<String>>>,
    redo_stack: RwLock<Vec<Vec<String>>>,
    highlights: RwLock<Vec<Highlight>>,
    parser: RwLock<tree_sitter::Parser>,
    query: Option<tree_sitter::Query>,
    diagnostics: RwLock<Vec<Diagnostic>>,
    read_only: RwLock<bool>,
    find_query: RwLock<String>,
    find_matches: RwLock<Vec<CursorPos>>,

    background_color: Color,
    gutter_color: Color,
    gutter_text_color: Color,
    text_color: Color,
    highlight_select_color: Color,
    find_match_color: Color,
    error_color: Color,
    warning_color: Color,
    info_color: Color,
    caret_color: Color,
    font_size: f32,
    line_height: f32,
    gutter_width: f32,
    width: f32,
    height: f32,
    enabled: bool,
    on_change: Option<TextChangeCallback>,
}

impl CodeEditor {
    /// Creates a new code editor with Rust syntax highlighting.
    pub fn new() -> Self {
        let theme = get_theme();
        let mut parser = tree_sitter::Parser::new();
        let query = parser
            .set_language(&tree_sitter_rust::LANGUAGE.into())
            .ok()
            .and_then(|_| {
                tree_sitter::Query::new(&tree_sitter_rust::LANGUAGE.into(), RUST_HIGHLIGHT_QUERY)
                    .map_err(|err| {
                        log::warn!("code_editor: highlight query failed to compile: {err}")
                    })
                    .ok()
            });

        let editor = Self {
            lines: RwLock::new(vec![String::new()]),
            cursor: RwLock::new(CursorPos::default()),
            selection_anchor: RwLock::new(None),
            scroll_offset: RwLock::new(0.0),
            state: RwLock::new(EditorState::Idle),
            undo_stack: RwLock::new(Vec::new()),
            redo_stack: RwLock::new(Vec::new()),
            highlights: RwLock::new(Vec::new()),
            parser: RwLock::new(parser),
            query,
            diagnostics: RwLock::new(Vec::new()),
            read_only: RwLock::new(false),
            find_query: RwLock::new(String::new()),
            find_matches: RwLock::new(Vec::new()),
            background_color: theme.input_box_color,
            gutter_color: theme.input_box_color.level(0.9),
            gutter_text_color: theme.text_box_idle_color,
            text_color: theme.text_box_font_color,
            highlight_select_color: theme.text_box_hilite_color,
            find_match_color: Color::from_rgb_u32(0xffd54a).with_alpha(0.45),
            error_color: Color::from_rgb_u32(0xe5484d),
            warning_color: Color::from_rgb_u32(0xf5a623),
            info_color: Color::from_rgb_u32(0x4a9fe5),
            caret_color: theme.text_box_caret_color,
            font_size: theme.text_box_font_size,
            line_height: theme.text_box_font_size * 1.4,
            gutter_width: 48.0,
            width: 600.0,
            height: 400.0,
            enabled: true,
            on_change: None,
        };
        editor.reparse();
        editor
    }

    /// Sets the initial text (replaces the buffer, clears undo history).
    pub fn text(mut self, text: impl Into<String>) -> Self {
        self.set_text_inner(text.into());
        *self.undo_stack.get_mut().unwrap() = Vec::new();
        *self.redo_stack.get_mut().unwrap() = Vec::new();
        self
    }

    /// Sets the width.
    pub fn width(mut self, width: f32) -> Self {
        self.width = width;
        self
    }

    /// Sets the height.
    pub fn height(mut self, height: f32) -> Self {
        self.height = height;
        self
    }

    /// Sets the change callback, invoked with the full buffer text after
    /// every edit.
    pub fn on_change<F: Fn(&str) + Send + Sync + 'static>(mut self, callback: F) -> Self {
        self.on_change = Some(Box::new(callback));
        self
    }

    /// Makes the editor read-only from construction (builder form). Cursor
    /// movement, selection, and copying still work; typing, paste, and
    /// undo/redo do not. Needed for e.g. MKIDE's build-output/log panel,
    /// which reuses this same editor purely as a scrollable, syntax-free
    /// text view that the user shouldn't be able to accidentally edit.
    pub fn read_only(mut self, read_only: bool) -> Self {
        *self.read_only.get_mut().unwrap() = read_only;
        self
    }

    /// Toggles read-only at runtime (e.g. locking the buffer while a build
    /// is in progress).
    pub fn set_read_only(&self, read_only: bool) {
        *self.read_only.write().unwrap() = read_only;
    }

    /// Returns whether the editor is currently read-only.
    pub fn is_read_only(&self) -> bool {
        *self.read_only.read().unwrap()
    }

    /// Replaces the set of per-line diagnostics shown as gutter markers and
    /// a faint full-line tint (e.g. from `mkide-lsp`'s
    /// `LspClient::diagnostics_for`). Lines outside the current buffer are
    /// silently ignored rather than panicking, since diagnostics can arrive
    /// slightly out of sync with the buffer (the language server saw an
    /// older or newer version of the file).
    pub fn set_diagnostics(&self, diagnostics: Vec<Diagnostic>) {
        *self.diagnostics.write().unwrap() = diagnostics;
    }

    /// Clears all diagnostic markers.
    pub fn clear_diagnostics(&self) {
        self.diagnostics.write().unwrap().clear();
    }

    /// Sets the text to highlight all occurrences of, and moves the cursor
    /// to the first match at or after the current cursor position (wrapping
    /// around to the start of the buffer if none is found after it). Pass
    /// an empty string to clear highlighting. Returns whether any match was
    /// found (always `false` for an empty query).
    pub fn find(&self, query: &str) -> bool {
        *self.find_query.write().unwrap() = query.to_string();
        self.recompute_find_matches();
        if query.is_empty() {
            return false;
        }
        let cursor = *self.cursor.read().unwrap();
        self.find_next_from(cursor, true)
    }

    /// Moves to the next match after the current cursor position, wrapping
    /// around. No-op (returns `false`) if `find` hasn't been called with a
    /// non-empty query, or there are no matches.
    pub fn find_next(&self) -> bool {
        let cursor = *self.cursor.read().unwrap();
        self.find_next_from(cursor, true)
    }

    /// Moves to the previous match before the current cursor position,
    /// wrapping around.
    pub fn find_prev(&self) -> bool {
        let cursor = *self.cursor.read().unwrap();
        self.find_next_from(cursor, false)
    }

    fn find_next_from(&self, from: CursorPos, forward: bool) -> bool {
        let matches = self.find_matches.read().unwrap();
        if matches.is_empty() {
            return false;
        }
        let next = if forward {
            matches
                .iter()
                .find(|m| **m > from)
                .or_else(|| matches.first())
        } else {
            matches
                .iter()
                .rev()
                .find(|m| **m < from)
                .or_else(|| matches.last())
        };
        let Some(&pos) = next else {
            return false;
        };
        drop(matches);
        *self.cursor.write().unwrap() = pos;
        *self.selection_anchor.write().unwrap() = None;
        true
    }

    fn recompute_find_matches(&self) {
        let query = self.find_query.read().unwrap().clone();
        let mut matches = Vec::new();
        if !query.is_empty() {
            let lines = self.lines.read().unwrap();
            for (line_index, line) in lines.iter().enumerate() {
                let mut start = 0;
                while let Some(byte_offset) = line[start..].find(&query) {
                    let byte_pos = start + byte_offset;
                    let column = line[..byte_pos].chars().count();
                    matches.push(CursorPos {
                        line: line_index,
                        column,
                    });
                    start = byte_pos + query.len().max(1);
                    if start >= line.len() {
                        break;
                    }
                }
            }
        }
        *self.find_matches.write().unwrap() = matches;
    }

    /// Returns the current buffer text (lines joined with `\n`).
    pub fn get_text(&self) -> String {
        self.lines.read().unwrap().join("\n")
    }

    /// Replaces the buffer text, resetting cursor/selection/scroll.
    pub fn set_text(&self, text: impl Into<String>) {
        self.push_undo_snapshot();
        self.set_text_inner(text.into());
    }

    fn set_text_inner(&self, text: String) {
        let lines: Vec<String> = if text.is_empty() {
            vec![String::new()]
        } else {
            text.split('\n').map(str::to_string).collect()
        };
        *self.lines.write().unwrap() = lines;
        *self.cursor.write().unwrap() = CursorPos::default();
        *self.selection_anchor.write().unwrap() = None;
        *self.scroll_offset.write().unwrap() = 0.0;
        self.reparse();
    }

    fn push_undo_snapshot(&self) {
        let snapshot = self.lines.read().unwrap().clone();
        self.undo_stack.write().unwrap().push(snapshot);
        self.redo_stack.write().unwrap().clear();
    }

    fn undo(&self) {
        if *self.read_only.read().unwrap() {
            return;
        }
        let Some(snapshot) = self.undo_stack.write().unwrap().pop() else {
            return;
        };
        let current = self.lines.read().unwrap().clone();
        self.redo_stack.write().unwrap().push(current);
        *self.lines.write().unwrap() = snapshot;
        self.clamp_cursor();
        self.reparse();
        self.notify_change();
    }

    fn redo(&self) {
        if *self.read_only.read().unwrap() {
            return;
        }
        let Some(snapshot) = self.redo_stack.write().unwrap().pop() else {
            return;
        };
        let current = self.lines.read().unwrap().clone();
        self.undo_stack.write().unwrap().push(current);
        *self.lines.write().unwrap() = snapshot;
        self.clamp_cursor();
        self.reparse();
        self.notify_change();
    }

    fn clamp_cursor(&self) {
        let lines = self.lines.read().unwrap();
        let mut cursor = self.cursor.write().unwrap();
        cursor.line = cursor.line.min(lines.len().saturating_sub(1));
        cursor.column = cursor.column.min(lines[cursor.line].chars().count());
    }

    fn notify_change(&self) {
        if let Some(ref callback) = self.on_change {
            callback(&self.get_text());
        }
    }

    /// Re-runs the tree-sitter parser over the whole buffer and recomputes
    /// highlight spans. Whole-buffer reparse (not incremental) -- simplest
    /// correct approach for a first version; fine at editor-buffer sizes,
    /// worth revisiting with `Tree::edit` if profiling ever shows it matters.
    fn reparse(&self) {
        // Called after every edit, so this is also the one place that keeps
        // find-match positions in sync with the buffer, regardless of
        // whether tree-sitter highlighting itself is available below.
        self.recompute_find_matches();

        let Some(query) = &self.query else {
            return;
        };
        let text = self.get_text();
        let mut parser = self.parser.write().unwrap();
        let Some(tree) = parser.parse(&text, None) else {
            return;
        };
        drop(parser);

        let line_starts = line_start_byte_offsets(&text);
        let mut cursor = tree_sitter::QueryCursor::new();
        let mut matches = cursor.matches(query, tree.root_node(), text.as_bytes());
        let theme_colors = HighlightColors::from_theme();
        let mut spans = Vec::new();
        while let Some(m) = matches.next() {
            for capture in m.captures {
                let name = &query.capture_names()[capture.index as usize];
                let Some(color) = theme_colors.for_capture(name) else {
                    continue;
                };
                let node = capture.node;
                let start = byte_to_cursor_pos(node.start_byte(), &line_starts);
                let end = byte_to_cursor_pos(node.end_byte(), &line_starts);
                spans.push(Highlight { start, end, color });
            }
        }
        *self.highlights.write().unwrap() = spans;
    }

    fn insert_text(&self, s: &str) {
        if *self.read_only.read().unwrap() {
            return;
        }
        self.push_undo_snapshot();
        let mut lines = self.lines.write().unwrap();
        let mut cursor = self.cursor.write().unwrap();
        let mut anchor = self.selection_anchor.write().unwrap();

        if let Some(sel) = *anchor {
            delete_range_inner(&mut lines, sel, *cursor, &mut cursor);
            *anchor = None;
        }

        if s == "\n" {
            let line = lines[cursor.line].clone();
            let byte = char_to_byte(&line, cursor.column);
            let (before, after) = line.split_at(byte);
            lines[cursor.line] = before.to_string();
            lines.insert(cursor.line + 1, after.to_string());
            cursor.line += 1;
            cursor.column = 0;
        } else {
            let line = &mut lines[cursor.line];
            let byte = char_to_byte(line, cursor.column);
            line.insert_str(byte, s);
            cursor.column += s.chars().count();
        }

        drop(lines);
        drop(cursor);
        drop(anchor);
        self.reparse();
        self.notify_change();
    }

    fn delete_backward(&self) {
        if *self.read_only.read().unwrap() {
            return;
        }
        let mut lines = self.lines.write().unwrap();
        let mut cursor = self.cursor.write().unwrap();
        let mut anchor = self.selection_anchor.write().unwrap();

        if let Some(sel) = anchor.take() {
            self.push_undo_snapshot_locked(&lines);
            delete_range_inner(&mut lines, sel, *cursor, &mut cursor);
        } else if cursor.column > 0 {
            self.push_undo_snapshot_locked(&lines);
            let line = &mut lines[cursor.line];
            let start = char_to_byte(line, cursor.column - 1);
            let end = char_to_byte(line, cursor.column);
            line.replace_range(start..end, "");
            cursor.column -= 1;
        } else if cursor.line > 0 {
            self.push_undo_snapshot_locked(&lines);
            let current = lines.remove(cursor.line);
            let prev_len = lines[cursor.line - 1].chars().count();
            lines[cursor.line - 1].push_str(&current);
            cursor.line -= 1;
            cursor.column = prev_len;
        }

        drop(lines);
        drop(cursor);
        drop(anchor);
        self.reparse();
        self.notify_change();
    }

    fn delete_forward(&self) {
        if *self.read_only.read().unwrap() {
            return;
        }
        let mut lines = self.lines.write().unwrap();
        let mut cursor = self.cursor.write().unwrap();
        let mut anchor = self.selection_anchor.write().unwrap();

        if let Some(sel) = anchor.take() {
            self.push_undo_snapshot_locked(&lines);
            delete_range_inner(&mut lines, sel, *cursor, &mut cursor);
        } else {
            let line_char_count = lines[cursor.line].chars().count();
            if cursor.column < line_char_count {
                self.push_undo_snapshot_locked(&lines);
                let line = &mut lines[cursor.line];
                let start = char_to_byte(line, cursor.column);
                let end = char_to_byte(line, cursor.column + 1);
                line.replace_range(start..end, "");
            } else if cursor.line + 1 < lines.len() {
                self.push_undo_snapshot_locked(&lines);
                let next = lines.remove(cursor.line + 1);
                lines[cursor.line].push_str(&next);
            }
        }

        drop(lines);
        drop(cursor);
        drop(anchor);
        self.reparse();
        self.notify_change();
    }

    /// Same as [`Self::push_undo_snapshot`] but takes an already-held read
    /// guard on `lines` to snapshot, since the delete methods above need to
    /// record history *before* mutating but while still holding the write
    /// lock they're about to mutate through.
    fn push_undo_snapshot_locked(&self, lines: &[String]) {
        self.undo_stack.write().unwrap().push(lines.to_vec());
        self.redo_stack.write().unwrap().clear();
    }

    fn move_left(&self, select: bool) {
        let lines = self.lines.read().unwrap();
        let mut cursor = self.cursor.write().unwrap();
        self.update_selection_anchor(select);
        if cursor.column > 0 {
            cursor.column -= 1;
        } else if cursor.line > 0 {
            cursor.line -= 1;
            cursor.column = lines[cursor.line].chars().count();
        }
    }

    fn move_right(&self, select: bool) {
        let lines = self.lines.read().unwrap();
        let mut cursor = self.cursor.write().unwrap();
        self.update_selection_anchor(select);
        let line_len = lines[cursor.line].chars().count();
        if cursor.column < line_len {
            cursor.column += 1;
        } else if cursor.line + 1 < lines.len() {
            cursor.line += 1;
            cursor.column = 0;
        }
    }

    fn move_up(&self, select: bool) {
        let lines = self.lines.read().unwrap();
        let mut cursor = self.cursor.write().unwrap();
        self.update_selection_anchor(select);
        if cursor.line > 0 {
            cursor.line -= 1;
            cursor.column = cursor.column.min(lines[cursor.line].chars().count());
        }
    }

    fn move_down(&self, select: bool) {
        let lines = self.lines.read().unwrap();
        let mut cursor = self.cursor.write().unwrap();
        self.update_selection_anchor(select);
        if cursor.line + 1 < lines.len() {
            cursor.line += 1;
            cursor.column = cursor.column.min(lines[cursor.line].chars().count());
        }
    }

    fn move_home(&self, select: bool) {
        let mut cursor = self.cursor.write().unwrap();
        self.update_selection_anchor(select);
        cursor.column = 0;
    }

    fn move_end(&self, select: bool) {
        let lines = self.lines.read().unwrap();
        let mut cursor = self.cursor.write().unwrap();
        self.update_selection_anchor(select);
        cursor.column = lines[cursor.line].chars().count();
    }

    fn select_all(&self) {
        let lines = self.lines.read().unwrap();
        let last_line = lines.len() - 1;
        let last_col = lines[last_line].chars().count();
        *self.selection_anchor.write().unwrap() = Some(CursorPos { line: 0, column: 0 });
        *self.cursor.write().unwrap() = CursorPos {
            line: last_line,
            column: last_col,
        };
    }

    fn update_selection_anchor(&self, select: bool) {
        let mut anchor = self.selection_anchor.write().unwrap();
        if select {
            if anchor.is_none() {
                *anchor = Some(*self.cursor.read().unwrap());
            }
        } else {
            *anchor = None;
        }
    }

    /// Highest-severity diagnostic on `line`, if any (a line with both an
    /// error and a warning shows the error marker, since that's the more
    /// actionable of the two).
    fn diagnostic_severity_for_line(&self, line: usize) -> Option<DiagnosticSeverity> {
        self.diagnostics
            .read()
            .unwrap()
            .iter()
            .filter(|d| d.line == line)
            .map(|d| d.severity)
            .max_by_key(|s| match s {
                DiagnosticSeverity::Error => 2,
                DiagnosticSeverity::Warning => 1,
                DiagnosticSeverity::Info => 0,
            })
    }

    fn severity_color(&self, severity: DiagnosticSeverity) -> Color {
        match severity {
            DiagnosticSeverity::Error => self.error_color,
            DiagnosticSeverity::Warning => self.warning_color,
            DiagnosticSeverity::Info => self.info_color,
        }
    }

    fn draw_gutter(&self, ctx: &Context, first_visible_line: usize, visible_lines: usize) {
        let mut canvas = ctx.canvas.borrow_mut();
        let gutter_rect = Rect::new(
            ctx.bounds.left,
            ctx.bounds.top,
            ctx.bounds.left + self.gutter_width,
            ctx.bounds.bottom,
        );
        canvas.fill_style(self.gutter_color);
        canvas.fill_rect(gutter_rect);

        let theme = get_theme();
        canvas.font(theme.text_box_font);
        canvas.font_size(self.font_size);

        let lines = self.lines.read().unwrap();
        for row in 0..visible_lines {
            let line_index = first_visible_line + row;
            if line_index >= lines.len() {
                break;
            }
            let y = ctx.bounds.top - self.scroll_offset.read().unwrap().fract() * 0.0
                + (row as f32 + 1.0) * self.line_height
                - self.line_height * 0.3;

            if let Some(severity) = self.diagnostic_severity_for_line(line_index) {
                canvas.fill_style(self.severity_color(severity));
                let dot_y = y - self.font_size * 0.35;
                canvas.fill_round_rect(
                    Rect::new(ctx.bounds.left + 4.0, dot_y - 3.0, ctx.bounds.left + 10.0, dot_y + 3.0),
                    3.0,
                );
            }

            let label = (line_index + 1).to_string();
            canvas.fill_style(self.gutter_text_color);
            let x = ctx.bounds.left + self.gutter_width - 8.0 - canvas.text_width(&label);
            canvas.fill_text(&label, Point::new(x, y));
        }
    }

    fn draw_selection_and_text(
        &self,
        ctx: &Context,
        first_visible_line: usize,
        visible_lines: usize,
    ) {
        let mut canvas = ctx.canvas.borrow_mut();
        let theme = get_theme();
        canvas.font(theme.text_box_font);
        canvas.font_size(self.font_size);

        let lines = self.lines.read().unwrap();
        let cursor = *self.cursor.read().unwrap();
        let anchor = *self.selection_anchor.read().unwrap();
        let highlights = self.highlights.read().unwrap();
        let find_matches = self.find_matches.read().unwrap();
        let query_len = self.find_query.read().unwrap().chars().count();
        let text_left = ctx.bounds.left + self.gutter_width + 6.0;

        for row in 0..visible_lines {
            let line_index = first_visible_line + row;
            if line_index >= lines.len() {
                break;
            }
            let line = &lines[line_index];
            let y_top = ctx.bounds.top + row as f32 * self.line_height;
            let y_baseline = y_top + self.line_height - self.font_size * 0.3;

            // Faint full-line tint for a diagnostic on this line, drawn
            // first so selection/find highlights and text stay legible on
            // top of it.
            if let Some(severity) = self.diagnostic_severity_for_line(line_index) {
                canvas.fill_style(self.severity_color(severity).with_alpha(0.12));
                canvas.fill_rect(Rect::new(
                    ctx.bounds.left + self.gutter_width,
                    y_top,
                    ctx.bounds.right,
                    y_top + self.line_height,
                ));
            }

            // Highlight every find match on this line.
            if query_len > 0 {
                for m in find_matches.iter().filter(|m| m.line == line_index) {
                    let x1 = text_left + canvas.text_width_to_position(line, m.column);
                    let x2 = text_left
                        + canvas.text_width_to_position(line, (m.column + query_len).min(line.chars().count()));
                    canvas.fill_style(self.find_match_color);
                    canvas.fill_rect(Rect::new(x1, y_top, x2.max(x1 + 2.0), y_top + self.line_height));
                }
            }

            // Selection background for this line, if any.
            if let Some(sel) = anchor {
                if let Some((start_col, end_col)) = selection_on_line(sel, cursor, line_index) {
                    let x1 = text_left + canvas.text_width_to_position(line, start_col);
                    let x2 = text_left
                        + canvas.text_width_to_position(line, end_col.min(line.chars().count()));
                    canvas.fill_style(self.highlight_select_color);
                    canvas.fill_rect(Rect::new(
                        x1,
                        y_top,
                        x2.max(x1 + 2.0),
                        y_top + self.line_height,
                    ));
                }
            }

            // Syntax-highlighted text, drawn as colored runs.
            let segments = line_color_segments(line, line_index, &highlights, self.text_color);
            for (start_col, end_col, color) in segments {
                let start_byte = char_to_byte(line, start_col);
                let end_byte = char_to_byte(line, end_col);
                if start_byte >= end_byte {
                    continue;
                }
                canvas.fill_style(color);
                let x = text_left + canvas.text_width_to_position(line, start_col);
                canvas.fill_text(&line[start_byte..end_byte], Point::new(x, y_baseline));
            }

            // Caret.
            if line_index == cursor.line && *self.state.read().unwrap() == EditorState::Focused {
                let x = text_left + canvas.text_width_to_position(line, cursor.column);
                canvas.stroke_style(self.caret_color);
                canvas.line_width(1.5);
                canvas.begin_path();
                canvas.move_to(Point::new(x, y_top + 2.0));
                canvas.line_to(Point::new(x, y_top + self.line_height - 2.0));
                canvas.stroke();
            }
        }
    }

    fn visible_line_window(&self, ctx: &Context) -> (usize, usize) {
        let scroll = *self.scroll_offset.read().unwrap();
        let first = (scroll / self.line_height).floor().max(0.0) as usize;
        let visible = (ctx.bounds.height() / self.line_height).ceil() as usize + 1;
        (first, visible)
    }

    fn cursor_pos_from_click(&self, ctx: &Context, p: Point) -> CursorPos {
        let lines = self.lines.read().unwrap();
        let scroll = *self.scroll_offset.read().unwrap();
        let row = (((p.y - ctx.bounds.top + scroll) / self.line_height)
            .floor()
            .max(0.0)) as usize;
        let line = row.min(lines.len().saturating_sub(1));
        let line_text = &lines[line];

        let mut canvas = ctx.canvas.borrow_mut();
        let theme = get_theme();
        canvas.font(theme.text_box_font);
        canvas.font_size(self.font_size);
        let text_left = ctx.bounds.left + self.gutter_width + 6.0;
        let rel_x = p.x - text_left;

        let char_count = line_text.chars().count();
        let mut column = char_count;
        for i in 0..=char_count {
            if canvas.text_width_to_position(line_text, i) >= rel_x {
                column = i;
                break;
            }
        }
        CursorPos { line, column }
    }
}

impl Default for CodeEditor {
    fn default() -> Self {
        Self::new()
    }
}

impl Element for CodeEditor {
    fn limits(&self, _ctx: &BasicContext) -> ViewLimits {
        // `min_size`, not `fixed`: `.width()`/`.height()` set a starting
        // size, not a hard cap. `ViewLimits::fixed` pins `max` to the same
        // value as `min`, and since `VTile`/`HTile` aggregate `max` across
        // children via `min()`, a single fixed-size editor in a layout caps
        // the *entire* surrounding column/row at that size forever, however
        // wide the window grows -- confirmed as the cause of MKIDE's editor
        // area never resizing past the ~700pt it was constructed with.
        ViewLimits::min_size(self.width, self.height)
    }

    fn stretch(&self) -> ViewStretch {
        ViewStretch::new(1.0, 1.0)
    }

    fn draw(&self, ctx: &Context) {
        {
            let mut canvas = ctx.canvas.borrow_mut();
            canvas.fill_style(self.background_color);
            canvas.fill_rect(ctx.bounds);
        }
        let (first, visible) = self.visible_line_window(ctx);
        self.draw_selection_and_text(ctx, first, visible);
        self.draw_gutter(ctx, first, visible);
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
        *self.state.write().unwrap() = EditorState::Focused;
    }

    fn end_focus(&mut self) -> bool {
        *self.state.write().unwrap() = EditorState::Idle;
        true
    }

    fn clear_focus(&self) {
        let mut state = self.state.write().unwrap();
        if *state == EditorState::Focused {
            *state = EditorState::Idle;
        }
    }

    fn handle_click(&self, ctx: &Context, btn: MouseButton) -> bool {
        if !self.enabled || btn.button != MouseButtonKind::Left {
            return false;
        }
        if btn.down {
            *self.state.write().unwrap() = EditorState::Focused;
            let pos = self.cursor_pos_from_click(ctx, btn.pos);
            *self.cursor.write().unwrap() = pos;
            *self.selection_anchor.write().unwrap() = None;
        }
        true
    }

    fn cursor(&mut self, _ctx: &Context, _p: Point, status: CursorTracking) -> bool {
        if !self.enabled {
            return false;
        }
        let mut state = self.state.write().unwrap();
        if *state == EditorState::Focused {
            return true;
        }
        match status {
            CursorTracking::Entering | CursorTracking::Hovering => *state = EditorState::Hover,
            CursorTracking::Leaving => *state = EditorState::Idle,
        }
        true
    }

    fn handle_scroll(&self, ctx: &Context, dir: Point, _p: Point) -> bool {
        if !self.enabled {
            return false;
        }
        let lines = self.lines.read().unwrap();
        let max_scroll = (lines.len() as f32 * self.line_height - ctx.bounds.height()).max(0.0);
        let mut scroll = self.scroll_offset.write().unwrap();
        *scroll = (*scroll - dir.y).clamp(0.0, max_scroll);
        true
    }

    fn key(&mut self, ctx: &Context, k: KeyInfo) -> bool {
        self.handle_key(ctx, k)
    }

    fn handle_key(&self, _ctx: &Context, k: KeyInfo) -> bool {
        if !self.enabled || *self.state.read().unwrap() != EditorState::Focused {
            return false;
        }
        if k.action != crate::view::KeyAction::Press && k.action != crate::view::KeyAction::Repeat {
            return true;
        }

        let shift = k.modifiers & crate::view::modifiers::SHIFT != 0;
        let ctrl =
            k.modifiers & (crate::view::modifiers::CONTROL | crate::view::modifiers::SUPER) != 0;

        match k.key {
            KeyCode::Left => self.move_left(shift),
            KeyCode::Right => self.move_right(shift),
            KeyCode::Up => self.move_up(shift),
            KeyCode::Down => self.move_down(shift),
            KeyCode::Home => self.move_home(shift),
            KeyCode::End => self.move_end(shift),
            KeyCode::Backspace => self.delete_backward(),
            KeyCode::Delete => self.delete_forward(),
            KeyCode::Enter => self.insert_text("\n"),
            KeyCode::Tab => self.insert_text("    "),
            KeyCode::A if ctrl => self.select_all(),
            KeyCode::Z if ctrl && shift => self.redo(),
            KeyCode::Z if ctrl => self.undo(),
            KeyCode::Y if ctrl => self.redo(),
            _ => return false,
        }
        true
    }

    fn text(&mut self, ctx: &Context, info: TextInfo) -> bool {
        self.handle_text(ctx, info)
    }

    fn handle_text(&self, _ctx: &Context, info: TextInfo) -> bool {
        if !self.enabled || *self.state.read().unwrap() != EditorState::Focused {
            return false;
        }
        let c = info.codepoint;
        if !c.is_control() {
            self.insert_text(&c.to_string());
        }
        true
    }

    fn enable(&mut self, state: bool) {
        self.enabled = state;
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

/// Creates a code editor.
pub fn code_editor() -> CodeEditor {
    CodeEditor::new()
}

// --- helpers -----------------------------------------------------------

fn char_to_byte(line: &str, column: usize) -> usize {
    line.char_indices()
        .nth(column)
        .map(|(i, _)| i)
        .unwrap_or(line.len())
}

/// Deletes the (line, column)-addressed range `[a, b)` (order-independent)
/// from `lines` in place, and sets `*cursor` to the range's start.
fn delete_range_inner(lines: &mut Vec<String>, a: CursorPos, b: CursorPos, cursor: &mut CursorPos) {
    let (start, end) = if (a.line, a.column) <= (b.line, b.column) {
        (a, b)
    } else {
        (b, a)
    };

    if start.line == end.line {
        let line = &mut lines[start.line];
        let sb = char_to_byte(line, start.column);
        let eb = char_to_byte(line, end.column);
        line.replace_range(sb..eb, "");
    } else {
        let start_byte = char_to_byte(&lines[start.line], start.column);
        let end_byte = char_to_byte(&lines[end.line], end.column);
        let remainder = lines[end.line][end_byte..].to_string();

        lines[start.line].truncate(start_byte);
        lines[start.line].push_str(&remainder);
        lines.drain(start.line + 1..=end.line);
    }
    *cursor = start;
}

/// Returns the `(start_column, end_column)` selection extent on `line_index`,
/// if the selection `[anchor, cursor)` (order-independent) touches that line.
fn selection_on_line(
    anchor: CursorPos,
    cursor: CursorPos,
    line_index: usize,
) -> Option<(usize, usize)> {
    let (start, end) = if (anchor.line, anchor.column) <= (cursor.line, cursor.column) {
        (anchor, cursor)
    } else {
        (cursor, anchor)
    };
    if line_index < start.line || line_index > end.line {
        return None;
    }
    let start_col = if line_index == start.line {
        start.column
    } else {
        0
    };
    let end_col = if line_index == end.line {
        end.column
    } else {
        usize::MAX
    };
    Some((start_col, end_col))
}

/// Byte offset (into the whole buffer text) that each line starts at.
fn line_start_byte_offsets(text: &str) -> Vec<usize> {
    let mut offsets = vec![0usize];
    for (i, b) in text.bytes().enumerate() {
        if b == b'\n' {
            offsets.push(i + 1);
        }
    }
    offsets
}

fn byte_to_cursor_pos(byte: usize, line_starts: &[usize]) -> CursorPos {
    let line = match line_starts.binary_search(&byte) {
        Ok(i) => i,
        Err(i) => i.saturating_sub(1),
    };
    let column_bytes = byte - line_starts[line];
    // `column_bytes` is a byte offset within the line; callers only use this
    // for highlight span comparison against char-column selection math via
    // `line_color_segments`, which re-derives char columns from byte offsets
    // consistently, so this stays correct for non-ASCII content too.
    CursorPos {
        line,
        column: column_bytes,
    }
}

/// Splits `line` into `(start_col, end_col, color)` runs for drawing, given
/// the highlight spans that overlap `line_index`. Gaps between highlights
/// (and the whole line, if none overlap) use `default_color`.
fn line_color_segments(
    line: &str,
    line_index: usize,
    highlights: &[Highlight],
    default_color: Color,
) -> Vec<(usize, usize, Color)> {
    let char_count = line.chars().count();
    let mut boundaries: Vec<usize> = vec![0, char_count];
    let mut applicable: Vec<(usize, usize, Color)> = Vec::new();

    for h in highlights {
        if line_index < h.start.line || line_index > h.end.line {
            continue;
        }
        let start_col = if line_index == h.start.line {
            byte_column_to_char_column(line, h.start.column)
        } else {
            0
        };
        let end_col = if line_index == h.end.line {
            byte_column_to_char_column(line, h.end.column)
        } else {
            char_count
        };
        if start_col >= end_col {
            continue;
        }
        boundaries.push(start_col);
        boundaries.push(end_col.min(char_count));
        applicable.push((start_col, end_col.min(char_count), h.color));
    }

    boundaries.sort_unstable();
    boundaries.dedup();

    let mut segments = Vec::new();
    for window in boundaries.windows(2) {
        let (a, b) = (window[0], window[1]);
        if a >= b {
            continue;
        }
        let color = applicable
            .iter()
            .rev()
            .find(|(s, e, _)| *s <= a && b <= *e)
            .map(|(_, _, c)| *c)
            .unwrap_or(default_color);
        segments.push((a, b, color));
    }
    segments
}

/// `byte_to_cursor_pos` stores a byte offset in `CursorPos::column` for
/// highlight spans (see its doc comment); this converts that byte offset,
/// for one specific line's text, into a char column for drawing/selection
/// math, which all use char columns.
fn byte_column_to_char_column(line: &str, byte_column: usize) -> usize {
    line.char_indices()
        .position(|(i, _)| i >= byte_column)
        .unwrap_or(line.chars().count())
}

struct HighlightColors {
    keyword: Color,
    string: Color,
    comment: Color,
    number: Color,
    ty: Color,
    function: Color,
    property: Color,
}

impl HighlightColors {
    fn from_theme() -> Self {
        // A fixed, reasonably dark-theme-oriented palette for v1 -- worth
        // promoting to `Theme` fields once more than one editor consumer
        // (MKIDE) needs to customize it.
        Self {
            keyword: Color::from_rgb_u8(198, 120, 221),
            string: Color::from_rgb_u8(152, 195, 121),
            comment: Color::from_rgb_u8(110, 118, 129),
            number: Color::from_rgb_u8(209, 154, 102),
            ty: Color::from_rgb_u8(224, 175, 104),
            function: Color::from_rgb_u8(97, 175, 239),
            property: Color::from_rgb_u8(224, 108, 117),
        }
    }

    fn for_capture(&self, name: &str) -> Option<Color> {
        match name {
            "keyword" => Some(self.keyword),
            "string" => Some(self.string),
            "comment" => Some(self.comment),
            "number" => Some(self.number),
            "type" => Some(self.ty),
            "function" => Some(self.function),
            "property" => Some(self.property),
            _ => None,
        }
    }
}

const RUST_HIGHLIGHT_QUERY: &str = r#"
(line_comment) @comment
(block_comment) @comment
(string_literal) @string
(char_literal) @string
(integer_literal) @number
(float_literal) @number
(type_identifier) @type
(primitive_type) @type
(field_identifier) @property
(function_item name: (identifier) @function)
(call_expression function: (identifier) @function)
(call_expression function: (field_expression field: (field_identifier) @function))
[
  "fn" "let" "pub" "struct" "impl" "use" "mod" "if" "else" "match" "for"
  "while" "loop" "return" "mut" "const" "static" "trait" "enum" "async"
  "await" "move" "in" "as" "ref" "where" "unsafe" "extern" "crate" "super"
  "dyn" "break" "continue" "true" "false" "self" "Self"
] @keyword
"#;
