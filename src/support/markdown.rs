//! A small, focused Markdown-to-styled-text renderer, built for
//! `ChatHistory`'s message bubbles rather than as a full CommonMark
//! implementation -- bold, italic, inline/block code, lists, headings,
//! and inline/display math (`$...$`/`$$...$$`, via `support::math`) are
//! supported (the realistic vocabulary of a chat assistant's replies);
//! tables, footnotes, and the rest of CommonMark's long tail are out of
//! scope.
//!
//! Split into three pure-ish stages so each is independently testable:
//! [`markdown_to_runs`] (no `Canvas` dependency -- text in, [`StyledRun`]s
//! out, math spans parsed to an AST but not yet laid out), [`wrap_runs`]
//! (needs `Canvas` for `text_width` measurement *and* to actually lay out
//! any math runs into real [`math::layout::MathBox`]es, producing
//! [`WrappedLine`]s with real per-line height/depth instead of a single
//! shared scalar), and [`draw_runs`] (the only stage that actually
//! paints, dispatching each run to either `fill_text` or
//! `math::draw::draw_math_box`).
//!
//! `Canvas::font(...)` has never been called more than once within a
//! single paragraph's draw anywhere else in this codebase (`CodeEditor`'s
//! own per-run styling only ever varies *color*, via
//! `line_color_segments`, never font) -- [`draw_runs`]/[`wrap_runs`] are
//! the first callers to switch it per run, which is legal per `Canvas`'s
//! own API (`current_font` is just mutable draw state) but exercises it
//! for the first time.

use std::sync::Arc;

use pulldown_cmark::{Event, Options, Parser, Tag, TagEnd};

use crate::support::canvas::Canvas;
use crate::support::color::Color;
use crate::support::font::Font;
use crate::support::math::{self, layout::MathBox};
use crate::support::point::Point;

/// One parsed inline element, before layout. `Text`/`Math` are the only
/// two kinds -- see this module's own doc comment on the three-stage
/// pipeline for why a math run only carries a parsed AST here, not an
/// already-laid-out box (that needs a `Canvas`, which this stage
/// doesn't have).
#[derive(Debug, Clone)]
pub enum StyledRun {
    Text(TextRun),
    Math(MathRun),
}

/// `bold`/`italic`/`monospace` are independent flags (not a single enum)
/// since Markdown allows them to combine, e.g. `` **bold `code`** `` is
/// both bold and monospace.
#[derive(Debug, Clone, PartialEq)]
pub struct TextRun {
    pub text: String,
    pub bold: bool,
    pub italic: bool,
    pub monospace: bool,
}

#[derive(Debug, Clone)]
pub struct MathRun {
    /// The raw TeX source (without the surrounding `$`/`$$`) -- kept
    /// around for cache-keying and `Debug`/test output, not just the
    /// parsed tree.
    pub source: String,
    /// `$$...$$` (display) vs `$...$` (inline) -- controls
    /// [`math::style::MathStyle::Display`] vs `Text` at layout time.
    pub display: bool,
    pub ast: Arc<math::ast::MathNode>,
}

/// A run that's been through [`wrap_runs`] -- text runs are unchanged,
/// but a math run now carries its actual laid-out [`MathBox`] (computed
/// once during wrapping, since that's the first stage with `Canvas`
/// access) instead of just a parsed AST.
#[derive(Debug, Clone)]
pub enum LaidOutRun {
    Text(TextRun),
    Math { source: String, layout: Arc<MathBox> },
}

/// One wrapped display line, with its own real height/depth (the max
/// ascent/descent across every run on it) -- this is what replaces the
/// old "every line is exactly `line_height` tall" assumption, which broke
/// as soon as a line could contain a math run taller than plain text.
#[derive(Debug, Clone)]
pub struct WrappedLine {
    pub runs: Vec<LaidOutRun>,
    pub height: f32,
    pub depth: f32,
}

