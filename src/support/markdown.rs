//! A small, focused Markdown-to-styled-text renderer, built for
//! `ChatHistory`'s message bubbles rather than as a full CommonMark
//! implementation -- bold, italic, inline/block code, lists, and
//! headings are supported (the realistic vocabulary of a chat
//! assistant's replies); tables, footnotes, and the rest of CommonMark's
//! long tail are out of scope.
//!
//! Split into three pure-ish stages so each is independently testable:
//! [`markdown_to_runs`] (no `Canvas` dependency -- text in, styled runs
//! out), [`wrap_runs`] (needs `Canvas` only for `text_width`
//! measurement), and [`draw_runs`] (the only stage that actually paints).
//!
//! `Canvas::font(...)` has never been called more than once within a
//! single paragraph's draw anywhere else in this codebase (`CodeEditor`'s
//! own per-run styling only ever varies *color*, via
//! `line_color_segments`, never font) -- [`draw_runs`]/[`wrap_runs`] are
//! the first callers to switch it per run, which is legal per `Canvas`'s
//! own API (`current_font` is just mutable draw state) but exercises it
//! for the first time.

use pulldown_cmark::{Event, Parser, Tag, TagEnd};

use crate::support::canvas::Canvas;
use crate::support::color::Color;
use crate::support::font::Font;
use crate::support::point::Point;

/// One run of text sharing a single style. `bold`/`italic`/`monospace`
/// are independent flags (not a single enum) since Markdown allows them
/// to combine, e.g. `` **bold `code`** `` is both bold and monospace.
#[derive(Debug, Clone, PartialEq)]
pub struct StyledRun {
    pub text: String,
    pub bold: bool,
    pub italic: bool,
    pub monospace: bool,
}

/// `pub(crate)`, not private: `chat_history.rs` needs this same
/// run-to-font mapping to measure a wrapped line's total width (summing
/// each run's width in its own font) when sizing a bubble, without
/// duplicating the bold/italic/monospace-to-`Font` logic.
pub(crate) fn run_font(run: &StyledRun) -> Font {
    let mut font = if run.monospace { Font::monospace() } else { Font::sans_serif() };
    if run.bold {
        font = font.bold();
    }
    if run.italic {
        font = font.italic();
    }
    font
}

/// Parses `source` into styled runs, grouped by logical line (one entry
/// per paragraph, list item, or heading; blank lines from block
/// boundaries are represented as empty inner `Vec`s so vertical spacing
/// between blocks survives). No `Canvas` dependency -- pure text in,
/// runs out, so this is unit-testable without a live font database.
pub fn markdown_to_runs(source: &str) -> Vec<Vec<StyledRun>> {
    let mut lines: Vec<Vec<StyledRun>> = vec![Vec::new()];
    let mut bold_depth = 0usize;
    let mut italic_depth = 0usize;
    let mut code_depth = 0usize;

    let push_text = |lines: &mut Vec<Vec<StyledRun>>, text: &str, bold: bool, italic: bool, monospace: bool| {
        let mut parts = text.split('\n');
        if let Some(first) = parts.next() {
            if !first.is_empty() {
                lines.last_mut().unwrap().push(StyledRun { text: first.to_string(), bold, italic, monospace });
            }
        }
        for part in parts {
            lines.push(Vec::new());
            if !part.is_empty() {
                lines.last_mut().unwrap().push(StyledRun { text: part.to_string(), bold, italic, monospace });
            }
        }
    };

    for event in Parser::new(source) {
        match event {
            Event::Text(text) => push_text(&mut lines, &text, bold_depth > 0, italic_depth > 0, code_depth > 0),
            Event::Code(text) => push_text(&mut lines, &text, bold_depth > 0, italic_depth > 0, true),
            Event::Start(Tag::Strong) => bold_depth += 1,
            Event::End(TagEnd::Strong) => bold_depth = bold_depth.saturating_sub(1),
            Event::Start(Tag::Emphasis) => italic_depth += 1,
            Event::End(TagEnd::Emphasis) => italic_depth = italic_depth.saturating_sub(1),
            Event::Start(Tag::CodeBlock(_)) => code_depth += 1,
            Event::End(TagEnd::CodeBlock) => {
                code_depth = code_depth.saturating_sub(1);
                lines.push(Vec::new());
            }
            Event::Start(Tag::Heading { .. }) => bold_depth += 1,
            Event::End(TagEnd::Heading(_)) => {
                bold_depth = bold_depth.saturating_sub(1);
                lines.push(Vec::new());
            }
            Event::Start(Tag::Item) => {
                lines.last_mut().unwrap().push(StyledRun {
                    text: "\u{2022} ".to_string(),
                    bold: false,
                    italic: false,
                    monospace: false,
                });
            }
            Event::End(TagEnd::Item) => lines.push(Vec::new()),
            Event::End(TagEnd::Paragraph) => lines.push(Vec::new()),
            Event::SoftBreak => push_text(&mut lines, " ", bold_depth > 0, italic_depth > 0, code_depth > 0),
            Event::HardBreak => lines.push(Vec::new()),
            _ => {}
        }
    }

    while lines.len() > 1 && lines.last().is_some_and(Vec::is_empty) {
        lines.pop();
    }
    lines
}

