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
    /// `.x` = horizontal scroll in points, `.y` = vertical scroll in points
    /// (not lines -- see [`Self::visible_line_window`] for how that maps
    /// back to a first-visible-line-plus-pixel-remainder for smooth,
    /// not line-snapped, scrolling).
    scroll_offset: RwLock<Point>,
    /// Width of the widest line in the buffer, in points -- the horizontal
    /// scrollbar's extent. Only recomputed when `content_width_dirty` is
    /// set (by `reparse`, i.e. once per edit) rather than every `draw` --
    /// scanning every line's text width is a real cost for a few-hundred-
    /// line file, and doing it 60 times a second regardless of whether the
    /// buffer changed was the direct cause of scrolling feeling sluggish.
    content_width: RwLock<f32>,
    content_width_dirty: RwLock<bool>,
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
    /// Remainder text of a pending inline suggestion (what Tab would
    /// insert), if any -- ghost-rendered by `draw_overlay` and accepted by
    /// `handle_key`'s `Tab` branch. `suggestion_pos` is the cursor position
    /// it was computed for; anything that moves the cursor away from that
    /// exact position invalidates it (checked before ever accepting or
    /// drawing it) rather than leaving stale ghost text or letting Tab
    /// insert text at the wrong place.
    suggestion: RwLock<Option<String>>,
    suggestion_pos: RwLock<Option<CursorPos>>,

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
    scrollbar_color: Color,
    scrollbar_hover_color: Color,
    scrollbar_width: f32,
    font_size: f32,
    line_height: f32,
    gutter_width: f32,
    width: f32,
    height: RwLock<f32>,
    /// Vertical stretch factor -- see `stretch_y`'s doc comment.
    stretch_y: f32,
    enabled: bool,
    on_change: Option<TextChangeCallback>,
    dragging_v: RwLock<bool>,
    dragging_h: RwLock<bool>,
    drag_start: RwLock<Point>,
    drag_start_scroll: RwLock<Point>,
    /// Buffer position of the most recent mouse-down on the text (not a
    /// scrollbar thumb) -- becomes the selection anchor the first time a
    /// drag is actually detected (see `handle_drag`), so a plain click with
    /// no movement never creates a zero-width "selection" (which would
    /// otherwise still paint a thin highlight, since the selection-drawing
    /// code gives even an empty range a minimum visible width).
    pending_selection_start: RwLock<Option<CursorPos>>,
}

impl CodeEditor {
    /// Creates a new code editor with Rust syntax highlighting.
    pub fn new() -> Self {
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
        Self::with_query(parser, query)
    }

    /// Creates a code editor with no syntax highlighting at all -- for
    /// content that isn't Rust (this element only ever had one hardcoded
    /// grammar, per its own module doc comment) but still wants every
    /// other `CodeEditor` capability: multi-line editing, undo/redo,
    /// find, scrolling. `reparse` (see its own doc comment) already
    /// early-returns when `query` is `None`, so this is just "never give
    /// it a query," not a separate code path -- `highlights` simply stays
    /// empty regardless of what's typed.
    pub fn plain() -> Self {
        Self::with_query(tree_sitter::Parser::new(), None)
    }

