//! A scrollable chat message list rendered as colored bubbles per sender,
//! word-wrapped to the available width. Built for `mkide-app`'s Assistant
//! chat panel, which used to render its history as plain `"You: ..."` /
//! `"Assistant: ..."` lines inside a read-only `CodeEditor` -- a plain
//! monospace text widget can't render distinct colored, aligned bubbles,
//! so this is a purpose-built element instead, modeled on `List`'s
//! scroll/clip/draw conventions (`src/element/list.rs`).
//!
//! Display-only, like `List`/`Dropdown` -- no text selection/copy of
//! history content (a real trade-off against the old `CodeEditor`-backed
//! history, inherent to bubble rendering).
//!
//! An assistant message can be built up incrementally -- see
//! [`ChatHistory::start_streaming_message`]/[`append_thinking`]/
//! [`append_response`] -- for a model whose reply streams in token by
//! token rather than arriving all at once. `thinking`/`response` are
//! separate fields (not one `text` string) because a model's reasoning,
//! when present, is meant to read as a distinct, de-emphasized section
//! above the real answer, not just more paragraph text. Both are
//! rendered as real Markdown (bold/italic/code/lists) via
//! `crate::support::markdown`, not the plain wrapped text this element
//! used to draw.
//!
//! [`append_thinking`]: ChatHistory::append_thinking
//! [`append_response`]: ChatHistory::append_response

use super::context::{BasicContext, Context};
use super::{Element, ViewLimits, ViewStretch};
use crate::support::canvas::Canvas;
use crate::support::color::Color;
use crate::support::markdown::{self, StyledRun};
use crate::support::point::Point;
use crate::support::rect::Rect;
use crate::support::theme::get_theme;
use std::any::Any;
use std::sync::RwLock;

/// Who sent a [`ChatMessage`] -- controls bubble alignment and color.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ChatSender {
    User,
    Assistant,
    /// Status/banner/error lines -- rendered full-width, centered, and
    /// muted, with no distinct bubble background, so they read as
    /// out-of-band notices rather than a conversation turn.
    System,
}

#[derive(Debug, Clone)]
pub struct ChatMessage {
    pub sender: ChatSender,
    /// The model's reasoning, if any -- empty for User/System messages
    /// and for an Assistant reply whose raw text never contained a
    /// `<|channel>thought`/`<channel|>` pair (see `mkide_llm::channel`).
    pub thinking: String,
    pub response: String,
}

impl ChatMessage {
    pub fn new(sender: ChatSender, text: impl Into<String>) -> Self {
        Self { sender, thinking: String::new(), response: text.into() }
    }
}

/// One laid-out message: its wrapped, styled lines and the bubble rect to
/// draw them in, already positioned in `ctx`-absolute coordinates with
/// scroll applied. `thinking_lines` is empty for any message with no
/// thinking text -- callers skip that whole section rather than drawing
/// an empty label.
struct LaidOutMessage {
    sender: ChatSender,
    thinking_lines: Vec<Vec<StyledRun>>,
    response_lines: Vec<Vec<StyledRun>>,
    bubble: Rect,
}

/// A scrollable list of chat bubbles.
pub struct ChatHistory {
    messages: RwLock<Vec<ChatMessage>>,
    scroll_offset: RwLock<f32>,
    width: f32,
    height: f32,
    font_size: f32,
    padding: f32,
    bubble_padding: f32,
    bubble_max_width_ratio: f32,
    corner_radius: f32,
    gap: f32,
    background_color: Color,
    user_bubble_color: Color,
    assistant_bubble_color: Color,
    user_text_color: Color,
    assistant_text_color: Color,
    system_text_color: Color,
    thinking_text_color: Color,
    enabled: bool,
}

