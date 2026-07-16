//! TeX's math "style" (TeXbook ch. 17 / Appendix G): each nested level of
//! a formula (a fraction's numerator, a superscript, a superscript's own
//! superscript, ...) is typeset in a smaller and/or cramped variant of
//! its parent's style, which is what makes nested scripts/fractions
//! shrink the way real math notation does.

/// `Cramped` variants suppress the extra headroom a superscript would
/// otherwise reserve above tall content (used for fraction denominators
/// and subscripts, where that headroom would look wrong).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MathStyle {
    Display,
    DisplayCramped,
    Text,
    TextCramped,
    Script,
    ScriptCramped,
    ScriptScript,
    ScriptScriptCramped,
}

impl MathStyle {
    pub fn cramped(self) -> Self {
        match self {
            Self::Display | Self::DisplayCramped => Self::DisplayCramped,
            Self::Text | Self::TextCramped => Self::TextCramped,
            Self::Script | Self::ScriptCramped => Self::ScriptCramped,
            Self::ScriptScript | Self::ScriptScriptCramped => Self::ScriptScriptCramped,
        }
    }

    pub fn is_display(self) -> bool {
        matches!(self, Self::Display | Self::DisplayCramped)
    }

    pub fn is_cramped(self) -> bool {
        matches!(
            self,
            Self::DisplayCramped
                | Self::TextCramped
                | Self::ScriptCramped
                | Self::ScriptScriptCramped
        )
    }

    /// One level smaller, preserving crampedness -- `Display`/`Text` both
    /// map to `Script` (`Text` is not itself smaller than `Display`, only
    /// the *next* level down is), `Script` maps to `ScriptScript`,
    /// `ScriptScript` saturates (there's nothing smaller in this scheme).
    pub fn smaller(self) -> Self {
        match self {
            Self::Display | Self::Text => Self::Script,
            Self::DisplayCramped | Self::TextCramped => Self::ScriptCramped,
            Self::Script => Self::ScriptScript,
            Self::ScriptCramped => Self::ScriptScriptCramped,
            Self::ScriptScript => Self::ScriptScript,
            Self::ScriptScriptCramped => Self::ScriptScriptCramped,
        }
    }

    /// Rule 15's numerator style: display style numerators stay full
    /// size (dropping only to `Text`, not `Script`) -- this is what gives
    /// a displayed fraction its characteristically large numerator/
    /// denominator, unlike an inline superscript which always shrinks.
    pub fn numerator_style(self) -> Self {
        match self {
            Self::Display => Self::Text,
            Self::DisplayCramped => Self::TextCramped,
            other => other.smaller(),
        }
    }

    /// Rule 15's denominator style: always cramped (a denominator is
    /// never itself divided visually the way an uncramped style would
    /// imply extra headroom for).
    pub fn denominator_style(self) -> Self {
        match self {
            Self::Display | Self::DisplayCramped => Self::TextCramped,
            other => other.smaller().cramped(),
        }
    }

    /// Rule 18's superscript style: one level smaller, crampedness
    /// inherited from the base (a superscript on already-cramped content
    /// -- e.g. inside a square root -- stays cramped).
    pub fn sup_style(self) -> Self {
        self.smaller()
    }

    /// Rule 18's subscript style: one level smaller and always cramped.
    pub fn sub_style(self) -> Self {
        self.smaller().cramped()
    }

    /// Relative size multiplier applied to the base font size at this
    /// style -- `Display`/`Text` are full size; `Script` is TeX's
    /// conventional ~70%; `ScriptScript` ~50%.
    pub fn size_scale(self) -> f32 {
        match self {
            Self::Display | Self::DisplayCramped | Self::Text | Self::TextCramped => 1.0,
            Self::Script | Self::ScriptCramped => 0.7,
            Self::ScriptScript | Self::ScriptScriptCramped => 0.5,
        }
    }
}