/// Word-wraps each logical line's runs to `max_width`, splitting across
/// run boundaries where a style change falls mid-line -- generalizes
/// `ChatHistory::wrap_text`'s greedy single-font approach to runs, each
/// measured in its own font (summing per-word widths rather than
/// measuring one concatenated string, since different runs on the same
/// display line can be different fonts).
pub fn wrap_runs(canvas: &mut Canvas, lines: &[Vec<StyledRun>], max_width: f32) -> Vec<Vec<StyledRun>> {
    let mut out = Vec::new();

    for line in lines {
        if line.is_empty() {
            out.push(Vec::new());
            continue;
        }

        let mut current: Vec<StyledRun> = Vec::new();
        let mut current_width = 0.0f32;

        for run in line {
            canvas.font(run_font(run));
            for (word_index, word) in run.text.split(' ').enumerate() {
                let piece = if word_index > 0 { format!(" {word}") } else { word.to_string() };
                if piece.is_empty() {
                    continue;
                }
                let piece_width = canvas.text_width(&piece);

                if current_width > 0.0 && current_width + piece_width > max_width {
                    out.push(std::mem::take(&mut current));
                    let trimmed = piece.trim_start().to_string();
                    let trimmed_width = canvas.text_width(&trimmed);
                    append_run(&mut current, trimmed, run);
                    current_width = trimmed_width;
                } else {
                    append_run(&mut current, piece, run);
                    current_width += piece_width;
                }
            }
        }
        out.push(current);
    }

    out
}

fn append_run(current: &mut Vec<StyledRun>, text: String, style: &StyledRun) {
    if text.is_empty() {
        return;
    }
    if let Some(last) = current.last_mut() {
        if last.bold == style.bold && last.italic == style.italic && last.monospace == style.monospace {
            last.text.push_str(&text);
            return;
        }
    }
    current.push(StyledRun { text, bold: style.bold, italic: style.italic, monospace: style.monospace });
}

/// Draws already-wrapped run-lines starting at `origin`, switching font
/// and advancing `x` per run. `color` is the default fill for every run
/// -- this renderer doesn't vary color per Markdown construct (no syntax
/// highlighting inside chat text), only weight/style/family.
pub fn draw_runs(canvas: &mut Canvas, lines: &[Vec<StyledRun>], origin: Point, line_height: f32, color: Color) {
    canvas.fill_style(color);
    let mut y = origin.y;
    for line in lines {
        let mut x = origin.x;
        for run in line {
            canvas.font(run_font(run));
            canvas.fill_text(&run.text, Point::new(x, y));
            x += canvas.text_width(&run.text);
        }
        y += line_height;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn plain_text_is_a_single_unstyled_run() {
        let lines = markdown_to_runs("hello world");
        assert_eq!(lines.len(), 1);
        assert_eq!(lines[0], vec![StyledRun { text: "hello world".to_string(), bold: false, italic: false, monospace: false }]);
    }

    #[test]
    fn bold_italic_and_inline_code_produce_distinct_runs() {
        let lines = markdown_to_runs("**bold** and *italic* and `code`");
        assert_eq!(lines.len(), 1);
        assert_eq!(
            lines[0],
            vec![
                StyledRun { text: "bold".to_string(), bold: true, italic: false, monospace: false },
                StyledRun { text: " and ".to_string(), bold: false, italic: false, monospace: false },
                StyledRun { text: "italic".to_string(), bold: false, italic: true, monospace: false },
                StyledRun { text: " and ".to_string(), bold: false, italic: false, monospace: false },
                StyledRun { text: "code".to_string(), bold: false, italic: false, monospace: true },
            ]
        );
    }

    #[test]
    fn a_code_block_preserves_its_text_without_treating_markdown_syntax_inside_it_specially() {
        let lines = markdown_to_runs("```\nlet x = **not bold**;\n```");
        let code_text: String =
            lines.iter().flat_map(|line| line.iter().map(|r| r.text.as_str())).collect::<Vec<_>>().join("\n");
        assert!(code_text.contains("let x = **not bold**;"), "code block text was mangled: {code_text:?}");
        assert!(
            lines.iter().flatten().all(|r| r.monospace || r.text.trim().is_empty()),
            "expected every non-empty run inside the code block to be monospace"
        );
    }

    #[test]
    fn a_bullet_list_prefixes_each_item_with_a_bullet_on_its_own_line() {
        let lines = markdown_to_runs("- first\n- second");
        let rendered: Vec<String> =
            lines.iter().map(|line| line.iter().map(|r| r.text.as_str()).collect::<String>()).collect();
        assert!(rendered.contains(&"\u{2022} first".to_string()), "rendered lines: {rendered:?}");
        assert!(rendered.contains(&"\u{2022} second".to_string()), "rendered lines: {rendered:?}");
    }

    #[test]
    fn a_heading_is_rendered_bold() {
        let lines = markdown_to_runs("# Title");
        let bold_run = lines.iter().flatten().find(|r| r.text.contains("Title"));
        assert!(bold_run.is_some_and(|r| r.bold), "expected the heading text to be a bold run");
    }

    #[test]
    fn wrap_runs_keeps_every_line_within_max_width() {
        let mut canvas = Canvas::new(400, 400).unwrap();
        let lines = markdown_to_runs("the quick brown **fox** jumps over the lazy dog again and again");

        let wrapped = wrap_runs(&mut canvas, &lines, 100.0);

        assert!(wrapped.len() > 1, "expected wrapping to produce multiple lines");
        for line in &wrapped {
            let mut width = 0.0f32;
            for run in line {
                canvas.font(run_font(run));
                width += canvas.text_width(&run.text);
            }
            assert!(width <= 101.0, "line {line:?} exceeds the 100px max width ({width})");
        }
    }
}