impl ChatHistory {
    /// Creates a new, empty chat history.
    pub fn new() -> Self {
        let theme = get_theme();
        Self {
            messages: RwLock::new(Vec::new()),
            scroll_offset: RwLock::new(0.0),
            width: 400.0,
            height: 300.0,
            font_size: theme.label_font_size,
            padding: 10.0,
            bubble_padding: 10.0,
            bubble_max_width_ratio: 0.75,
            corner_radius: 10.0,
            gap: 8.0,
            background_color: theme.input_box_color,
            user_bubble_color: theme.chat_user_bubble_color,
            assistant_bubble_color: theme.chat_assistant_bubble_color,
            user_text_color: Color::from_rgb_u8(255, 255, 255),
            assistant_text_color: theme.label_font_color,
            system_text_color: theme.chat_system_text_color,
            thinking_text_color: theme.chat_thinking_text_color,
            enabled: true,
        }
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

    /// Appends a message and requests a scroll to the bottom to reveal
    /// it. The real clamped scroll position (which needs a wrapped-text
    /// measurement pass against the current width) is resolved the next
    /// time `layout_messages` runs -- see its own doc comment.
    pub fn push_message(&self, sender: ChatSender, text: impl Into<String>) {
        self.messages.write().unwrap().push(ChatMessage::new(sender, text));
        *self.scroll_offset.write().unwrap() = f32::MAX;
    }

    /// Starts a new, empty message that [`append_thinking`]/
    /// [`append_response`] grow in place -- for a reply that streams in
    /// token by token rather than arriving all at once. Requests a
    /// scroll to the bottom the same way `push_message` does.
    ///
    /// [`append_thinking`]: Self::append_thinking
    /// [`append_response`]: Self::append_response
    pub fn start_streaming_message(&self, sender: ChatSender) {
        self.messages.write().unwrap().push(ChatMessage::new(sender, ""));
        *self.scroll_offset.write().unwrap() = f32::MAX;
    }

    /// Appends `delta` to the thinking text of whatever message is
    /// currently last (the one [`start_streaming_message`] started) --
    /// a no-op if there is no message yet.
    ///
    /// [`start_streaming_message`]: Self::start_streaming_message
    pub fn append_thinking(&self, delta: &str) {
        if let Some(last) = self.messages.write().unwrap().last_mut() {
            last.thinking.push_str(delta);
        }
        *self.scroll_offset.write().unwrap() = f32::MAX;
    }

    /// Appends `delta` to the response text of whatever message is
    /// currently last -- see [`append_thinking`](Self::append_thinking).
    pub fn append_response(&self, delta: &str) {
        if let Some(last) = self.messages.write().unwrap().last_mut() {
            last.response.push_str(delta);
        }
        *self.scroll_offset.write().unwrap() = f32::MAX;
    }

    /// Removes every message and resets scroll.
    pub fn clear(&self) {
        self.messages.write().unwrap().clear();
        *self.scroll_offset.write().unwrap() = 0.0;
    }

    fn line_height(&self) -> f32 {
        self.font_size * 1.3
    }

    fn max_text_width(&self) -> f32 {
        (self.width * self.bubble_max_width_ratio - 2.0 * self.bubble_padding).max(20.0)
    }

    /// Markdown-parses and word-wraps `text` to `max_width`, or `None` if
    /// `text` is empty (a streaming message's thinking/response section
    /// before its first chunk has arrived, or a message with no thinking
    /// at all) -- callers skip an empty section entirely rather than
    /// drawing a lone blank line for it.
    fn wrap_markdown(canvas: &mut Canvas, text: &str, max_width: f32, force_italic: bool) -> Option<Vec<Vec<StyledRun>>> {
        if text.is_empty() {
            return None;
        }
        let mut runs = markdown::markdown_to_runs(text);
        if force_italic {
            for line in &mut runs {
                for run in line.iter_mut() {
                    run.italic = true;
                }
            }
        }
        Some(markdown::wrap_runs(canvas, &runs, max_width))
    }

    fn measure_lines_width(canvas: &mut Canvas, lines: &[Vec<StyledRun>]) -> f32 {
        lines
            .iter()
            .map(|line| {
                line.iter()
                    .map(|run| {
                        canvas.font(markdown::run_font(run));
                        canvas.text_width(&run.text)
                    })
                    .sum::<f32>()
            })
            .fold(0.0f32, f32::max)
    }

    /// Lays out every message top-down, in scroll-adjusted absolute
    /// coordinates matching `ctx.bounds`, returning `(messages,
    /// total_content_height)`. Recomputed on every call (draw/scroll/
    /// hit-test all receive `&Context`, which carries the `Canvas`
    /// measurement needs) rather than cached against message count or
    /// width -- chat histories are small, and a cache keyed on the wrong
    /// invalidation condition is exactly the `VTile::bounds_of` bug class
    /// this session already hit once.
    ///
    /// As a side effect, self-corrects `scroll_offset` against the fresh
    /// `total_content_height` (clamping it into a valid range) -- this is
    /// what actually resolves `push_message`'s `f32::MAX` "scroll to
    /// bottom" sentinel into a real position.
    fn layout_messages(&self, ctx: &Context) -> (Vec<LaidOutMessage>, f32) {
        let messages = self.messages.read().unwrap();
        let line_height = self.line_height();
        let max_text_width = self.max_text_width();

        let mut out = Vec::with_capacity(messages.len());
        let mut y = self.padding;

        {
            let mut canvas = ctx.canvas.borrow_mut();
            canvas.font_size(self.font_size);

            for msg in messages.iter() {
                // The "Thinking" label is folded into `thinking_lines` as
                // its own first line rather than tracked separately --
                // draw_messages then just draws whatever's in
                // `thinking_lines` uniformly, no special-cased label draw.
                let thinking_lines =
                    Self::wrap_markdown(&mut canvas, &msg.thinking, max_text_width, true).map(|mut lines| {
                        lines.insert(
                            0,
                            vec![StyledRun {
                                text: "Thinking".to_string(),
                                bold: false,
                                italic: true,
                                monospace: false,
                            }],
                        );
                        lines
                    });
                let response_lines = Self::wrap_markdown(&mut canvas, &msg.response, max_text_width, false);

                let thinking_height = thinking_lines.as_ref().map_or(0.0, |l| l.len() as f32 * line_height);
                let response_height = response_lines.as_ref().map_or(0.0, |l| l.len() as f32 * line_height);
                let section_gap = if thinking_lines.is_some() && response_lines.is_some() {
                    self.gap * 0.5
                } else {
                    0.0
                };
                let text_height = thinking_height + section_gap + response_height;

                let thinking_lines = thinking_lines.unwrap_or_default();
                let response_lines = response_lines.unwrap_or_default();

                let bubble = match msg.sender {
                    ChatSender::System => {
                        Rect::new(self.padding, y, self.width - self.padding, y + text_height)
                    }
                    ChatSender::User | ChatSender::Assistant => {
                        let natural_width = Self::measure_lines_width(&mut canvas, &thinking_lines)
                            .max(Self::measure_lines_width(&mut canvas, &response_lines))
                            + 2.0 * self.bubble_padding;
                        let bubble_width = natural_width.min(self.width * self.bubble_max_width_ratio);
                        let (left, right) = if msg.sender == ChatSender::User {
                            (self.width - self.padding - bubble_width, self.width - self.padding)
                        } else {
                            (self.padding, self.padding + bubble_width)
                        };
                        Rect::new(left, y, right, y + text_height + 2.0 * self.bubble_padding)
                    }
                };

                y = bubble.bottom + self.gap;
                out.push(LaidOutMessage { sender: msg.sender, thinking_lines, response_lines, bubble });
            }
        }

        let total_height = (y - self.gap + self.padding).max(0.0);
        drop(messages);

        let visible_height = ctx.bounds.height();
        let scroll = {
            let mut scroll_guard = self.scroll_offset.write().unwrap();
            *scroll_guard = if total_height <= visible_height {
                0.0
            } else {
                (*scroll_guard).min(total_height - visible_height).max(0.0)
            };
            *scroll_guard
        };

        let dx = ctx.bounds.left;
        let dy = ctx.bounds.top - scroll;
        let out = out
            .into_iter()
            .map(|m| LaidOutMessage { bubble: m.bubble.translate(dx, dy), ..m })
            .collect();

        (out, total_height)
    }

    fn draw_background(&self, ctx: &Context) {
        let mut canvas = ctx.canvas.borrow_mut();
        canvas.fill_style(self.background_color);
        canvas.fill_round_rect(ctx.bounds, self.corner_radius);
    }

    fn draw_messages(&self, ctx: &Context, messages: &[LaidOutMessage]) {
        let mut canvas = ctx.canvas.borrow_mut();
        canvas.font_size(self.font_size);
        let line_height = self.line_height();

        for msg in messages {
            if msg.bubble.bottom < ctx.bounds.top || msg.bubble.top > ctx.bounds.bottom {
                continue;
            }

            match msg.sender {
                ChatSender::System => {
                    // System messages are display-only status text (see
                    // this module's own doc comment) and never carry
                    // `thinking` -- only `response_lines` is ever
                    // populated for them, but each line is still centered
                    // per-line rather than left-aligned like a bubble.
                    let mut y = msg.bubble.top + self.font_size * 0.85;
                    for line in &msg.response_lines {
                        let width: f32 = line
                            .iter()
                            .map(|run| {
                                canvas.font(markdown::run_font(run));
                                canvas.text_width(&run.text)
                            })
                            .sum();
                        canvas.fill_style(self.system_text_color);
                        let mut x = msg.bubble.left + (msg.bubble.width() - width) * 0.5;
                        for run in line {
                            canvas.font(markdown::run_font(run));
                            canvas.fill_text(&run.text, Point::new(x, y));
                            x += canvas.text_width(&run.text);
                        }
                        y += line_height;
                    }
                }
                ChatSender::User | ChatSender::Assistant => {
                    let (bubble_color, text_color) = if msg.sender == ChatSender::User {
                        (self.user_bubble_color, self.user_text_color)
                    } else {
                        (self.assistant_bubble_color, self.assistant_text_color)
                    };

                    canvas.fill_style(bubble_color);
                    canvas.fill_round_rect(msg.bubble, self.corner_radius);

                    let mut y = msg.bubble.top + self.bubble_padding + self.font_size * 0.85;
                    if !msg.thinking_lines.is_empty() {
                        markdown::draw_runs(
                            &mut canvas,
                            &msg.thinking_lines,
                            Point::new(msg.bubble.left + self.bubble_padding, y),
                            line_height,
                            self.thinking_text_color,
                        );
                        y += msg.thinking_lines.len() as f32 * line_height;
                        if !msg.response_lines.is_empty() {
                            y += self.gap * 0.5;
                        }
                    }
                    if !msg.response_lines.is_empty() {
                        markdown::draw_runs(
                            &mut canvas,
                            &msg.response_lines,
                            Point::new(msg.bubble.left + self.bubble_padding, y),
                            line_height,
                            text_color,
                        );
                    }
                }
            }
        }
    }

    fn draw_scrollbar(&self, ctx: &Context, total_height: f32, visible_height: f32) {
        if total_height <= visible_height {
            return;
        }

        let theme = get_theme();
        let scroll = *self.scroll_offset.read().unwrap();

        let scrollbar_height = (visible_height / total_height * visible_height).max(20.0);
        let scrollbar_y =
            scroll / (total_height - visible_height) * (visible_height - scrollbar_height);

        let scrollbar_rect = Rect::new(
            ctx.bounds.right - 8.0,
            ctx.bounds.top + scrollbar_y,
            ctx.bounds.right - 2.0,
            ctx.bounds.top + scrollbar_y + scrollbar_height,
        );

        let mut canvas = ctx.canvas.borrow_mut();
        canvas.fill_style(theme.scrollbar_color);
        canvas.fill_round_rect(scrollbar_rect, 3.0);
    }
}

impl Default for ChatHistory {
    fn default() -> Self {
        Self::new()
    }
}

impl Element for ChatHistory {
    fn limits(&self, _ctx: &BasicContext) -> ViewLimits {
        ViewLimits::min_size(self.width, self.height)
    }