/// `pub(crate)`, not private: `chat_history.rs` needs this same
/// run-to-font mapping to measure a wrapped line's total width (summing
/// each run's width in its own font) when sizing a bubble, without
/// duplicating the bold/italic/monospace-to-`Font` logic.
pub(crate) fn run_font(run: &TextRun) -> Font {
    let mut font = if run.monospace {
        Font::monospace()
    } else {
        Font::sans_serif()
    };
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
///
/// A malformed math span (one `support::math::parser::parse_math` can't
/// parse) falls back to a literal, monospace text run of the raw
/// `$...$`/`$$...$$` source, including the delimiters -- this must never
/// panic on a model's raw text, so an unparseable formula degrades to
/// "shows the LaTeX source" rather than crashing the chat bubble.
pub fn markdown_to_runs(source: &str) -> Vec<Vec<StyledRun>> {
    let mut lines: Vec<Vec<StyledRun>> = vec![Vec::new()];
    let mut bold_depth = 0usize;
    let mut italic_depth = 0usize;
    let mut code_depth = 0usize;

    let push_text =
        |lines: &mut Vec<Vec<StyledRun>>, text: &str, bold: bool, italic: bool, monospace: bool| {
            let mut parts = text.split('\n');
            if let Some(first) = parts.next() {
                if !first.is_empty() {
                    lines.last_mut().unwrap().push(StyledRun::Text(TextRun {
                        text: first.to_string(),
                        bold,
                        italic,
                        monospace,
                    }));
                }
            }
            for part in parts {
                lines.push(Vec::new());
                if !part.is_empty() {
                    lines.last_mut().unwrap().push(StyledRun::Text(TextRun {
                        text: part.to_string(),
                        bold,
                        italic,
                        monospace,
                    }));
                }
            }
        };

    let push_math =
        |lines: &mut Vec<Vec<StyledRun>>, tex: &str, display: bool, bold: bool, italic: bool| {
            match math::parser::parse_math(tex) {
                Ok(node) => lines.last_mut().unwrap().push(StyledRun::Math(MathRun {
                    source: tex.to_string(),
                    display,
                    ast: Arc::new(node),
                })),
                Err(_) => {
                    let delim = if display { "$$" } else { "$" };
                    lines.last_mut().unwrap().push(StyledRun::Text(TextRun {
                        text: format!("{delim}{tex}{delim}"),
                        bold,
                        italic,
                        monospace: true,
                    }));
                }
            }
        };

    for event in Parser::new_ext(source, Options::ENABLE_MATH) {
        match event {
            Event::Text(text) => push_text(
                &mut lines,
                &text,
                bold_depth > 0,
                italic_depth > 0,
                code_depth > 0,
            ),
            Event::Code(text) => {
                push_text(&mut lines, &text, bold_depth > 0, italic_depth > 0, true)
            }
            Event::InlineMath(tex) => {
                push_math(&mut lines, &tex, false, bold_depth > 0, italic_depth > 0)
            }
            Event::DisplayMath(tex) => {
                lines.push(Vec::new());
                push_math(&mut lines, &tex, true, bold_depth > 0, italic_depth > 0);
                lines.push(Vec::new());
            }
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
                lines.last_mut().unwrap().push(StyledRun::Text(TextRun {
                    text: "\u{2022} ".to_string(),
                    bold: false,
                    italic: false,
                    monospace: false,
                }));
            }
            Event::End(TagEnd::Item) => lines.push(Vec::new()),
            Event::End(TagEnd::Paragraph) => lines.push(Vec::new()),
            Event::SoftBreak => push_text(
                &mut lines,
                " ",
                bold_depth > 0,
                italic_depth > 0,
                code_depth > 0,
            ),
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
///
/// A math run is laid out here (the first stage with `Canvas` access)
/// and treated as one atomic, unsplittable "word" -- wrapping happens
/// *before* it if it doesn't fit, never inside it.
pub fn wrap_runs(
    canvas: &mut Canvas,
    lines: &[Vec<StyledRun>],
    max_width: f32,
) -> Vec<WrappedLine> {
    let mut out = Vec::new();

    for line in lines {
        if line.is_empty() {
            out.push(WrappedLine {
                runs: Vec::new(),
                height: 0.0,
                depth: 0.0,
            });
            continue;
        }

        let mut current: Vec<LaidOutRun> = Vec::new();
        let mut current_width = 0.0f32;

        for run in line {
            match run {
                StyledRun::Text(text_run) => {
                    canvas.font(run_font(text_run));
                    for (word_index, word) in text_run.text.split(' ').enumerate() {
                        let piece = if word_index > 0 {
                            format!(" {word}")
                        } else {
                            word.to_string()
                        };
                        if piece.is_empty() {
                            continue;
                        }
                        let piece_width = canvas.text_width(&piece);

                        if current_width > 0.0 && current_width + piece_width > max_width {
                            out.push(finish_line(std::mem::take(&mut current)));
                            let trimmed = piece.trim_start().to_string();
                            let trimmed_width = canvas.text_width(&trimmed);
                            append_text_run(&mut current, trimmed, text_run);
                            current_width = trimmed_width;
                        } else {
                            append_text_run(&mut current, piece, text_run);
                            current_width += piece_width;
                        }
                    }
                }
                StyledRun::Math(math_run) => {
                    let style = if math_run.display {
                        math::style::MathStyle::Display
                    } else {
                        math::style::MathStyle::Text
                    };
                    let font_size = canvas.current_font_size();
                    let math_box =
                        math::layout::layout_math(&math_run.ast, style, font_size, canvas);
                    let width = math_box.width;

                    if current_width > 0.0 && current_width + width > max_width {
                        out.push(finish_line(std::mem::take(&mut current)));
                        current_width = 0.0;
                    }
                    current.push(LaidOutRun::Math {
                        source: math_run.source.clone(),
                        layout: Arc::new(math_box),
                    });
                    current_width += width;
                }
            }
        }
        out.push(finish_line(current));
    }

    out
}

fn finish_line(runs: Vec<LaidOutRun>) -> WrappedLine {
    let mut height = 0.0f32;
    let mut depth = 0.0f32;
    for run in &runs {
        match run {
            LaidOutRun::Text(_) => {
                // Real per-run text metrics need a `Canvas` (and its
                // currently-set font/size, which have moved on by the
                // time `finish_line` runs) -- callers fall back to the
                // line's own text-only height via `Canvas::font_metrics`
                // at draw time for plain-text-only lines; a line
                // containing at least one math run uses that run's real
                // box metrics, which is the case that actually needed
                // fixing (see this module's own doc comment on the old
                // "every line is `line_height` tall" assumption).
            }
            LaidOutRun::Math { layout, .. } => {
                height = height.max(layout.height);
                depth = depth.max(layout.depth);
            }
        }
    }
    WrappedLine {
        runs,
        height,
        depth,
    }
}

fn append_text_run(current: &mut Vec<LaidOutRun>, text: String, style: &TextRun) {
    if text.is_empty() {
        return;
    }
    if let Some(LaidOutRun::Text(last)) = current.last_mut() {
        if last.bold == style.bold
            && last.italic == style.italic
            && last.monospace == style.monospace
        {
            last.text.push_str(&text);
            return;
        }
    }
    current.push(LaidOutRun::Text(TextRun {
        text,
        bold: style.bold,
        italic: style.italic,
        monospace: style.monospace,
    }));
}

/// Total rendered height of `lines` at `base_font_size` -- matches
/// exactly the vertical advance [`draw_runs`] uses internally, so a
/// caller that needs to reserve space *before* drawing (bubble sizing,
/// stacking a thinking section above a response section) doesn't have
/// to duplicate that cursor math or fall back to the old "N lines ×
/// scalar `line_height`" assumption, which breaks as soon as a line can
/// contain a math run taller than plain text.
pub fn measure_wrapped_height(
    canvas: &mut Canvas,
    lines: &[WrappedLine],
    base_font_size: f32,
) -> f32 {
    canvas.font_size(base_font_size);
    let leading = canvas.font_metrics().leading.max(base_font_size * 0.2);
    let mut total = 0.0f32;
    for line in lines {
        canvas.font_size(base_font_size);
        let text_metrics = canvas.font_metrics();
        let line_height = line.height.max(text_metrics.ascent);
        let line_depth = line.depth.max(text_metrics.descent);
        total += line_height + line_depth + leading;
    }
    total
}

/// Draws already-wrapped lines starting at `origin`, switching font and
/// advancing `x` per run, and `y` by each line's own real
/// `height`/`depth` (plus a small fixed leading) rather than a shared
/// scalar `line_height`. `color` is the default fill for every run --
/// this renderer doesn't vary color per Markdown construct (no syntax
/// highlighting inside chat text), only weight/style/family.
pub fn draw_runs(
    canvas: &mut Canvas,
    lines: &[WrappedLine],
    origin: Point,
    base_font_size: f32,
    color: Color,
) {
    canvas.fill_style(color);
    canvas.font_size(base_font_size);
    let leading = canvas.font_metrics().leading.max(base_font_size * 0.2);
    let mut y = origin.y;

    for line in lines {
        canvas.font_size(base_font_size);
        let text_metrics = canvas.font_metrics();
        let line_height = line.height.max(text_metrics.ascent);
        let line_depth = line.depth.max(text_metrics.descent);
        let baseline = y + line_height;

        let mut x = origin.x;
        for run in &line.runs {
            match run {
                LaidOutRun::Text(text_run) => {
                    canvas.font(run_font(text_run));
                    canvas.fill_text(&text_run.text, Point::new(x, baseline));
                    x += canvas.text_width(&text_run.text);
                }
                LaidOutRun::Math { layout, .. } => {
                    math::draw::draw_math_box(canvas, layout, Point::new(x, baseline), color);
                    x += layout.width;
                }
            }
        }
        y = baseline + line_depth + leading;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn text(s: &str) -> StyledRun {
        StyledRun::Text(TextRun {
            text: s.to_string(),
            bold: false,
            italic: false,
            monospace: false,
        })
    }

    #[test]
    fn plain_text_is_a_single_unstyled_run() {
        let lines = markdown_to_runs("hello world");
        assert_eq!(lines.len(), 1);
        let StyledRun::Text(run) = &lines[0][0] else {
            panic!("expected a Text run")
        };
        assert_eq!(
            run,
            &TextRun {
                text: "hello world".to_string(),
                bold: false,
                italic: false,
                monospace: false
            }
        );
    }

    #[test]
    fn bold_italic_and_inline_code_produce_distinct_runs() {
        let lines = markdown_to_runs("**bold** and *italic* and `code`");
        assert_eq!(lines.len(), 1);
        let texts: Vec<&TextRun> = lines[0]
            .iter()
            .map(|r| match r {
                StyledRun::Text(t) => t,
                StyledRun::Math(_) => panic!("expected only Text runs"),
            })
            .collect();
        assert_eq!(
            texts[0],
            &TextRun {
                text: "bold".to_string(),
                bold: true,
                italic: false,
                monospace: false
            }
        );
        assert_eq!(
            texts[2],
            &TextRun {
                text: "italic".to_string(),
                bold: false,
                italic: true,
                monospace: false
            }
        );
        assert_eq!(
            texts[4],
            &TextRun {
                text: "code".to_string(),
                bold: false,
                italic: false,
                monospace: true
            }
        );
    }

    #[test]
    fn a_code_block_preserves_its_text_without_treating_markdown_syntax_inside_it_specially() {
        let lines = markdown_to_runs("```\nlet x = **not bold**;\n```");
        let code_text: String = lines
            .iter()
            .flat_map(|line| {
                line.iter().map(|r| match r {
                    StyledRun::Text(t) => t.text.as_str(),
                    StyledRun::Math(_) => "",
                })
            })
            .collect::<Vec<_>>()
            .join("\n");
        assert!(
            code_text.contains("let x = **not bold**;"),
            "code block text was mangled: {code_text:?}"
        );
        assert!(
            lines.iter().flatten().all(|r| match r {
                StyledRun::Text(t) => t.monospace || t.text.trim().is_empty(),
                StyledRun::Math(_) => false,
            }),
            "expected every non-empty run inside the code block to be monospace"
        );
    }

    #[test]
    fn a_bullet_list_prefixes_each_item_with_a_bullet_on_its_own_line() {
        let lines = markdown_to_runs("- first\n- second");
        let rendered: Vec<String> = lines
            .iter()
            .map(|line| {
                line.iter()
                    .map(|r| match r {
                        StyledRun::Text(t) => t.text.as_str(),
                        StyledRun::Math(_) => "",
                    })
                    .collect::<String>()
            })
            .collect();
        assert!(
            rendered.contains(&"\u{2022} first".to_string()),
            "rendered lines: {rendered:?}"
        );
        assert!(
            rendered.contains(&"\u{2022} second".to_string()),
            "rendered lines: {rendered:?}"
        );
    }

    #[test]
    fn a_heading_is_rendered_bold() {
        let lines = markdown_to_runs("# Title");
        let bold_run = lines.iter().flatten().find_map(|r| match r {
            StyledRun::Text(t) if t.text.contains("Title") => Some(t),
            _ => None,
        });
        assert!(
            bold_run.is_some_and(|r| r.bold),
            "expected the heading text to be a bold run"
        );
    }

    #[test]
    fn wrap_runs_keeps_every_line_within_max_width() {
        let mut canvas = Canvas::new(400, 400).unwrap();
        let lines =
            markdown_to_runs("the quick brown **fox** jumps over the lazy dog again and again");

        let wrapped = wrap_runs(&mut canvas, &lines, 100.0);

        assert!(
            wrapped.len() > 1,
            "expected wrapping to produce multiple lines"
        );
        for line in &wrapped {
            let mut width = 0.0f32;
            for run in &line.runs {
                if let LaidOutRun::Text(t) = run {
                    canvas.font(run_font(t));
                    width += canvas.text_width(&t.text);
                }
            }
            assert!(width <= 101.0, "line exceeds the 100px max width ({width})");
        }
    }

    #[test]
    fn inline_math_parses_into_a_math_run() {
        let lines = markdown_to_runs("Hello $x^2$ world");
        let runs = &lines[0];
        let math_run = runs.iter().find_map(|r| match r {
            StyledRun::Math(m) => Some(m),
            _ => None,
        });
        assert!(
            math_run.is_some_and(|m| m.source == "x^2" && !m.display),
            "expected an inline math run for x^2"
        );
    }

    #[test]
    fn display_math_parses_into_a_math_run_on_its_own_line() {
        let lines = markdown_to_runs("$$\\frac{1}{2}$$");
        let math_run = lines.iter().flatten().find_map(|r| match r {
            StyledRun::Math(m) => Some(m),
            _ => None,
        });
        assert!(
            math_run.is_some_and(|m| m.display),
            "expected a display math run"
        );
    }

    #[test]
    fn malformed_math_falls_back_to_a_literal_text_run_instead_of_panicking() {
        let lines = markdown_to_runs("$\\notarealcommand$");
        let run = lines
            .iter()
            .flatten()
            .next()
            .expect("expected at least one run");
        let StyledRun::Text(text_run) = run else {
            panic!("expected a Text fallback run, got {run:?}")
        };
        assert!(
            text_run.text.contains("notarealcommand"),
            "expected the raw source to survive in the fallback text"
        );
    }

    #[test]
    fn a_math_run_is_wrapped_as_one_atomic_unsplittable_word() {
        let mut canvas = Canvas::new(400, 400).unwrap();
        let lines = vec![vec![
            text("start "),
            StyledRun::Math(MathRun {
                source: "x^2".to_string(),
                display: false,
                ast: Arc::new(math::parser::parse_math("x^2").unwrap()),
            }),
        ]];
        let wrapped = wrap_runs(&mut canvas, &lines, 10_000.0);
        let has_whole_math_run = wrapped.iter().any(|line| {
            line.runs
                .iter()
                .any(|r| matches!(r, LaidOutRun::Math { source, .. } if source == "x^2"))
        });
        assert!(
            has_whole_math_run,
            "expected the math run to survive intact, not split across lines"
        );
    }

    #[test]
    fn a_line_containing_math_reports_the_math_boxs_real_height() {
        let mut canvas = Canvas::new(400, 400).unwrap();
        canvas.font_size(20.0);
        let lines = vec![vec![StyledRun::Math(MathRun {
            source: "\\frac{1}{2}".to_string(),
            display: true,
            ast: Arc::new(math::parser::parse_math("\\frac{1}{2}").unwrap()),
        })]];
        let wrapped = wrap_runs(&mut canvas, &lines, 10_000.0);
        assert!(
            wrapped[0].height > 0.0,
            "expected a real, non-zero height from the fraction's own box metrics"
        );
        assert!(
            wrapped[0].depth > 0.0,
            "expected a real, non-zero depth from the fraction's own box metrics"
        );
    }
}