    fn with_query(parser: tree_sitter::Parser, query: Option<tree_sitter::Query>) -> Self {
        let theme = get_theme();
        let editor = Self {
            lines: RwLock::new(vec![String::new()]),
            cursor: RwLock::new(CursorPos::default()),
            selection_anchor: RwLock::new(None),
            scroll_offset: RwLock::new(Point::zero()),
            content_width: RwLock::new(0.0),
            content_width_dirty: RwLock::new(true),
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
            suggestion: RwLock::new(None),
            suggestion_pos: RwLock::new(None),
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
            scrollbar_color: theme.scrollbar_color,
            scrollbar_hover_color: theme.scrollbar_color.level(1.3),
            scrollbar_width: theme.scrollbar_width,
            font_size: theme.text_box_font_size,
            line_height: theme.text_box_font_size * 1.4,
            gutter_width: 48.0,
            width: 600.0,
            height: RwLock::new(400.0),
            stretch_y: 1.0,
            enabled: true,
            on_change: None,
            dragging_v: RwLock::new(false),
            dragging_h: RwLock::new(false),
            drag_start: RwLock::new(Point::zero()),
            drag_start_scroll: RwLock::new(Point::zero()),
            pending_selection_start: RwLock::new(None),
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
        self.height = RwLock::new(height);
        self
    }

    /// Returns the current height (see `set_height`).
    pub fn get_height(&self) -> f32 {
        *self.height.read().unwrap()
    }

    /// Adjusts the height at runtime, e.g. from a `Splitter`'s drag
    /// callback. Clamped to a small minimum so a drag can't collapse the
    /// editor to nothing.
    pub fn set_height(&self, height: f32) {
        *self.height.write().unwrap() = height.max(40.0);
    }

    /// Sets the vertical stretch factor (default `1.0`, matching every
    /// other stretchy element). Set this to `0.0` for an editor whose
    /// height should be driven *only* by `set_height` (e.g. a `Splitter`)
    /// and never by a `VTile` sibling competing for "extra" space --
    /// without this, a log panel with equal stretch to its stretchy
    /// neighbor only moved at half the speed of the mouse while dragging
    /// their shared splitter (both siblings split the delta), which read
    /// as the drag being broken/capped rather than just slow.
    pub fn stretch_y(mut self, stretch_y: f32) -> Self {
        self.stretch_y = stretch_y;
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
        *self.scroll_offset.write().unwrap() = Point::zero();
        self.reparse();
    }

    fn push_undo_snapshot(&self) {
        let snapshot = self.lines.read().unwrap().clone();
        self.undo_stack.write().unwrap().push(snapshot);
        self.redo_stack.write().unwrap().clear();
    }

    /// Appends one line to the end of the buffer, without touching the
    /// cursor, selection, or undo history -- built for read-only log
    /// panels (e.g. MKIDE's build/run/test/debug output) that get many
    /// small appends as a process streams output, rather than being edited
    /// by hand. Scrolls to the bottom afterward so newly streamed lines
    /// stay visible instead of silently landing off-screen.
    pub fn append_line(&self, line: &str) {
        {
            let mut lines = self.lines.write().unwrap();
            // The buffer starts as one empty-string line (see `new()`) --
            // replace that placeholder instead of leaving a stray blank
            // line before the first real one.
            if lines.len() == 1 && lines[0].is_empty() {
                lines[0] = line.to_string();
            } else {
                lines.push(line.to_string());
            }
        }
        self.reparse();
        self.scroll_to_bottom();
    }

    /// Scrolls to the last line. Uses the editor's own configured height
    /// as an approximation of the actual rendered viewport height (exact
    /// only when no horizontal scrollbar is showing to shave a few points
    /// off it) since no `Context` is available outside of draw/event
    /// handling -- close enough to reliably reveal the last few lines.
    pub fn scroll_to_bottom(&self) {
        let content_height = self.content_height();
        let viewport_height = *self.height.read().unwrap();
        let max_y = (content_height - viewport_height).max(0.0);
        self.scroll_offset.write().unwrap().y = max_y;
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
        // whether tree-sitter highlighting itself is available below. Also
        // the one place that marks the cached content width stale -- see
        // `content_width`'s doc comment.
        self.recompute_find_matches();
        *self.content_width_dirty.write().unwrap() = true;

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
        self.recompute_local_suggestion();
    }

    /// Built-in, model-free inline suggestion: matches the identifier
    /// fragment immediately before the cursor against distinct identifiers
    /// already in the buffer and a small static Rust keyword list, favoring
    /// buffer-local matches as more contextually relevant. Runs from
    /// `reparse` (i.e. after every edit, not per-frame -- same cost model as
    /// highlighting). Callers that want a smarter suggestion (e.g. an LLM)
    /// can overwrite whatever this produces via `set_suggestion`.
    fn recompute_local_suggestion(&self) {
        let lines = self.lines.read().unwrap();
        let cursor = *self.cursor.read().unwrap();
        let Some(line) = lines.get(cursor.line) else {
            return;
        };
        let chars: Vec<char> = line.chars().collect();
        let prefix_start = chars[..cursor.column.min(chars.len())]
            .iter()
            .rposition(|c| !is_ident_char(*c))
            .map(|i| i + 1)
            .unwrap_or(0);
        let prefix: String = chars[prefix_start..cursor.column.min(chars.len())]
            .iter()
            .collect();
        drop(lines);

        if prefix.chars().count() < 2 {
            *self.suggestion.write().unwrap() = None;
            *self.suggestion_pos.write().unwrap() = None;
            return;
        }

        let candidate = self
            .buffer_identifier_completing(&prefix, cursor)
            .or_else(|| {
                RUST_KEYWORDS
                    .iter()
                    .find(|kw| kw.starts_with(prefix.as_str()) && **kw != prefix)
                    .map(|kw| kw.to_string())
            });

        match candidate {
            Some(candidate) => {
                let remainder = candidate[prefix.len()..].to_string();
                *self.suggestion.write().unwrap() = Some(remainder);
                *self.suggestion_pos.write().unwrap() = Some(cursor);
            }
            None => {
                *self.suggestion.write().unwrap() = None;
                *self.suggestion_pos.write().unwrap() = None;
            }
        }
    }

    /// Finds a distinct identifier elsewhere in the buffer that starts with
    /// `prefix` and isn't just `prefix` itself, skipping the occurrence at
    /// `cursor` (the one currently being typed).
    fn buffer_identifier_completing(&self, prefix: &str, cursor: CursorPos) -> Option<String> {
        let lines = self.lines.read().unwrap();
        for (line_idx, line) in lines.iter().enumerate() {
            let chars: Vec<char> = line.chars().collect();
            let mut start = 0;
            while start < chars.len() {
                if !is_ident_char(chars[start]) {
                    start += 1;
                    continue;
                }
                let mut end = start;
                while end < chars.len() && is_ident_char(chars[end]) {
                    end += 1;
                }
                let is_cursor_word =
                    line_idx == cursor.line && start <= cursor.column && cursor.column <= end;
                if !is_cursor_word {
                    let word: String = chars[start..end].iter().collect();
                    if word.len() > prefix.len() && word.starts_with(prefix) {
                        return Some(word);
                    }
                }
                start = end;
            }
        }
        None
    }

    /// If a pending suggestion's recorded position still matches the
    /// current cursor, clears and returns its remainder text (for `Tab` to
    /// insert); otherwise leaves state untouched and returns `None`.
    fn take_matching_suggestion(&self) -> Option<String> {
        let matches = *self.suggestion_pos.read().unwrap() == Some(*self.cursor.read().unwrap());
        if !matches {
            return None;
        }
        self.suggestion_pos.write().unwrap().take();
        self.suggestion.write().unwrap().take()
    }

    /// Overwrites (or clears, with `None`) the currently pending suggestion
    /// -- for external callers (e.g. an LLM-backed completion source) to
    /// supply a better suggestion than the built-in local one. Only takes
    /// effect if the cursor hasn't moved since this call (checked at accept/
    /// draw time via `suggestion_pos`, still set to wherever it was last
    /// computed).
    pub fn set_suggestion(&self, text: Option<String>) {
        if text.is_some() {
            *self.suggestion_pos.write().unwrap() = Some(*self.cursor.read().unwrap());
        }
        *self.suggestion.write().unwrap() = text;
    }

    /// The current line's text up to (not including) the cursor -- e.g. for
    /// building an LLM completion prompt from outside this element.
    pub fn cursor_line_prefix(&self) -> String {
        let lines = self.lines.read().unwrap();
        let cursor = *self.cursor.read().unwrap();
        let Some(line) = lines.get(cursor.line) else {
            return String::new();
        };
        line.chars().take(cursor.column).collect()
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

    /// Copies the current selection to the system clipboard, if any --
    /// a no-op with nothing selected (matches every other text editor's
    /// Cmd+C behavior of leaving the clipboard untouched rather than
    /// clearing it).
    fn copy_selection(&self) {
        let anchor = *self.selection_anchor.read().unwrap();
        let Some(sel) = anchor else { return };
        let lines = self.lines.read().unwrap();
        let cursor = *self.cursor.read().unwrap();
        crate::host::set_clipboard(&extract_range(&lines, sel, cursor));
    }

    /// Copies the current selection (if any) then deletes it -- `Some`
    /// anchor makes `delete_backward` remove exactly the selection, same
    /// as a plain Delete/Backspace on a selection would.
    fn cut_selection(&self) {
        if self.selection_anchor.read().unwrap().is_none() {
            return;
        }
        self.copy_selection();
        self.delete_backward();
    }

    /// Inserts the clipboard's text at the cursor, replacing the current
    /// selection if any (`insert_text` already does this).
    fn paste_clipboard(&self) {
        let text = crate::host::get_clipboard();
        if !text.is_empty() {
            self.insert_text(&text);
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

    /// Widest line in the buffer, measured with the editor's own font.
    fn measure_content_width(&self, ctx: &Context) -> f32 {
        let mut canvas = ctx.canvas.borrow_mut();
        let theme = get_theme();
        canvas.font(theme.text_box_font);
        canvas.font_size(self.font_size);
        self.lines
            .read()
            .unwrap()
            .iter()
            .map(|l| canvas.text_width(l))
            .fold(0.0, f32::max)
    }

    fn content_height(&self) -> f32 {
        self.lines.read().unwrap().len() as f32 * self.line_height
    }

    /// Whole-editor-bounds check, not the (possibly already-narrowed-by-the-
    /// other-scrollbar) viewport -- avoids the two scrollbars' visibility
    /// depending on each other, matching `ScrollView`'s equivalent tradeoff.
    fn needs_v_scrollbar(&self, ctx: &Context) -> bool {
        self.content_height() > ctx.bounds.height()
    }

    fn needs_h_scrollbar(&self, ctx: &Context) -> bool {
        *self.content_width.read().unwrap() > ctx.bounds.width() - self.gutter_width
    }

    /// Full editor area minus whichever scrollbar(s) are showing.
    fn viewport_rect(&self, ctx: &Context) -> Rect {
        let has_v = self.needs_v_scrollbar(ctx);
        let has_h = self.needs_h_scrollbar(ctx);
        Rect::new(
            ctx.bounds.left,
            ctx.bounds.top,
            ctx.bounds.right - if has_v { self.scrollbar_width } else { 0.0 },
            ctx.bounds.bottom - if has_h { self.scrollbar_width } else { 0.0 },
        )
    }

    /// Where text (not the gutter) is drawn/scrolled/clipped.
    fn text_viewport(&self, ctx: &Context) -> Rect {
        let viewport = self.viewport_rect(ctx);
        Rect::new(
            viewport.left + self.gutter_width,
            viewport.top,
            viewport.right,
            viewport.bottom,
        )
    }

    fn v_scrollbar_rect(&self, ctx: &Context) -> Rect {
        if !self.needs_v_scrollbar(ctx) {
            return Rect::zero();
        }
        let has_h = self.needs_h_scrollbar(ctx);
        Rect::new(
            ctx.bounds.right - self.scrollbar_width,
            ctx.bounds.top,
            ctx.bounds.right,
            ctx.bounds.bottom - if has_h { self.scrollbar_width } else { 0.0 },
        )
    }

    fn h_scrollbar_rect(&self, ctx: &Context) -> Rect {
        if !self.needs_h_scrollbar(ctx) {
            return Rect::zero();
        }
        let has_v = self.needs_v_scrollbar(ctx);
        Rect::new(
            ctx.bounds.left + self.gutter_width,
            ctx.bounds.bottom - self.scrollbar_width,
            ctx.bounds.right - if has_v { self.scrollbar_width } else { 0.0 },
            ctx.bounds.bottom,
        )
    }

    fn v_thumb_rect(&self, ctx: &Context) -> Rect {
        let track = self.v_scrollbar_rect(ctx);
        if track.is_empty() {
            return Rect::zero();
        }
        let content_height = self.content_height();
        let viewport = self.text_viewport(ctx);
        let scroll_y = self.scroll_offset.read().unwrap().y;

        let visible_ratio = (viewport.height() / content_height).min(1.0);
        let thumb_height = (track.height() * visible_ratio).max(20.0);
        let scroll_range = (content_height - viewport.height()).max(0.0);
        let scroll_ratio = if scroll_range > 0.0 {
            scroll_y / scroll_range
        } else {
            0.0
        };
        let thumb_y = track.top + scroll_ratio * (track.height() - thumb_height);

        Rect::new(
            track.left + 2.0,
            thumb_y,
            track.right - 2.0,
            thumb_y + thumb_height,
        )
    }

    fn h_thumb_rect(&self, ctx: &Context) -> Rect {
        let track = self.h_scrollbar_rect(ctx);
        if track.is_empty() {
            return Rect::zero();
        }
        let content_width = *self.content_width.read().unwrap();
        let viewport = self.text_viewport(ctx);
        let scroll_x = self.scroll_offset.read().unwrap().x;

        let visible_ratio = (viewport.width() / content_width).min(1.0);
        let thumb_width = (track.width() * visible_ratio).max(20.0);
        let scroll_range = (content_width - viewport.width()).max(0.0);
        let scroll_ratio = if scroll_range > 0.0 {
            scroll_x / scroll_range
        } else {
            0.0
        };
        let thumb_x = track.left + scroll_ratio * (track.width() - thumb_width);

        Rect::new(
            thumb_x,
            track.top + 2.0,
            thumb_x + thumb_width,
            track.bottom - 2.0,
        )
    }

    fn draw_scrollbars(&self, ctx: &Context) {
        let mut canvas = ctx.canvas.borrow_mut();

        if self.needs_v_scrollbar(ctx) {
            let track = self.v_scrollbar_rect(ctx);
            let thumb = self.v_thumb_rect(ctx);
            canvas.fill_style(self.scrollbar_color.with_alpha(0.2));
            canvas.fill_rect(track);
            let color = if *self.dragging_v.read().unwrap() {
                self.scrollbar_hover_color
            } else {
                self.scrollbar_color
            };
            canvas.fill_style(color);
            canvas.fill_round_rect(thumb, 3.0);
        }

        if self.needs_h_scrollbar(ctx) {
            let track = self.h_scrollbar_rect(ctx);
            let thumb = self.h_thumb_rect(ctx);
            canvas.fill_style(self.scrollbar_color.with_alpha(0.2));
            canvas.fill_rect(track);
            let color = if *self.dragging_h.read().unwrap() {
                self.scrollbar_hover_color
            } else {
                self.scrollbar_color
            };
            canvas.fill_style(color);
            canvas.fill_round_rect(thumb, 3.0);
        }

        if self.needs_v_scrollbar(ctx) && self.needs_h_scrollbar(ctx) {
            let corner = Rect::new(
                ctx.bounds.right - self.scrollbar_width,
                ctx.bounds.bottom - self.scrollbar_width,
                ctx.bounds.right,
                ctx.bounds.bottom,
            );
            canvas.fill_style(self.scrollbar_color.with_alpha(0.3));
            canvas.fill_rect(corner);
        }
    }

    /// Clamps and stores a new scroll position against the current content
    /// size -- the single place both wheel-scroll and scrollbar-thumb-drag
    /// funnel through, so neither can push the view past the buffer's
    /// actual extent.
    fn set_scroll(&self, ctx: &Context, x: f32, y: f32) {
        let content_width = *self.content_width.read().unwrap();
        let content_height = self.content_height();
        let viewport = self.text_viewport(ctx);
        let max_x = (content_width - viewport.width()).max(0.0);
        let max_y = (content_height - viewport.height()).max(0.0);
        *self.scroll_offset.write().unwrap() = Point::new(x.clamp(0.0, max_x), y.clamp(0.0, max_y));
    }

    /// Nudges scroll (both axes) just enough to bring the cursor back
    /// inside the visible viewport, without moving it more than necessary
    /// (a cursor already in view is left alone). Called after every
    /// key-driven cursor move/edit -- see `handle_key`/`handle_text`.
    fn scroll_cursor_into_view(&self, ctx: &Context) {
        let cursor = *self.cursor.read().unwrap();
        let scroll = *self.scroll_offset.read().unwrap();
        let viewport = self.text_viewport(ctx);

        let cursor_top = cursor.line as f32 * self.line_height;
        let cursor_bottom = cursor_top + self.line_height;
        let new_y = if cursor_top < scroll.y {
            cursor_top
        } else if cursor_bottom > scroll.y + viewport.height() {
            cursor_bottom - viewport.height()
        } else {
            scroll.y
        };

        let cursor_x = {
            let lines = self.lines.read().unwrap();
            let mut canvas = ctx.canvas.borrow_mut();
            let theme = get_theme();
            canvas.font(theme.text_box_font);
            canvas.font_size(self.font_size);
            canvas.text_width_to_position(&lines[cursor.line], cursor.column)
        };
        let new_x = if cursor_x < scroll.x {
            cursor_x
        } else if cursor_x > scroll.x + viewport.width() {
            cursor_x - viewport.width()
        } else {
            scroll.x
        };

        self.set_scroll(ctx, new_x, new_y);
    }

    fn draw_gutter(
        &self,
        ctx: &Context,
        first_visible_line: usize,
        visible_lines: usize,
        line_offset: f32,
    ) {
        let mut canvas = ctx.canvas.borrow_mut();
        let gutter_rect = Rect::new(
            ctx.bounds.left,
            ctx.bounds.top,
            ctx.bounds.left + self.gutter_width,
            self.viewport_rect(ctx).bottom,
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
            let y = ctx.bounds.top + (row as f32 + 1.0) * self.line_height
                - self.line_height * 0.3
                - line_offset;

            if let Some(severity) = self.diagnostic_severity_for_line(line_index) {
                canvas.fill_style(self.severity_color(severity));
                let dot_y = y - self.font_size * 0.35;
                canvas.fill_round_rect(
                    Rect::new(
                        ctx.bounds.left + 4.0,
                        dot_y - 3.0,
                        ctx.bounds.left + 10.0,
                        dot_y + 3.0,
                    ),
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
        line_offset: f32,
    ) {
        let text_viewport = self.text_viewport(ctx);
        let mut canvas = ctx.canvas.borrow_mut();
        let theme = get_theme();
        canvas.font(theme.text_box_font);
        canvas.font_size(self.font_size);
        canvas.save();
        canvas.clip(text_viewport);

        let lines = self.lines.read().unwrap();
        let cursor = *self.cursor.read().unwrap();
        let anchor = *self.selection_anchor.read().unwrap();
        let highlights = self.highlights.read().unwrap();
        let find_matches = self.find_matches.read().unwrap();
        let query_len = self.find_query.read().unwrap().chars().count();
        let scroll_x = self.scroll_offset.read().unwrap().x;
        let text_left = ctx.bounds.left + self.gutter_width + 6.0 - scroll_x;

        for row in 0..visible_lines {
            let line_index = first_visible_line + row;
            if line_index >= lines.len() {
                break;
            }
            let line = &lines[line_index];
            let y_top = ctx.bounds.top + row as f32 * self.line_height - line_offset;
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
                        + canvas.text_width_to_position(
                            line,
                            (m.column + query_len).min(line.chars().count()),
                        );
                    canvas.fill_style(self.find_match_color);
                    canvas.fill_rect(Rect::new(
                        x1,
                        y_top,
                        x2.max(x1 + 2.0),
                        y_top + self.line_height,
                    ));
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

        canvas.restore();
    }

    /// Returns `(first_visible_line, visible_line_count, line_offset)`.
    /// `line_offset` is the sub-line-height pixel remainder of the vertical
    /// scroll (`scroll.y - first_visible_line * line_height`), applied as a
    /// y-offset when drawing so scrolling is smooth rather than snapped to
    /// whole lines.
    fn visible_line_window(&self, ctx: &Context) -> (usize, usize, f32) {
        let scroll_y = self.scroll_offset.read().unwrap().y;
        let first = (scroll_y / self.line_height).floor().max(0.0) as usize;
        let line_offset = scroll_y - first as f32 * self.line_height;
        let visible = (self.text_viewport(ctx).height() / self.line_height).ceil() as usize + 1;
        (first, visible, line_offset)
    }

    fn cursor_pos_from_click(&self, ctx: &Context, p: Point) -> CursorPos {
        let lines = self.lines.read().unwrap();
        let scroll = *self.scroll_offset.read().unwrap();
        let row = (((p.y - ctx.bounds.top + scroll.y) / self.line_height)
            .floor()
            .max(0.0)) as usize;
        let line = row.min(lines.len().saturating_sub(1));
        let line_text = &lines[line];

        let mut canvas = ctx.canvas.borrow_mut();
        let theme = get_theme();
        canvas.font(theme.text_box_font);
        canvas.font_size(self.font_size);
        let text_left = ctx.bounds.left + self.gutter_width + 6.0 - scroll.x;
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
        ViewLimits::min_size(self.width, *self.height.read().unwrap())
    }

    fn stretch(&self) -> ViewStretch {
        ViewStretch::new(1.0, self.stretch_y)
    }

    fn draw(&self, ctx: &Context) {
        {
            let mut canvas = ctx.canvas.borrow_mut();
            canvas.fill_style(self.background_color);
            canvas.fill_rect(ctx.bounds);
        }
        if *self.content_width_dirty.read().unwrap() {
            *self.content_width.write().unwrap() = self.measure_content_width(ctx);
            *self.content_width_dirty.write().unwrap() = false;
        }
        let (first, visible, line_offset) = self.visible_line_window(ctx);
        self.draw_selection_and_text(ctx, first, visible, line_offset);
        self.draw_gutter(ctx, first, visible, line_offset);
        self.draw_scrollbars(ctx);
    }

    // Ghost-renders a pending suggestion's remainder text right after the
    // caret, dimmed so it reads as a suggestion rather than real buffer
    // content. A real overlay pass (not embedded in `draw`) so it always
    // paints on top of everything else in the tree, including popups from
    // sibling/ancestor elements -- see `Element::draw_overlay`'s doc comment
    // and this crate's overlay z-order fix from earlier this session.
    fn draw_overlay(&self, ctx: &Context) {
        let Some(suggestion) = self.suggestion.read().unwrap().clone() else {
            return;
        };
        let cursor = *self.cursor.read().unwrap();
        if *self.suggestion_pos.read().unwrap() != Some(cursor)
            || *self.state.read().unwrap() != EditorState::Focused
        {
            return;
        }

        let (first_visible_line, visible_lines, line_offset) = self.visible_line_window(ctx);
        if cursor.line < first_visible_line || cursor.line >= first_visible_line + visible_lines {
            return;
        }

        let lines = self.lines.read().unwrap();
        let Some(line) = lines.get(cursor.line) else {
            return;
        };
        let scroll_x = self.scroll_offset.read().unwrap().x;
        let text_left = ctx.bounds.left + self.gutter_width + 6.0 - scroll_x;
        let text_viewport = self.text_viewport(ctx);

        let mut canvas = ctx.canvas.borrow_mut();
        let theme = get_theme();
        canvas.font(theme.text_box_font);
        canvas.font_size(self.font_size);
        canvas.save();
        canvas.clip(text_viewport);

        let row = cursor.line - first_visible_line;
        let y_top = ctx.bounds.top + row as f32 * self.line_height - line_offset;
        let y_baseline = y_top + self.line_height - self.font_size * 0.3;
        let x = text_left + canvas.text_width_to_position(line, cursor.column);

        canvas.fill_style(self.text_color.with_alpha(0.4));
        canvas.fill_text(&suggestion, Point::new(x, y_baseline));

        canvas.restore();
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
            if self.v_thumb_rect(ctx).contains(btn.pos) {
                *self.dragging_v.write().unwrap() = true;
                *self.drag_start.write().unwrap() = btn.pos;
                *self.drag_start_scroll.write().unwrap() = *self.scroll_offset.read().unwrap();
                return true;
            }
            if self.h_thumb_rect(ctx).contains(btn.pos) {
                *self.dragging_h.write().unwrap() = true;
                *self.drag_start.write().unwrap() = btn.pos;
                *self.drag_start_scroll.write().unwrap() = *self.scroll_offset.read().unwrap();
                return true;
            }
            *self.state.write().unwrap() = EditorState::Focused;
            let pos = self.cursor_pos_from_click(ctx, btn.pos);
            // Shift-click extends the existing selection (or starts one
            // from wherever the cursor already was, if there wasn't one)
            // to the click point, the same as a shift+arrow key press --
            // rather than the plain-click behavior of moving the cursor
            // there and dropping any selection.
            if btn.modifiers & crate::view::modifiers::SHIFT != 0 {
                let mut anchor = self.selection_anchor.write().unwrap();
                if anchor.is_none() {
                    *anchor = Some(*self.cursor.read().unwrap());
                }
            } else {
                *self.selection_anchor.write().unwrap() = None;
            }
            *self.cursor.write().unwrap() = pos;
            *self.pending_selection_start.write().unwrap() = Some(pos);
            *self.suggestion.write().unwrap() = None;
            *self.suggestion_pos.write().unwrap() = None;
        } else {
            *self.dragging_v.write().unwrap() = false;
            *self.dragging_h.write().unwrap() = false;
            *self.pending_selection_start.write().unwrap() = None;
        }
        true
    }

    fn drag(&mut self, ctx: &Context, btn: MouseButton) {
        self.handle_drag(ctx, btn);
    }

    fn handle_drag(&self, ctx: &Context, btn: MouseButton) {
        let drag_start = *self.drag_start.read().unwrap();
        let start_scroll = *self.drag_start_scroll.read().unwrap();

        if *self.dragging_v.read().unwrap() {
            let track = self.v_scrollbar_rect(ctx);
            let thumb = self.v_thumb_rect(ctx);
            let viewport = self.text_viewport(ctx);
            let delta_y = btn.pos.y - drag_start.y;
            let track_range = track.height() - thumb.height();
            let scroll_range = (self.content_height() - viewport.height()).max(0.0);
            if track_range > 0.0 {
                let new_y = start_scroll.y + delta_y * scroll_range / track_range;
                self.set_scroll(ctx, start_scroll.x, new_y);
            }
        }

        if *self.dragging_h.read().unwrap() {
            let track = self.h_scrollbar_rect(ctx);
            let thumb = self.h_thumb_rect(ctx);
            let viewport = self.text_viewport(ctx);
            let content_width = *self.content_width.read().unwrap();
            let delta_x = btn.pos.x - drag_start.x;
            let track_range = track.width() - thumb.width();
            let scroll_range = (content_width - viewport.width()).max(0.0);
            if track_range > 0.0 {
                let new_x = start_scroll.x + delta_x * scroll_range / track_range;
                self.set_scroll(ctx, new_x, start_scroll.y);
            }
        }

        // Click-drag text selection: a mouse-down on the text (not a
        // scrollbar thumb) records where it landed in `pending_selection_
        // start` but doesn't touch `selection_anchor` itself, so a plain
        // click with no movement never creates a zero-width "selection"
        // (the drawing code gives even an empty range a minimum visible
        // width, which would otherwise show a stray highlight sliver on
        // every click). The anchor is only established here, the first
        // time a drag actually happens, then left in place for every
        // subsequent drag tick while the cursor tracks the pointer.
        if !*self.dragging_v.read().unwrap() && !*self.dragging_h.read().unwrap() {
            if let Some(start) = *self.pending_selection_start.read().unwrap() {
                let mut anchor = self.selection_anchor.write().unwrap();
                if anchor.is_none() {
                    *anchor = Some(start);
                }
                drop(anchor);
                *self.cursor.write().unwrap() = self.cursor_pos_from_click(ctx, btn.pos);
                *self.suggestion.write().unwrap() = None;
                *self.suggestion_pos.write().unwrap() = None;
            }
        }
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
        let scroll = *self.scroll_offset.read().unwrap();
        self.set_scroll(ctx, scroll.x - dir.x, scroll.y - dir.y);
        true
    }

    fn key(&mut self, ctx: &Context, k: KeyInfo) -> bool {
        self.handle_key(ctx, k)
    }

    fn handle_key(&self, ctx: &Context, k: KeyInfo) -> bool {
        if !self.enabled || *self.state.read().unwrap() != EditorState::Focused {
            return false;
        }
        if k.action != crate::view::KeyAction::Press && k.action != crate::view::KeyAction::Repeat {
            return true;
        }

        let shift = k.modifiers & crate::view::modifiers::SHIFT != 0;
        let ctrl =
            k.modifiers & (crate::view::modifiers::CONTROL | crate::view::modifiers::SUPER) != 0;

        // Tab accepts a pending suggestion (see `take_matching_suggestion`)
        // instead of indenting, but only while the cursor is still exactly
        // where the suggestion was computed for -- anything else (cursor
        // moved on since, or none pending) falls through to the plain
        // 4-space insert below, unchanged.
        if k.key == KeyCode::Tab && !shift && !ctrl {
            if let Some(remainder) = self.take_matching_suggestion() {
                self.insert_text(&remainder);
                self.scroll_cursor_into_view(ctx);
                return true;
            }
        }

        // Any other key invalidates whatever suggestion was showing --
        // `reparse` (called from the edit paths below) recomputes a fresh
        // one where that makes sense, so this only has a lasting effect for
        // cursor-movement keys, which should clear stale ghost text rather
        // than leave it pointing at a position that no longer matches.
        *self.suggestion.write().unwrap() = None;
        *self.suggestion_pos.write().unwrap() = None;

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
            KeyCode::C if ctrl => self.copy_selection(),
            KeyCode::X if ctrl => self.cut_selection(),
            KeyCode::V if ctrl => self.paste_clipboard(),
            _ => return false,
        }
        // Without this, moving the cursor past whatever's currently
        // scrolled into view (e.g. holding Down past the bottom line, or
        // End on a line wider than the viewport) left the caret invisible
        // off-screen with nothing on screen changing -- indistinguishable
        // from arrow keys simply not working.
        self.scroll_cursor_into_view(ctx);
        true
    }

    fn text(&mut self, ctx: &Context, info: TextInfo) -> bool {
        self.handle_text(ctx, info)
    }

    fn handle_text(&self, ctx: &Context, info: TextInfo) -> bool {
        if !self.enabled || *self.state.read().unwrap() != EditorState::Focused {
            return false;
        }
        let c = info.codepoint;
        if !c.is_control() {
            self.insert_text(&c.to_string());
            self.scroll_cursor_into_view(ctx);
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

/// Creates a code editor with no syntax highlighting -- see
/// [`CodeEditor::plain`].
pub fn code_editor_plain() -> CodeEditor {
    CodeEditor::plain()
}

// --- helpers -----------------------------------------------------------

fn char_to_byte(line: &str, column: usize) -> usize {
    line.char_indices()
        .nth(column)
        .map(|(i, _)| i)
        .unwrap_or(line.len())
}

/// Whether `c` can be part of an identifier word for the built-in local
/// suggestion engine -- deliberately just `[A-Za-z0-9_]`, matching Rust
/// identifier syntax closely enough for prefix-matching without needing a
/// real tokenizer.
fn is_ident_char(c: char) -> bool {
    c.is_alphanumeric() || c == '_'
}

/// Static keyword list for the built-in local suggestion engine's fallback
/// candidate set, used when no buffer-local identifier completes the typed
/// prefix. Rust-only (this editor only ever has one hardcoded grammar, see
/// the module doc comment) -- not exhaustive, just the common ones worth
/// suggesting.
const RUST_KEYWORDS: &[&str] = &[
    "as", "break", "const", "continue", "crate", "else", "enum", "extern", "false", "fn", "for",
    "if", "impl", "in", "let", "loop", "match", "mod", "move", "mut", "pub", "ref", "return",
    "self", "Self", "static", "struct", "super", "trait", "true", "type", "unsafe", "use", "where",
    "while", "async", "await", "dyn", "String", "Vec", "Option", "Some", "None", "Result", "Ok",
    "Err", "Arc", "RwLock", "Mutex", "Box",
];

/// Returns the text within the (line, column)-addressed range `[a, b)`
/// (order-independent), for copy/cut -- mirrors `delete_range_inner`'s own
/// range math but reads instead of mutating.
fn extract_range(lines: &[String], a: CursorPos, b: CursorPos) -> String {
    let (start, end) = if (a.line, a.column) <= (b.line, b.column) {
        (a, b)
    } else {
        (b, a)
    };

    if start.line == end.line {
        let line = &lines[start.line];
        let sb = char_to_byte(line, start.column);
        let eb = char_to_byte(line, end.column);
        line[sb..eb].to_string()
    } else {
        let mut out = String::new();
        let start_byte = char_to_byte(&lines[start.line], start.column);
        out.push_str(&lines[start.line][start_byte..]);
        for line in &lines[start.line + 1..end.line] {
            out.push('\n');
            out.push_str(line);
        }
        let end_byte = char_to_byte(&lines[end.line], end.column);
        out.push('\n');
        out.push_str(&lines[end.line][..end_byte]);
        out
    }
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

/// A real, pre-existing bug caught while adding `CodeEditor::plain()` (see
/// its own doc comment): `tree_sitter::Query::new` was silently failing to
/// compile this *entire* query, because `"mut"`/`"crate"`/`"super"`/`"self"`
/// aren't bare anonymous tokens in `tree-sitter-rust` 0.23's grammar --
/// confirmed via its own `node-types.json`: `mut` has no token at all
/// (only the named nodes `mutable_specifier`/`mut_pattern` exist), and
/// `crate`/`super`/`self` exist *only* as named nodes (`"named": true`,
/// no anonymous variant), since they can appear as real path segments
/// (`crate::foo`, `self.bar`), not just bare keywords. A bare string
/// literal in a query can only ever match an anonymous token, so all four
/// were rejected as "invalid node type," and tree-sitter fails the whole
/// query over a single bad alternative, not just that one. Since the
/// failure is swallowed by `log::warn!` (a no-op with no logger installed,
/// e.g. under `cargo test`) and turns into a silently empty `Option<Query>`
/// (see `CodeEditor::new`), Rust syntax highlighting had been completely
/// non-functional -- `query` was always `None`, `reparse` always
/// early-returned, `highlights` was always empty -- with nothing visibly
/// wrong short of noticing the editor never actually highlighted anything.
/// Fixed by matching each as its own named-node pattern instead of a bare
/// string in the keyword list (`"Self"` was dropped outright: it has no
/// node-types.json entry at all in this grammar, named or anonymous --
/// it's just tokenized as a regular `type_identifier`/`identifier`, which
/// already gets `@type` coloring via the pattern above).
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
(mutable_specifier) @keyword
(crate) @keyword
(super) @keyword
(self) @keyword
[
  "fn" "let" "pub" "struct" "impl" "use" "mod" "if" "else" "match" "for"
  "while" "loop" "return" "const" "static" "trait" "enum" "async"
  "await" "move" "in" "as" "ref" "where" "unsafe" "extern"
  "dyn" "break" "continue" "true" "false"
] @keyword
"#;

#[cfg(test)]
mod editor_interaction_tests {
    use super::*;
    use crate::support::canvas::Canvas;
    use crate::view::{MouseButtonKind, TextInfo};
    use std::cell::RefCell;

    fn click_and_type(editor: &CodeEditor, click_pos: Point, text: &str) {
        let view = crate::view::View::new(crate::support::point::Extent::new(700.0, 400.0));
        let canvas = RefCell::new(Canvas::new(700, 400).unwrap());
        let bounds = Rect::new(0.0, 0.0, 700.0, 400.0);
        let ctx = Context::new(&view, &canvas, bounds);

        assert!(
            editor.hit_test(&ctx, click_pos, false, false).is_some(),
            "hit_test should find the editor at {click_pos:?}"
        );

        let down = MouseButton {
            down: true,
            click_count: 1,
            button: MouseButtonKind::Left,
            modifiers: 0,
            pos: click_pos,
        };
        assert!(
            editor.handle_click(&ctx, down),
            "mouse-down should be handled"
        );

        let up = MouseButton {
            down: false,
            ..down
        };
        editor.handle_click(&ctx, up);

        for c in text.chars() {
            let handled = editor.handle_text(
                &ctx,
                TextInfo {
                    codepoint: c,
                    modifiers: 0,
                },
            );
            assert!(handled, "handle_text should accept '{c}' once focused");
        }
    }

    #[test]
    fn click_then_type_inserts_text() {
        let editor = CodeEditor::new().text("");
        click_and_type(&editor, Point::new(60.0, 10.0), "hi");
        assert_eq!(editor.get_text(), "hi");
    }

    #[test]
    fn click_then_arrow_keys_move_cursor() {
        let editor = CodeEditor::new().text("hello");
        let view = crate::view::View::new(crate::support::point::Extent::new(700.0, 400.0));
        let canvas = RefCell::new(Canvas::new(700, 400).unwrap());
        let bounds = Rect::new(0.0, 0.0, 700.0, 400.0);
        let ctx = Context::new(&view, &canvas, bounds);

        let down = MouseButton {
            down: true,
            click_count: 1,
            button: MouseButtonKind::Left,
            modifiers: 0,
            pos: Point::new(60.0, 10.0),
        };
        editor.handle_click(&ctx, down);

        let before = *editor.cursor.read().unwrap();
        let key = KeyInfo {
            key: KeyCode::Left,
            action: crate::view::KeyAction::Press,
            modifiers: 0,
        };
        assert!(
            editor.handle_key(&ctx, key),
            "Left arrow should be handled once focused"
        );
        let after = *editor.cursor.read().unwrap();
        assert_ne!(before, after, "cursor should move after pressing Left");
    }

    /// Reproduces the "splitter drag only moves the panel at half speed"
    /// bug: an output-log `CodeEditor` sitting in a `VTile` next to a
    /// stretchy sibling, with the *default* stretch (1.0), only grows by
    /// half of whatever `set_height` sets it to -- the sibling's equal
    /// stretch claims the other half of the "extra" space MKIDE's
    /// `Splitter` is trying to hand entirely to the editor being dragged.
    /// `.stretch_y(0.0)` is the fix; this locks in that it actually works
    /// (the rendered height exactly matches `set_height`, not half of it).
    #[test]
    fn stretch_y_zero_makes_rendered_height_track_set_height_exactly() {
        use crate::element::composite::CompositeBase;
        use crate::element::tile::VTile;
        use crate::support::point::Extent;

        struct StretchySibling;
        impl Element for StretchySibling {
            fn limits(&self, _ctx: &BasicContext) -> ViewLimits {
                ViewLimits::min_size(200.0, 300.0)
            }
            fn stretch(&self) -> ViewStretch {
                ViewStretch::new(1.0, 1.0)
            }
            fn as_any(&self) -> &dyn Any {
                self
            }
            fn as_any_mut(&mut self) -> &mut dyn Any {
                self
            }
        }

        let output =
            std::sync::Arc::new(CodeEditor::new().width(200.0).height(90.0).stretch_y(0.0));
        let vtile = VTile::from_vec(vec![
            crate::element::share(StretchySibling),
            output.clone() as crate::element::ElementPtr,
        ]);

        let view = crate::view::View::new(Extent::new(200.0, 600.0));
        let canvas = RefCell::new(Canvas::new(200, 600).unwrap());
        let bounds = Rect::new(0.0, 0.0, 200.0, 600.0);
        let ctx = Context::new(&view, &canvas, bounds);

        // 600pt window, 300pt sibling min + 90pt output min = 390pt total
        // min, 210pt "extra". With stretch_y(0.0) *none* of that extra
        // should go to `output` -- it should render at exactly its own
        // 90pt min, not 90 + 105 (half of 210).
        let initial = vtile.bounds_of(&ctx, 1);
        assert_eq!(
            initial.height(),
            90.0,
            "output should render at exactly its own min, claiming none of the extra"
        );

        // Drag the output panel's height up by 100pt (what `Splitter`'s
        // callback does) -- the window itself hasn't resized.
        output.set_height(190.0);
        let after = vtile.bounds_of(&ctx, 1);
        assert_eq!(
            after.height(),
            190.0,
            "output's rendered height should track set_height exactly (1:1), not half of the delta"
        );
    }

    #[test]
    fn plain_never_produces_highlights_even_for_rust_like_text() {
        let editor = CodeEditor::plain().text("fn main() { let x: u32 = 1; }");
        assert!(
            editor.highlights.read().unwrap().is_empty(),
            "a plain() editor should never populate highlights, regardless of content"
        );
    }

    #[test]
    fn new_still_highlights_rust_text_unlike_plain() {
        let editor = CodeEditor::new().text("fn main() { let x: u32 = 1; }");
        assert!(
            !editor.highlights.read().unwrap().is_empty(),
            "new() should still apply real Rust highlighting -- plain() shouldn't have changed that"
        );
    }

    fn focused_ctx_for(_editor: &CodeEditor) -> (crate::view::View, RefCell<Canvas>, Rect) {
        let view = crate::view::View::new(crate::support::point::Extent::new(700.0, 400.0));
        let canvas = RefCell::new(Canvas::new(700, 400).unwrap());
        let bounds = Rect::new(0.0, 0.0, 700.0, 400.0);
        (view, canvas, bounds)
    }

    fn press(key: KeyCode) -> KeyInfo {
        KeyInfo {
            key,
            action: crate::view::KeyAction::Press,
            modifiers: 0,
        }
    }

    #[test]
    fn typing_a_prefix_matching_an_existing_identifier_suggests_its_remainder() {
        let editor = CodeEditor::new().text("let processor = 1;\n");
        *editor.cursor.write().unwrap() = CursorPos { line: 1, column: 0 };
        for c in "proc".chars() {
            editor.insert_text(&c.to_string());
        }
        assert_eq!(editor.get_text(), "let processor = 1;\nproc");
        assert_eq!(
            *editor.suggestion.read().unwrap(),
            Some("essor".to_string()),
            "\"proc\" should suggest the rest of the existing \"processor\" identifier"
        );
    }

    #[test]
    fn tab_accepts_a_pending_suggestion_instead_of_indenting() {
        let editor = CodeEditor::new().text("let processor = 1;\n");
        *editor.cursor.write().unwrap() = CursorPos { line: 1, column: 0 };
        for c in "proc".chars() {
            editor.insert_text(&c.to_string());
        }
        assert!(editor.suggestion.read().unwrap().is_some());
        *editor.state.write().unwrap() = EditorState::Focused;

        let (view, canvas, bounds) = focused_ctx_for(&editor);
        let ctx = Context::new(&view, &canvas, bounds);
        assert!(editor.handle_key(&ctx, press(KeyCode::Tab)));

        assert_eq!(
            editor.get_text(),
            "let processor = 1;\nprocessor",
            "Tab should insert the suggestion's remainder, not a 4-space indent"
        );
        assert!(
            editor.suggestion.read().unwrap().is_none(),
            "accepting a suggestion should clear it"
        );
    }

    #[test]
    fn tab_without_a_pending_suggestion_still_indents() {
        let editor = CodeEditor::new().text("");
        *editor.state.write().unwrap() = EditorState::Focused;

        let (view, canvas, bounds) = focused_ctx_for(&editor);
        let ctx = Context::new(&view, &canvas, bounds);
        assert!(editor.handle_key(&ctx, press(KeyCode::Tab)));

        assert_eq!(
            editor.get_text(),
            "    ",
            "Tab with no pending suggestion should fall through to the existing indent behavior"
        );
    }

    #[test]
    fn moving_the_cursor_clears_a_pending_suggestion() {
        let editor = CodeEditor::new().text("let processor = 1;\n");
        *editor.cursor.write().unwrap() = CursorPos { line: 1, column: 0 };
        for c in "proc".chars() {
            editor.insert_text(&c.to_string());
        }
        assert!(editor.suggestion.read().unwrap().is_some());
        *editor.state.write().unwrap() = EditorState::Focused;

        let (view, canvas, bounds) = focused_ctx_for(&editor);
        let ctx = Context::new(&view, &canvas, bounds);
        editor.handle_key(&ctx, press(KeyCode::Left));

        assert!(
            editor.suggestion.read().unwrap().is_none(),
            "moving the cursor away should invalidate the pending suggestion"
        );
    }

    #[test]
    fn dragging_over_text_selects_it() {
        let editor = CodeEditor::new().text("hello world");
        let (view, canvas, bounds) = focused_ctx_for(&editor);
        let ctx = Context::new(&view, &canvas, bounds);

        let start_pos = Point::new(60.0, 10.0);
        let end_pos = Point::new(150.0, 10.0);
        let expected_start = editor.cursor_pos_from_click(&ctx, start_pos);
        let expected_end = editor.cursor_pos_from_click(&ctx, end_pos);
        assert_ne!(
            expected_start, expected_end,
            "test click positions should land on different columns"
        );

        let down = MouseButton {
            down: true,
            click_count: 1,
            button: MouseButtonKind::Left,
            modifiers: 0,
            pos: start_pos,
        };
        editor.handle_click(&ctx, down);
        assert!(
            editor.selection_anchor.read().unwrap().is_none(),
            "a plain mouse-down shouldn't create a selection until actually dragged"
        );

        let drag = MouseButton {
            pos: end_pos,
            ..down
        };
        editor.handle_drag(&ctx, drag);

        assert_eq!(
            *editor.selection_anchor.read().unwrap(),
            Some(expected_start),
            "dragging should anchor the selection at the mouse-down position"
        );
        assert_eq!(
            *editor.cursor.read().unwrap(),
            expected_end,
            "the cursor should track the drag position, extending the selection"
        );
    }

    #[test]
    fn releasing_the_drag_keeps_the_selection() {
        let editor = CodeEditor::new().text("hello world");
        let (view, canvas, bounds) = focused_ctx_for(&editor);
        let ctx = Context::new(&view, &canvas, bounds);

        let down = MouseButton {
            down: true,
            click_count: 1,
            button: MouseButtonKind::Left,
            modifiers: 0,
            pos: Point::new(60.0, 10.0),
        };
        editor.handle_click(&ctx, down);
        editor.handle_drag(
            &ctx,
            MouseButton {
                pos: Point::new(150.0, 10.0),
                ..down
            },
        );
        assert!(editor.selection_anchor.read().unwrap().is_some());

        let up = MouseButton {
            down: false,
            ..down
        };
        editor.handle_click(&ctx, up);

        assert!(
            editor.selection_anchor.read().unwrap().is_some(),
            "releasing the mouse after a drag-select shouldn't clear the selection"
        );
    }

    #[test]
    fn shift_click_extends_the_selection_from_the_current_cursor() {
        let editor = CodeEditor::new().text("hello world");
        let (view, canvas, bounds) = focused_ctx_for(&editor);
        let ctx = Context::new(&view, &canvas, bounds);

        let plain_click = MouseButton {
            down: true,
            click_count: 1,
            button: MouseButtonKind::Left,
            modifiers: 0,
            pos: Point::new(60.0, 10.0),
        };
        editor.handle_click(&ctx, plain_click);
        let start_cursor = *editor.cursor.read().unwrap();
        assert!(
            editor.selection_anchor.read().unwrap().is_none(),
            "a plain click shouldn't leave a selection"
        );

        let shift_click_pos = Point::new(150.0, 10.0);
        let expected_end = editor.cursor_pos_from_click(&ctx, shift_click_pos);
        assert_ne!(
            start_cursor, expected_end,
            "test positions should land on different columns"
        );

        let shift_click = MouseButton {
            down: true,
            click_count: 1,
            button: MouseButtonKind::Left,
            modifiers: crate::view::modifiers::SHIFT,
            pos: shift_click_pos,
        };
        editor.handle_click(&ctx, shift_click);

        assert_eq!(
            *editor.selection_anchor.read().unwrap(),
            Some(start_cursor),
            "shift-click should anchor the selection at the cursor's position before the click"
        );
        assert_eq!(
            *editor.cursor.read().unwrap(),
            expected_end,
            "shift-click should move the cursor to the click point, extending the selection"
        );
    }

    #[test]
    fn a_second_shift_click_extends_further_without_resetting_the_anchor() {
        let editor = CodeEditor::new().text("hello world");
        let (view, canvas, bounds) = focused_ctx_for(&editor);
        let ctx = Context::new(&view, &canvas, bounds);

        let plain_click = MouseButton {
            down: true,
            click_count: 1,
            button: MouseButtonKind::Left,
            modifiers: 0,
            pos: Point::new(60.0, 10.0),
        };
        editor.handle_click(&ctx, plain_click);
        let start_cursor = *editor.cursor.read().unwrap();

        let first_shift_click = MouseButton {
            modifiers: crate::view::modifiers::SHIFT,
            pos: Point::new(100.0, 10.0),
            ..plain_click
        };
        editor.handle_click(&ctx, first_shift_click);
        assert_eq!(*editor.selection_anchor.read().unwrap(), Some(start_cursor));

        let second_shift_click = MouseButton {
            modifiers: crate::view::modifiers::SHIFT,
            pos: Point::new(150.0, 10.0),
            ..plain_click
        };
        editor.handle_click(&ctx, second_shift_click);

        assert_eq!(
            *editor.selection_anchor.read().unwrap(),
            Some(start_cursor),
            "a later shift-click should keep extending from the same original anchor, not move it"
        );
    }

    #[test]
    fn extract_range_reads_a_single_line_span() {
        let lines = vec!["hello world".to_string()];
        let a = CursorPos { line: 0, column: 6 };
        let b = CursorPos {
            line: 0,
            column: 11,
        };
        assert_eq!(extract_range(&lines, a, b), "world");
        // Order-independent, same as `delete_range_inner`.
        assert_eq!(extract_range(&lines, b, a), "world");
    }

    #[test]
    fn extract_range_reads_a_multi_line_span() {
        let lines = vec!["one".to_string(), "two".to_string(), "three".to_string()];
        let a = CursorPos { line: 0, column: 1 };
        let b = CursorPos { line: 2, column: 3 };
        assert_eq!(extract_range(&lines, a, b), "ne\ntwo\nthr");
    }
}
