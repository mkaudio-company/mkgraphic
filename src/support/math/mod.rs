//! Real typeset math rendering for the chat/Markdown pipeline
//! (`support::markdown`), hand-rolled against the real TeX layout rules
//! (Knuth's TeXbook Appendix G) rather than a third-party crate --
//! deliberately, to avoid depending on a math-typesetting crate young
//! enough to still be churning weekly and requiring a parallel bundled-font
//! glyph-rendering path alongside this crate's own `Canvas`/`Font` system.
//!
//! Pipeline: [`parser::parse_math`] (LaTeX source -> [`ast::MathNode`],
//! no `Canvas` dependency) -> [`layout::layout_math`] (`MathNode` ->
//! [`layout::MathBox`], a TeX-style box with width/height/depth relative
//! to its own baseline -- needs `Canvas` for real glyph widths/metrics)
//! -> [`draw::draw_math_box`] (paints a laid-out `MathBox` at a given
//! origin). `support::markdown` is the only consumer -- see that
//! module's `StyledRun`/`wrap_runs`/`draw_runs` for how a math run is
//! spliced into an otherwise-plain-text wrapped line.
//!
//! Scoped to the realistic vocabulary an LLM chat reply's LaTeX actually
//! uses (fractions, super/subscripts, radicals, `\left`/`\right` growing
//! delimiters, `\sum`/`\int`/etc. with limits, `\text{}`, Greek letters,
//! common operators/relations) -- not full LaTeX. Full TeX Rule-20
//! inter-atom spacing is also out of scope for now (see `ast::AtomClass`'s
//! own doc comment).

pub mod ast;
pub mod delimiter;
pub mod draw;
pub mod glyphs;
pub mod layout;
pub mod parser;
pub mod radical;
pub mod style;