/// Appendix G's font parameters (classic cmsy10/cmex10 values), scaled to
/// a concrete font size. These are the same regardless of which real
/// font is in use -- without a font's own OTF MATH table (not available
/// here), these classic constants are the standard fallback real
/// implementations (including KaTeX) also use when one isn't present.
/// Values marked "approx" in the doc comments below are the least
/// independently verified this session -- check against a primary source
/// (TeXbook Appendix G Table 13, or `fontMetricsData.js` in a real KaTeX
/// checkout) before fine-tuning further; the *structure* of every rule
/// that uses them is what matters most, not the exact fourth decimal.
#[derive(Debug, Clone, Copy)]
pub struct FontParams {
    pub sup1: f32,
    pub sup2: f32,
    pub sup3: f32,
    pub sub1: f32,
    pub sub2: f32,
    pub sup_drop: f32,
    pub sub_drop: f32,
    pub axis_height: f32,
    pub default_rule_thickness: f32,
    pub num1: f32,
    pub num2: f32,
    pub num3: f32,
    pub denom1: f32,
    pub denom2: f32,
    /// Approx -- gap between a display-style stacked upper limit's bottom
    /// and the operator glyph's top.
    pub big_op_spacing1: f32,
    /// Approx -- gap between the operator glyph's bottom and a
    /// display-style stacked lower limit's top.
    pub big_op_spacing2: f32,
    /// Approx -- additional gap above the upper limit / below the lower
    /// limit when the operator's own natural spacing needs padding out.
    pub big_op_spacing3: f32,
    /// Approx -- max total extra height added above the whole group.
    pub big_op_spacing4: f32,
    /// Approx -- max total extra depth added below the whole group.
    pub big_op_spacing5: f32,
}

impl FontParams {
    pub fn for_size(font_size: f32) -> Self {
        Self {
            sup1: 0.412892 * font_size,
            sup2: 0.362892 * font_size,
            sup3: 0.288889 * font_size,
            sub1: 0.15 * font_size,
            sub2: 0.247217 * font_size,
            sup_drop: 0.386108 * font_size,
            sub_drop: 0.05 * font_size,
            axis_height: 0.25 * font_size,
            default_rule_thickness: 0.04 * font_size,
            num1: 0.677 * font_size,
            num2: 0.394 * font_size,
            num3: 0.444 * font_size,
            denom1: 0.686 * font_size,
            denom2: 0.345 * font_size,
            big_op_spacing1: 0.111 * font_size,
            big_op_spacing2: 0.166 * font_size,
            big_op_spacing3: 0.2 * font_size,
            big_op_spacing4: 0.6 * font_size,
            big_op_spacing5: 0.1 * font_size,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn display_numerator_style_stays_full_size_at_text_level() {
        assert_eq!(MathStyle::Display.numerator_style(), MathStyle::Text);
        assert_eq!(MathStyle::Display.numerator_style().size_scale(), 1.0);
    }

    #[test]
    fn denominator_style_is_always_cramped() {
        assert!(MathStyle::Display.denominator_style().is_cramped());
        assert!(MathStyle::Text.denominator_style().is_cramped());
    }

    #[test]
    fn sup_style_shrinks_but_sub_style_shrinks_and_cramps() {
        assert_eq!(MathStyle::Text.sup_style(), MathStyle::Script);
        assert_eq!(MathStyle::Text.sub_style(), MathStyle::ScriptCramped);
    }

    #[test]
    fn script_script_is_the_smallest_and_saturates() {
        assert_eq!(MathStyle::ScriptScript.smaller(), MathStyle::ScriptScript);
        assert_eq!(MathStyle::ScriptScript.size_scale(), 0.5);
    }

    #[test]
    fn font_params_scale_linearly_with_font_size() {
        let small = FontParams::for_size(10.0);
        let big = FontParams::for_size(20.0);
        assert!((big.axis_height - 2.0 * small.axis_height).abs() < 1e-4);
        assert!((big.default_rule_thickness - 2.0 * small.default_rule_thickness).abs() < 1e-4);
    }
}