    fn stretch(&self) -> ViewStretch {
        ViewStretch::new(1.0, 1.0)
    }

    fn draw(&self, ctx: &Context) {
        self.draw_background(ctx);

        let (messages, total_height) = self.layout_messages(ctx);
        let visible_height = ctx.bounds.height();

        {
            let mut canvas = ctx.canvas.borrow_mut();
            canvas.save();
            let clip_bounds = Rect::new(
                ctx.bounds.left + self.corner_radius,
                ctx.bounds.top + self.corner_radius,
                ctx.bounds.right - self.corner_radius,
                ctx.bounds.bottom - self.corner_radius,
            );
            canvas.clip(clip_bounds);
        }

        self.draw_messages(ctx, &messages);

        {
            let mut canvas = ctx.canvas.borrow_mut();
            canvas.restore();
        }

        self.draw_scrollbar(ctx, total_height, visible_height);
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

    fn scroll(&mut self, ctx: &Context, dir: Point, p: Point) -> bool {
        self.handle_scroll(ctx, dir, p)
    }

    fn handle_scroll(&self, ctx: &Context, dir: Point, _p: Point) -> bool {
        if !self.enabled {
            return false;
        }

        // Also self-corrects `scroll_offset` against the fresh
        // `total_height` as a side effect -- see `layout_messages`'s doc
        // comment.
        let (_, total_height) = self.layout_messages(ctx);
        let visible_height = ctx.bounds.height();

        if total_height <= visible_height {
            return false;
        }

        let mut scroll = self.scroll_offset.write().unwrap();
        *scroll = (*scroll - dir.y * 20.0).min(total_height - visible_height).max(0.0);

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

/// Creates a chat history.
pub fn chat_history() -> ChatHistory {
    ChatHistory::new()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::support::point::Extent;
    use crate::view::View;

    #[test]
    fn wrap_markdown_keeps_every_line_within_max_width() {
        let mut canvas = Canvas::new(400, 400).unwrap();
        canvas.font_size(14.0);
        let text = "the quick brown fox jumps over the lazy dog again and again and again";

        let lines = ChatHistory::wrap_markdown(&mut canvas, text, 100.0, false).expect("non-empty text");

        assert!(lines.len() > 1, "expected wrapping to produce multiple lines");
        for line in &lines {
            let width: f32 = line
                .iter()
                .map(|run| {
                    canvas.font(markdown::run_font(run));
                    canvas.text_width(&run.text)
                })
                .sum();
            assert!(width <= 101.0, "line {line:?} exceeds the 100px max width ({width})");
        }
    }

    #[test]
    fn wrap_markdown_forces_a_break_on_a_blank_line_even_under_the_width_limit() {
        // Real CommonMark semantics (a deliberate change from the old
        // plain-text `wrap_text`, which forced a break on *any* literal
        // `\n`): a single newline inside a paragraph is a soft break --
        // it collapses to a space, not a new display line -- only a
        // blank line actually starts a new paragraph.
        let mut canvas = Canvas::new(400, 400).unwrap();
        canvas.font_size(14.0);

        let lines = ChatHistory::wrap_markdown(&mut canvas, "line1\n\nline2", 1000.0, false).expect("non-empty text");

        let rendered: Vec<String> =
            lines.iter().map(|line| line.iter().map(|r| r.text.as_str()).collect()).collect();
        assert_eq!(rendered, vec!["line1".to_string(), "line2".to_string()]);
    }

    #[test]
    fn wrap_markdown_returns_none_for_empty_text() {
        let mut canvas = Canvas::new(400, 400).unwrap();
        assert!(ChatHistory::wrap_markdown(&mut canvas, "", 100.0, false).is_none());
    }

    #[test]
    fn push_message_grows_total_content_height() {
        let history = ChatHistory::new().width(300.0).height(200.0);
        let view = View::new(Extent::new(300.0, 200.0));
        let canvas = std::cell::RefCell::new(Canvas::new(300, 200).unwrap());
        let bounds = Rect::new(0.0, 0.0, 300.0, 200.0);
        let ctx = Context::new(&view, &canvas, bounds);

        let (_, height_before) = history.layout_messages(&ctx);
        history.push_message(ChatSender::User, "hello");
        let (_, height_after) = history.layout_messages(&ctx);

        assert!(height_after > height_before);
    }

    #[test]
    fn user_bubbles_align_further_right_than_assistant_bubbles_for_the_same_text() {
        let history = ChatHistory::new().width(300.0).height(200.0);
        history.push_message(ChatSender::User, "hi");
        history.push_message(ChatSender::Assistant, "hi");

        let view = View::new(Extent::new(300.0, 200.0));
        let canvas = std::cell::RefCell::new(Canvas::new(300, 200).unwrap());
        let bounds = Rect::new(0.0, 0.0, 300.0, 200.0);
        let ctx = Context::new(&view, &canvas, bounds);

        let (messages, _) = history.layout_messages(&ctx);

        assert_eq!(messages.len(), 2);
        assert!(
            messages[0].bubble.right > messages[1].bubble.right,
            "user bubble (right-aligned) should sit further right than the assistant bubble \
             (left-aligned) for identical text"
        );
    }

    #[test]
    fn handle_scroll_is_a_noop_when_content_already_fits() {
        let history = ChatHistory::new().width(300.0).height(400.0);
        history.push_message(ChatSender::User, "short");

        let view = View::new(Extent::new(300.0, 400.0));
        let canvas = std::cell::RefCell::new(Canvas::new(300, 400).unwrap());
        let bounds = Rect::new(0.0, 0.0, 300.0, 400.0);
        let ctx = Context::new(&view, &canvas, bounds);

        let handled = history.handle_scroll(&ctx, Point::new(0.0, -5.0), Point::zero());

        assert!(!handled);
        assert_eq!(*history.scroll_offset.read().unwrap(), 0.0);
    }

    #[test]
    fn streaming_append_grows_the_last_messages_response_in_place() {
        let history = ChatHistory::new().width(300.0).height(200.0);
        history.start_streaming_message(ChatSender::Assistant);
        history.append_response("Hello");
        history.append_response(", world!");

        let messages = history.messages.read().unwrap();
        assert_eq!(messages.len(), 1);
        assert_eq!(messages[0].sender, ChatSender::Assistant);
        assert_eq!(messages[0].response, "Hello, world!");
        assert!(messages[0].thinking.is_empty());
    }

    #[test]
    fn streaming_append_thinking_and_response_target_separate_fields() {
        let history = ChatHistory::new().width(300.0).height(200.0);
        history.start_streaming_message(ChatSender::Assistant);
        history.append_thinking("reasoning here");
        history.append_response("the answer");

        let messages = history.messages.read().unwrap();
        assert_eq!(messages[0].thinking, "reasoning here");
        assert_eq!(messages[0].response, "the answer");
    }

    #[test]
    fn append_before_any_streaming_message_started_is_a_harmless_noop() {
        let history = ChatHistory::new().width(300.0).height(200.0);
        history.append_response("should go nowhere");
        assert!(history.messages.read().unwrap().is_empty());
    }

    #[test]
    fn a_message_with_thinking_text_lays_out_taller_than_one_with_only_a_response() {
        let history = ChatHistory::new().width(300.0).height(200.0);
        history.push_message(ChatSender::Assistant, "just an answer");

        let view = View::new(Extent::new(300.0, 200.0));
        let canvas = std::cell::RefCell::new(Canvas::new(300, 200).unwrap());
        let bounds = Rect::new(0.0, 0.0, 300.0, 200.0);
        let ctx = Context::new(&view, &canvas, bounds);
        let (_, height_without_thinking) = history.layout_messages(&ctx);

        history.clear();
        history.start_streaming_message(ChatSender::Assistant);
        history.append_thinking("some reasoning about the problem");
        history.append_response("just an answer");
        let (messages, height_with_thinking) = history.layout_messages(&ctx);

        assert!(
            height_with_thinking > height_without_thinking,
            "a message with a thinking section should lay out taller"
        );
        assert!(!messages[0].thinking_lines.is_empty());
        assert!(!messages[0].response_lines.is_empty());
    }

    #[test]
    fn handle_scroll_clamps_into_a_valid_range_when_content_overflows() {
        let history = ChatHistory::new().width(200.0).height(80.0);
        for i in 0..20 {
            history.push_message(ChatSender::Assistant, format!("message number {i}"));
        }

        let view = View::new(Extent::new(200.0, 80.0));
        let canvas = std::cell::RefCell::new(Canvas::new(200, 80).unwrap());
        let bounds = Rect::new(0.0, 0.0, 200.0, 80.0);
        let ctx = Context::new(&view, &canvas, bounds);

        // Scroll far past the top -- should clamp to 0, not go negative.
        let handled = history.handle_scroll(&ctx, Point::new(0.0, 1000.0), Point::zero());
        assert!(handled);
        assert_eq!(*history.scroll_offset.read().unwrap(), 0.0);

        // Scroll far past the bottom -- should clamp to the max valid
        // offset, not run away unbounded.
        let handled = history.handle_scroll(&ctx, Point::new(0.0, -100000.0), Point::zero());
        assert!(handled);
        let (_, total_height) = history.layout_messages(&ctx);
        assert_eq!(*history.scroll_offset.read().unwrap(), total_height - 80.0);
    }
}
