//! The parsed representation of a math expression -- deliberately scoped
//! to the realistic vocabulary a chat assistant's LaTeX output actually
//! uses (fractions, scripts, radicals, growing delimiters, sum/integral
//! limits, Greek letters and common operators/relations), not full
//! LaTeX. See `support::math`'s own module doc comment for the overall
//! pipeline this feeds into.

/// One parsed math expression, ready for [`super::layout::layout_math`].
#[derive(Debug, Clone, PartialEq)]
pub enum MathNode {
    /// A single glyph -- a letter, digit, or looked-up symbol (Greek,
    /// operator, relation). `class` drives spacing decisions; full
    /// TeX Rule-20 inter-atom spacing is out of scope for now (see this
    /// module's own simplification note on [`AtomClass`]), but the class
    /// is threaded through from the start so that can be added later
    /// without an AST change.
    Symbol { glyph: char, class: AtomClass },
    /// A sequence of atoms laid out left to right, e.g. `x+1`.
    Row(Vec<MathNode>),
    /// `\text{...}` -- upright (non-italic) text inside math, e.g. unit
    /// labels like `\text{Hz}`.
    Text(String),
    /// `\frac{num}{den}`.
    Frac {
        num: Box<MathNode>,
        den: Box<MathNode>,
    },
    /// `base^{sup}`, `base_{sub}`, or `base_{sub}^{sup}` -- both present
    /// at once is legal and common (`x_i^2`).
    Script {
        base: Box<MathNode>,
        sup: Option<Box<MathNode>>,
        sub: Option<Box<MathNode>>,
    },
    /// `\sqrt{radicand}`. `\sqrt[n]{...}` (an explicit root index) is not
    /// supported in this first pass -- realistically rare in chat
    /// replies, and can be added later as an extra field without
    /// changing this variant's shape for callers that don't use it.
    Sqrt(Box<MathNode>),
    /// `\left X ... \right Y` -- delimiters that grow to fit their
    /// content. Either side may be absent (`\left.`/`\right.` in real
    /// LaTeX means "no visible delimiter on this side").
    Delimited {
        open: Option<DelimiterKind>,
        body: Box<MathNode>,
        close: Option<DelimiterKind>,
    },
    /// `\sum`/`\int`/`\prod`/etc. with optional lower/upper limits.
    BigOp {
        kind: BigOpKind,
        lower: Option<Box<MathNode>>,
        upper: Option<Box<MathNode>>,
    },
}

/// TeX's atom classification (TeXbook ch. 17) -- carried per [`MathNode::
/// Symbol`] for future inter-atom spacing (Rule 20's 8x8 spacing-class
/// matrix). **Not yet used for spacing** -- `layout::layout_row` inserts
/// one fixed small gap between every pair of adjacent atoms regardless of
/// class, a deliberate v1 simplification (documented here so it isn't
/// mistaken for an oversight), since getting the *shapes* right
/// (fractions/radicals/scripts/delimiters/limits) matters far more than
/// spacing nuance for legibility of an LLM's math output.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AtomClass {
    /// Ordinary: letters, digits, most symbols.
    Ord,
    /// Large operator glyphs not covered by [`BigOpKind`] (rare on their
    /// own; most big operators go through `MathNode::BigOp` instead).
    Op,
    /// Binary operator: `+`, `-`, `\times`, `\cdot`, `\pm`, ...
    Bin,
    /// Relation: `=`, `\leq`, `\geq`, `\neq`, `\in`, `\to`, ...
    Rel,
    /// Opening punctuation/delimiter glyph used outside `\left`/`\right`.
    Open,
    /// Closing punctuation/delimiter glyph used outside `\left`/`\right`.
    Close,
    /// Punctuation: `,`, `;`.
    Punct,
    /// Inner: a sub-formula already wrapped in its own delimiters.
    Inner,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DelimiterKind {
    Paren,
    Bracket,
    Brace,
    Floor,
    Ceil,
    Bar,
    DoubleBar,
    AngleLeft,
    AngleRight,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BigOpKind {
    Sum,
    Prod,
    Int,
    Oint,
    BigCup,
    BigCap,
    Lim,
    Max,
    Min,
}

impl BigOpKind {
    /// The glyph (or, for `\lim`/`\max`/`\min`, the literal word) drawn
    /// for this operator.
    pub fn glyph(self) -> &'static str {
        match self {
            Self::Sum => "\u{2211}",
            Self::Prod => "\u{220F}",
            Self::Int => "\u{222B}",
            Self::Oint => "\u{222E}",
            Self::BigCup => "\u{22C3}",
            Self::BigCap => "\u{22C2}",
            Self::Lim => "lim",
            Self::Max => "max",
            Self::Min => "min",
        }
    }

    /// TeX Rule 13's display/text-style distinction: in display style,
    /// `\sum`/`\prod`/`\bigcup`/`\bigcap`/`\lim`/`\max`/`\min` stack their
    /// limits directly above/below the operator; `\int`/`\oint`
    /// conventionally keep corner scripts (to the operator's
    /// upper-right/lower-right) even in display style. In text style,
    /// every operator falls back to corner scripts regardless of this
    /// flag -- see `layout::layout_bigop`.
    pub fn display_limits(self) -> bool {
        !matches!(self, Self::Int | Self::Oint)
    }
}
