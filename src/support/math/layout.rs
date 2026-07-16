//! `MathNode` -> `MathBox`: TeX's box model (TeXbook ch. 11 / Appendix
//! G). Every box's `width`/`height`/`depth` are measured relative to its
//! *own* baseline (`height` = extent above, `depth` = extent below) --
//! composition means placing a child box's baseline at an offset from
//! its parent's, not stacking top-left rectangles the way the rest of
//! this crate's layout usually works.
//!
//! **Known simplification**: box `height`/`depth` for a single glyph use
//! the *font's* real ascent/descent (`Canvas::font_metrics`), not that
//! specific glyph's own ink extents -- this crate has no per-glyph
//! bounding-box API, and adding one is out of scope here. This means,
//! e.g., a lone `x` (no descender) and a lone `g` (has one) report
//! identical `height`/`depth` today. Visually close enough for chat-reply
//! math; a real fix would read each glyph's outline bbox via
//! `ttf_parser::Face::glyph_bounding_box`.

use crate::support::canvas::Canvas;
use crate::support::font::Font;

use super::ast::{BigOpKind, DelimiterKind, MathNode};
use super::style::{FontParams, MathStyle};

#[derive(Debug, Clone)]
pub struct MathBox {
    pub width: f32,
    pub height: f32,
    pub depth: f32,
    pub content: MathBoxContent,
}

#[derive(Debug, Clone)]
pub enum MathBoxContent {
    /// One or more glyphs in a single font/size -- drawn via the same
    /// `Canvas::fill_text` every plain text run already uses.
    Glyphs {
        text: String,
        font: Font,
        font_size: f32,
    },
    /// Already-laid-out children, each already positioned relative to
    /// this box's own baseline.
    Hlist(Vec<PlacedBox>),
    /// Non-glyph geometry -- fraction bars (Phase 3), radical hooks
    /// (Phase 4), delimiter paths (Phase 5).
    Rule(RuleKind),
}

#[derive(Debug, Clone)]
pub struct PlacedBox {
    pub x_offset: f32,
    /// Positive = this child's baseline sits *below* the parent's own
    /// baseline (e.g. a subscript or a fraction's denominator); negative
    /// = raised above it (a superscript, a fraction's numerator).
    pub baseline_shift: f32,
    pub math_box: MathBox,
}

#[derive(Debug, Clone)]
pub enum RuleKind {
    HorizontalBar {
        width: f32,
        thickness: f32,
    },
    /// A radical (√) sign: hook rises `total_height` above this rule
    /// box's own baseline, vinculum spans `radicand_width` to the right
    /// of the hook (see `super::radical`).
    Radical {
        total_height: f32,
        radicand_width: f32,
        thickness: f32,
    },
    /// A `\left`/`\right` growing delimiter (see `super::delimiter`).
    /// `is_open` mirrors the shape horizontally for the closing side --
    /// most of these shapes (`(`/`)`, brackets, braces, floor/ceil,
    /// angles) aren't symmetric, so which side they're drawn on matters.
    Delimiter {
        kind: DelimiterKind,
        thickness: f32,
        is_open: bool,
    },
}

/// Lays out `node` at `font_size` in the given [`MathStyle`], using
/// `canvas` for real glyph widths/metrics (matching `wrap_runs`'s own
/// existing `&mut Canvas` need for the same reason).
pub fn layout_math(
    node: &MathNode,
    style: MathStyle,
    font_size: f32,
    canvas: &mut Canvas,
) -> MathBox {
    match node {
        MathNode::Symbol { glyph, .. } => layout_symbol(*glyph, style, font_size, canvas),
        MathNode::Row(items) => layout_row(items, style, font_size, canvas),
        MathNode::Text(text) => layout_text(text, style, font_size, canvas),
        MathNode::Script { base, sup, sub } => layout_script(
            base,
            sup.as_deref(),
            sub.as_deref(),
            style,
            font_size,
            canvas,
        ),
        MathNode::Frac { num, den } => layout_frac(num, den, style, font_size, canvas),
        MathNode::Sqrt(radicand) => layout_sqrt(radicand, style, font_size, canvas),
        MathNode::Delimited { open, body, close } => {
            layout_delimited(*open, body, *close, style, font_size, canvas)
        }
        MathNode::BigOp { kind, lower, upper } => layout_bigop(
            *kind,
            lower.as_deref(),
            upper.as_deref(),
            style,
            font_size,
            canvas,
        ),
    }
}

/// Approximate x-height -- half the font's real ascent, since `Canvas`
/// has no direct x-height accessor (see this module's caching sibling,
/// `Canvas::font_metrics`, whose own doc comment notes the same gap).
/// Used only for Rule 18's "ink must clear N% of x-height" checks, where
/// an approximation is acceptable -- those checks are a minimum-clearance
/// safety net, not a precise typographic measurement.
fn approx_x_height(style: MathStyle, font_size: f32, canvas: &mut Canvas) -> f32 {
    canvas.font_size(font_size * style.size_scale());
    canvas.font_metrics().ascent * 0.5
}

fn layout_symbol(glyph: char, style: MathStyle, font_size: f32, canvas: &mut Canvas) -> MathBox {
    let scaled_size = font_size * style.size_scale();
    // TeX's math-italic convention: letters (Latin and Greek alike --
    // both are `char::is_alphabetic`) render italic; digits, operators,
    // and punctuation stay upright.
    let font = if glyph.is_alphabetic() {
        Font::sans_serif().italic()
    } else {
        Font::sans_serif()
    };
    canvas.font(font.clone());
    canvas.font_size(scaled_size);
    let text = glyph.to_string();
    let width = canvas.text_width(&text);
    let metrics = canvas.font_metrics();
    MathBox {
        width,
        height: metrics.ascent,
        depth: metrics.descent,
        content: MathBoxContent::Glyphs {
            text,
            font,
            font_size: scaled_size,
        },
    }
}

fn layout_text(text: &str, style: MathStyle, font_size: f32, canvas: &mut Canvas) -> MathBox {
    let scaled_size = font_size * style.size_scale();
    let font = Font::sans_serif();
    canvas.font(font.clone());
    canvas.font_size(scaled_size);
    let width = canvas.text_width(text);
    let metrics = canvas.font_metrics();
    MathBox {
        width,
        height: metrics.ascent,
        depth: metrics.descent,
        content: MathBoxContent::Glyphs {
            text: text.to_string(),
            font,
            font_size: scaled_size,
        },
    }
}

/// Concatenates atoms left to right with one small fixed gap between
/// each pair -- real TeX Rule 20 varies this gap by the adjacent atoms'
/// `AtomClass` pairing (e.g. more space around a binary operator than
/// around ordinary/ordinary); deferred for v1, see `AtomClass`'s own doc
/// comment for why that's a documented simplification, not an oversight.
fn layout_row(
    items: &[MathNode],
    style: MathStyle,
    font_size: f32,
    canvas: &mut Canvas,
) -> MathBox {
    let params = FontParams::for_size(font_size * style.size_scale());
    let gap = params.default_rule_thickness * 3.0;

    let mut children = Vec::with_capacity(items.len());
    let mut x = 0.0f32;
    let mut height = 0.0f32;
    let mut depth = 0.0f32;

    for (i, item) in items.iter().enumerate() {
        if i > 0 {
            x += gap;
        }
        let child_box = layout_math(item, style, font_size, canvas);
        height = height.max(child_box.height);
        depth = depth.max(child_box.depth);
        let width = child_box.width;
        children.push(PlacedBox {
            x_offset: x,
            baseline_shift: 0.0,
            math_box: child_box,
        });
        x += width;
    }

    MathBox {
        width: x,
        height,
        depth,
        content: MathBoxContent::Hlist(children),
    }
}

/// Rule 18: superscript/subscript placement. Both may be present at
/// once (`x_i^2`); when they are, both are placed at the same
/// horizontal offset (stacked vertically after the base), and their
/// shifts are additionally constrained to keep a minimum gap between
/// them.
fn layout_script(
    base: &MathNode,
    sup: Option<&MathNode>,
    sub: Option<&MathNode>,
    style: MathStyle,
    font_size: f32,
    canvas: &mut Canvas,
) -> MathBox {
    let params = FontParams::for_size(font_size);
    let base_box = layout_math(base, style, font_size, canvas);
    let x_height = approx_x_height(style, font_size, canvas);

    let sup_box = sup.map(|s| layout_math(s, style.sup_style(), font_size, canvas));
    let sub_box = sub.map(|s| layout_math(s, style.sub_style(), font_size, canvas));

    let mut sup_shift = 0.0f32;
    if let Some(sup_box) = &sup_box {
        let base_shift = if style.is_cramped() {
            params.sup3
        } else if style.is_display() {
            params.sup1
        } else {
            params.sup2
        };
        let clearance = base_box.height - params.sup_drop;
        let ink_min = sup_box.depth + x_height * 0.25;
        sup_shift = base_shift.max(clearance).max(ink_min);
    }

    let mut sub_shift = 0.0f32;
    if let Some(sub_box) = &sub_box {
        let base_shift = if style.is_display() {
            params.sub1
        } else {
            params.sub2
        };
        let clearance = base_box.depth + params.sub_drop;
        let ink_min = sub_box.height + x_height * 0.8;
        sub_shift = base_shift.max(clearance).max(ink_min);
    }

    if let (Some(sup_box), Some(sub_box)) = (&sup_box, &sub_box) {
        let gap = (sup_shift - sup_box.depth) - (sub_box.height - sub_shift);
        let min_gap = 4.0 * params.default_rule_thickness;
        if gap < min_gap {
            let push = (min_gap - gap) / 2.0;
            sup_shift += push;
            sub_shift += push;
        }
    }

    let script_width = match (&sup_box, &sub_box) {
        (Some(s), Some(b)) => s.width.max(b.width),
        (Some(s), None) => s.width,
        (None, Some(b)) => b.width,
        (None, None) => 0.0,
    };

    let mut height = base_box.height;
    let mut depth = base_box.depth;
    let base_width = base_box.width;
    let mut children = vec![PlacedBox {
        x_offset: 0.0,
        baseline_shift: 0.0,
        math_box: base_box,
    }];

    if let Some(sup_box) = sup_box {
        height = height.max(sup_shift + sup_box.height);
        depth = depth.max((sup_box.depth - sup_shift).max(0.0));
        children.push(PlacedBox {
            x_offset: base_width,
            baseline_shift: -sup_shift,
            math_box: sup_box,
        });
    }
    if let Some(sub_box) = sub_box {
        depth = depth.max(sub_shift + sub_box.depth);
        height = height.max((sub_box.height - sub_shift).max(0.0));
        children.push(PlacedBox {
            x_offset: base_width,
            baseline_shift: sub_shift,
            math_box: sub_box,
        });
    }

    MathBox {
        width: base_width + script_width,
        height,
        depth,
        content: MathBoxContent::Hlist(children),
    }
}

/// Rule 15: fraction layout. The bar is centered on the axis (not the
/// baseline -- math notation's horizontal "center line," per
/// `FontParams::axis_height`); numerator/denominator get a minimum
/// clearance from it, symmetrically pushed apart if their natural
/// (style-appropriate) shift would violate that minimum.
fn layout_frac(
    num: &MathNode,
    den: &MathNode,
    style: MathStyle,
    font_size: f32,
    canvas: &mut Canvas,
) -> MathBox {
    let params = FontParams::for_size(font_size);
    let num_box = layout_math(num, style.numerator_style(), font_size, canvas);
    let den_box = layout_math(den, style.denominator_style(), font_size, canvas);
    let thickness = params.default_rule_thickness;

    let (num_shift_ideal, den_shift_ideal, min_gap) = if style.is_display() {
        (params.num1, params.denom1, 3.0 * thickness)
    } else {
        (params.num2, params.denom2, 1.0 * thickness)
    };

    // The bar is centered on the axis, `axis_height` *above* the
    // baseline -- not at the baseline itself -- so numerator/denominator
    // clearance is measured against the bar's own top/bottom edge, each
    // independently (this is two separate minimum-clearance checks, not
    // one shared gap split across both sides).
    let bar_top = params.axis_height + thickness / 2.0;
    let bar_bottom = params.axis_height - thickness / 2.0;

    let num_clearance = (num_shift_ideal - num_box.depth) - bar_top;
    let num_shift = if num_clearance < min_gap {
        num_shift_ideal + (min_gap - num_clearance)
    } else {
        num_shift_ideal
    };

    let den_clearance = bar_bottom - (den_box.height - den_shift_ideal);
    let den_shift = if den_clearance < min_gap {
        den_shift_ideal + (min_gap - den_clearance)
    } else {
        den_shift_ideal
    };

    let width = num_box.width.max(den_box.width);
    let bar = MathBox {
        width,
        height: thickness / 2.0,
        depth: thickness / 2.0,
        content: MathBoxContent::Rule(RuleKind::HorizontalBar { width, thickness }),
    };

    let num_width = num_box.width;
    let den_width = den_box.width;
    let children = vec![
        PlacedBox {
            x_offset: (width - num_width) / 2.0,
            baseline_shift: -num_shift,
            math_box: num_box,
        },
        PlacedBox {
            x_offset: 0.0,
            baseline_shift: -params.axis_height,
            math_box: bar,
        },
        PlacedBox {
            x_offset: (width - den_width) / 2.0,
            baseline_shift: den_shift,
            math_box: den_box,
        },
    ];

    let height = num_shift + children[0].math_box.height;
    let depth = den_shift + children[2].math_box.depth;
    MathBox {
        width,
        height,
        depth,
        content: MathBoxContent::Hlist(children),
    }
}

/// Rule 11: radical layout. The radicand keeps the expression's own
/// baseline (no vertical shift -- this is what makes `\sqrt{x}` sit at
/// the same baseline as surrounding text); the radical sign itself rises
/// above it by the radicand's height plus a clearance gap plus the bar's
/// own thickness, so the vinculum floats just above the tallest content
/// it covers.
fn layout_sqrt(
    radicand: &MathNode,
    style: MathStyle,
    font_size: f32,
    canvas: &mut Canvas,
) -> MathBox {
    let params = FontParams::for_size(font_size);
    // The radicand is cramped (Rule 11): no extra headroom is reserved
    // above it the way an uncramped style would imply, since the
    // radical sign itself is what visually "covers" it.
    let radicand_box = layout_math(radicand, style.cramped(), font_size, canvas);
    let thickness = params.default_rule_thickness;
    let clearance = thickness + params.axis_height.abs() / 4.0;
    let total_height = radicand_box.height + clearance + thickness;

    let tick = super::radical::tick_width(total_height);
    let width = tick + radicand_box.width;
    let radicand_width = radicand_box.width;
    let depth = radicand_box.depth;

    let sign = MathBox {
        width,
        height: total_height,
        depth: 0.0,
        content: MathBoxContent::Rule(RuleKind::Radical {
            total_height,
            radicand_width,
            thickness,
        }),
    };

    let children = vec![
        PlacedBox {
            x_offset: 0.0,
            baseline_shift: 0.0,
            math_box: sign,
        },
        PlacedBox {
            x_offset: tick,
            baseline_shift: 0.0,
            math_box: radicand_box,
        },
    ];
    MathBox {
        width,
        height: total_height,
        depth,
        content: MathBoxContent::Hlist(children),
    }
}

/// `\left`/`\right`: the body keeps its own natural style (unlike a
/// radicand, a delimited body isn't cramped); each present delimiter is
/// sized to the body's full height+depth (plus a small clearance) and
/// centered on the math axis, which is what makes a tall stretched
/// paren look correctly balanced around the middle of its content rather
/// than around the baseline.
fn layout_delimited(
    open: Option<DelimiterKind>,
    body: &MathNode,
    close: Option<DelimiterKind>,
    style: MathStyle,
    font_size: f32,
    canvas: &mut Canvas,
) -> MathBox {
    let params = FontParams::for_size(font_size);
    let body_box = layout_math(body, style, font_size, canvas);
    let thickness = params.default_rule_thickness;

    let target_height = body_box.height + body_box.depth + 2.0 * thickness;
    let half = target_height / 2.0;
    let height_above_axis = half + params.axis_height;
    let depth_below_axis = (half - params.axis_height).max(0.0);

    let delimiter_box = |kind: DelimiterKind, is_open: bool| MathBox {
        width: super::delimiter::delimiter_width(target_height),
        height: height_above_axis,
        depth: depth_below_axis,
        content: MathBoxContent::Rule(RuleKind::Delimiter {
            kind,
            thickness,
            is_open,
        }),
    };

    let mut children = Vec::with_capacity(3);
    let mut x = 0.0f32;
    let mut height = body_box.height;
    let mut depth = body_box.depth;

    if let Some(kind) = open {
        let b = delimiter_box(kind, true);
        height = height.max(b.height);
        depth = depth.max(b.depth);
        x += b.width;
        children.push(PlacedBox {
            x_offset: 0.0,
            baseline_shift: 0.0,
            math_box: b,
        });
    }
    let body_width = body_box.width;
    children.push(PlacedBox {
        x_offset: x,
        baseline_shift: 0.0,
        math_box: body_box,
    });
    x += body_width;
    if let Some(kind) = close {
        let b = delimiter_box(kind, false);
        height = height.max(b.height);
        depth = depth.max(b.depth);
        children.push(PlacedBox {
            x_offset: x,
            baseline_shift: 0.0,
            math_box: b,
        });
        x += super::delimiter::delimiter_width(target_height);
    }

    MathBox {
        width: x,
        height,
        depth,
        content: MathBoxContent::Hlist(children),
    }
}

/// Rule 13: big-operator limits. In display style, `\sum`/`\prod`/etc.
/// (per [`BigOpKind::display_limits`] -- `\int`/`\oint` are the
/// exception) stack their limits directly above/below the operator
/// glyph, centered on its width. Every other case (text style, or
/// `\int`/`\oint` even in display style) falls back to ordinary corner
/// scripts, using the same placement math as Rule 18.
fn layout_bigop(
    kind: BigOpKind,
    lower: Option<&MathNode>,
    upper: Option<&MathNode>,
    style: MathStyle,
    font_size: f32,
    canvas: &mut Canvas,
) -> MathBox {
    let params = FontParams::for_size(font_size);
    let scaled_size = font_size * style.size_scale();
    let op_text = kind.glyph().to_string();
    let op_font = Font::sans_serif();
    canvas.font(op_font.clone());
    canvas.font_size(scaled_size);
    let op_width = canvas.text_width(&op_text);
    let op_metrics = canvas.font_metrics();
    let op_box = MathBox {
        width: op_width,
        height: op_metrics.ascent,
        depth: op_metrics.descent,
        content: MathBoxContent::Glyphs {
            text: op_text,
            font: op_font,
            font_size: scaled_size,
        },
    };

    if style.is_display() && kind.display_limits() {
        layout_bigop_stacked(op_box, lower, upper, style, font_size, &params, canvas)
    } else {
        layout_bigop_corner_scripts(op_box, lower, upper, style, font_size, &params, canvas)
    }
}

fn layout_bigop_stacked(
    op_box: MathBox,
    lower: Option<&MathNode>,
    upper: Option<&MathNode>,
    style: MathStyle,
    font_size: f32,
    params: &FontParams,
    canvas: &mut Canvas,
) -> MathBox {
    let upper_box = upper.map(|u| layout_math(u, style.sup_style(), font_size, canvas));
    let lower_box = lower.map(|l| layout_math(l, style.sub_style(), font_size, canvas));

    let width = op_box
        .width
        .max(upper_box.as_ref().map_or(0.0, |b| b.width))
        .max(lower_box.as_ref().map_or(0.0, |b| b.width));

    let mut children = Vec::with_capacity(3);
    let mut height = op_box.height;
    let mut depth = op_box.depth;
    let op_width = op_box.width;
    let op_height = op_box.height;
    let op_depth = op_box.depth;

    if let Some(upper_box) = upper_box {
        let shift = op_height + params.big_op_spacing1 + upper_box.depth;
        height = height.max(shift + upper_box.height);
        let x = (width - upper_box.width) / 2.0;
        children.push(PlacedBox {
            x_offset: x,
            baseline_shift: -shift,
            math_box: upper_box,
        });
    }
    children.push(PlacedBox {
        x_offset: (width - op_width) / 2.0,
        baseline_shift: 0.0,
        math_box: op_box,
    });
    if let Some(lower_box) = lower_box {
        let shift = op_depth + params.big_op_spacing2 + lower_box.height;
        depth = depth.max(shift + lower_box.depth);
        let x = (width - lower_box.width) / 2.0;
        children.push(PlacedBox {
            x_offset: x,
            baseline_shift: shift,
            math_box: lower_box,
        });
    }

    MathBox {
        width,
        height,
        depth,
        content: MathBoxContent::Hlist(children),
    }
}

/// Corner-script placement (text style, or `\int`/`\oint` even in
/// display style) -- the same Rule-18 shift/clearance math
/// `layout_script` uses, applied to the operator glyph as the base
/// instead of a general `MathNode`.
fn layout_bigop_corner_scripts(
    op_box: MathBox,
    lower: Option<&MathNode>,
    upper: Option<&MathNode>,
    style: MathStyle,
    font_size: f32,
    params: &FontParams,
    canvas: &mut Canvas,
) -> MathBox {
    let sup_box = upper.map(|u| layout_math(u, style.sup_style(), font_size, canvas));
    let sub_box = lower.map(|l| layout_math(l, style.sub_style(), font_size, canvas));

    let mut sup_shift = 0.0f32;
    if let Some(sup_box) = &sup_box {
        let base_shift = if style.is_cramped() {
            params.sup3
        } else if style.is_display() {
            params.sup1
        } else {
            params.sup2
        };
        sup_shift = base_shift.max(op_box.height - params.sup_drop);
    }
    let mut sub_shift = 0.0f32;
    if let Some(sub_box) = &sub_box {
        let base_shift = if style.is_display() {
            params.sub1
        } else {
            params.sub2
        };
        sub_shift = base_shift
            .max(op_box.depth + params.sub_drop)
            .max(sub_box.height);
    }

    let script_width = match (&sup_box, &sub_box) {
        (Some(s), Some(b)) => s.width.max(b.width),
        (Some(s), None) => s.width,
        (None, Some(b)) => b.width,
        (None, None) => 0.0,
    };

    let mut height = op_box.height;
    let mut depth = op_box.depth;
    let op_width = op_box.width;
    let mut children = vec![PlacedBox {
        x_offset: 0.0,
        baseline_shift: 0.0,
        math_box: op_box,
    }];

    if let Some(sup_box) = sup_box {
        height = height.max(sup_shift + sup_box.height);
        children.push(PlacedBox {
            x_offset: op_width,
            baseline_shift: -sup_shift,
            math_box: sup_box,
        });
    }
    if let Some(sub_box) = sub_box {
        depth = depth.max(sub_shift + sub_box.depth);
        children.push(PlacedBox {
            x_offset: op_width,
            baseline_shift: sub_shift,
            math_box: sub_box,
        });
    }

    MathBox {
        width: op_width + script_width,
        height,
        depth,
        content: MathBoxContent::Hlist(children),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::support::math::ast::AtomClass;

    fn sym(glyph: char) -> MathNode {
        MathNode::Symbol {
            glyph,
            class: AtomClass::Ord,
        }
    }

    #[test]
    fn a_single_symbol_has_positive_width_and_uses_real_font_metrics() {
        let mut canvas = Canvas::new(200, 200).unwrap();
        let math_box = layout_math(&sym('x'), MathStyle::Text, 20.0, &mut canvas);
        assert!(math_box.width > 0.0);
        assert!(
            math_box.height > 0.0,
            "expected a positive ascent from real font metrics"
        );
    }

    #[test]
    fn a_row_concatenates_children_left_to_right_with_gaps() {
        let mut canvas = Canvas::new(200, 200).unwrap();
        let row = MathNode::Row(vec![sym('x'), sym('y')]);
        let math_box = layout_math(&row, MathStyle::Text, 20.0, &mut canvas);
        let MathBoxContent::Hlist(children) = &math_box.content else {
            panic!("expected Hlist")
        };
        assert_eq!(children.len(), 2);
        assert!(
            children[1].x_offset > children[0].math_box.width,
            "second child should start after the first plus a gap"
        );
        assert!(
            (math_box.width - (children[1].x_offset + children[1].math_box.width)).abs() < 1e-3
        );
    }

    #[test]
    fn superscript_is_raised_above_the_baseline() {
        let mut canvas = Canvas::new(200, 200).unwrap();
        let node = MathNode::Script {
            base: Box::new(sym('x')),
            sup: Some(Box::new(sym('2'))),
            sub: None,
        };
        let math_box = layout_math(&node, MathStyle::Text, 20.0, &mut canvas);
        let MathBoxContent::Hlist(children) = &math_box.content else {
            panic!("expected Hlist")
        };
        assert_eq!(children.len(), 2);
        assert!(
            children[1].baseline_shift < 0.0,
            "superscript should be raised (negative baseline_shift), got {}",
            children[1].baseline_shift
        );
    }

    #[test]
    fn subscript_is_lowered_below_the_baseline() {
        let mut canvas = Canvas::new(200, 200).unwrap();
        let node = MathNode::Script {
            base: Box::new(sym('x')),
            sup: None,
            sub: Some(Box::new(sym('i'))),
        };
        let math_box = layout_math(&node, MathStyle::Text, 20.0, &mut canvas);
        let MathBoxContent::Hlist(children) = &math_box.content else {
            panic!("expected Hlist")
        };
        assert_eq!(children.len(), 2);
        assert!(
            children[1].baseline_shift > 0.0,
            "subscript should be lowered (positive baseline_shift), got {}",
            children[1].baseline_shift
        );
    }

    #[test]
    fn both_scripts_at_once_are_placed_at_the_same_x_offset() {
        let mut canvas = Canvas::new(200, 200).unwrap();
        let node = MathNode::Script {
            base: Box::new(sym('x')),
            sup: Some(Box::new(sym('2'))),
            sub: Some(Box::new(sym('i'))),
        };
        let math_box = layout_math(&node, MathStyle::Text, 20.0, &mut canvas);
        let MathBoxContent::Hlist(children) = &math_box.content else {
            panic!("expected Hlist")
        };
        assert_eq!(children.len(), 3, "expected [base, sup, sub]");
        assert_eq!(
            children[1].x_offset, children[2].x_offset,
            "sup and sub should stack at the same x offset"
        );
        assert!(children[1].baseline_shift < 0.0);
        assert!(children[2].baseline_shift > 0.0);
    }

    #[test]
    fn both_scripts_maintain_a_minimum_gap_between_them() {
        let mut canvas = Canvas::new(200, 200).unwrap();
        let node = MathNode::Script {
            base: Box::new(sym('x')),
            sup: Some(Box::new(sym('2'))),
            sub: Some(Box::new(sym('i'))),
        };
        let math_box = layout_math(&node, MathStyle::Text, 20.0, &mut canvas);
        let MathBoxContent::Hlist(children) = &math_box.content else {
            panic!("expected Hlist")
        };
        let sup = &children[1];
        let sub = &children[2];
        let params = FontParams::for_size(20.0);
        let gap =
            (-sup.baseline_shift - sup.math_box.depth) - (sub.math_box.height - sub.baseline_shift);
        assert!(
            gap >= 4.0 * params.default_rule_thickness - 1e-3,
            "expected at least the Rule-18 minimum gap between sup and sub, got {gap}"
        );
    }

    #[test]
    fn nested_scripts_use_a_smaller_style_and_shrink() {
        let mut canvas = Canvas::new(200, 200).unwrap();
        let plain = layout_math(&sym('x'), MathStyle::Text, 20.0, &mut canvas);
        let node = MathNode::Script {
            base: Box::new(sym('x')),
            sup: Some(Box::new(sym('2'))),
            sub: None,
        };
        let scripted = layout_math(&node, MathStyle::Text, 20.0, &mut canvas);
        let MathBoxContent::Hlist(children) = &scripted.content else {
            panic!("expected Hlist")
        };
        assert!(
            children[1].math_box.width < plain.width,
            "the superscript itself should be smaller than a full-size symbol"
        );
    }

    #[test]
    fn fraction_bar_sits_at_axis_height() {
        let mut canvas = Canvas::new(200, 200).unwrap();
        let node = MathNode::Frac {
            num: Box::new(sym('1')),
            den: Box::new(sym('2')),
        };
        let math_box = layout_math(&node, MathStyle::Display, 20.0, &mut canvas);
        let MathBoxContent::Hlist(children) = &math_box.content else {
            panic!("expected Hlist")
        };
        let bar = &children[1];
        assert!(matches!(
            bar.math_box.content,
            MathBoxContent::Rule(RuleKind::HorizontalBar { .. })
        ));
        let params = FontParams::for_size(20.0);
        assert!(
            (-bar.baseline_shift - params.axis_height).abs() < 1e-4,
            "expected the bar's baseline_shift to be exactly -axis_height, got {}",
            bar.baseline_shift
        );
    }

    #[test]
    fn fraction_enforces_minimum_display_style_gap() {
        let mut canvas = Canvas::new(200, 200).unwrap();
        // A row deep enough that the *ideal* (unadjusted) shifts alone
        // wouldn't leave the required 3x-rule-thickness display-style gap.
        let tall = MathNode::Script {
            base: Box::new(sym('x')),
            sup: Some(Box::new(sym('2'))),
            sub: Some(Box::new(sym('2'))),
        };
        let node = MathNode::Frac {
            num: Box::new(tall.clone()),
            den: Box::new(tall),
        };
        let math_box = layout_math(&node, MathStyle::Display, 20.0, &mut canvas);
        let MathBoxContent::Hlist(children) = &math_box.content else {
            panic!("expected Hlist")
        };
        let (num, bar, den) = (&children[0], &children[1], &children[2]);
        let thickness = FontParams::for_size(20.0).default_rule_thickness;
        let num_bottom = -num.baseline_shift - num.math_box.depth;
        let bar_top = -bar.baseline_shift + thickness / 2.0;
        let den_top = den.baseline_shift - den.math_box.height;
        let bar_bottom = -bar.baseline_shift - thickness / 2.0;
        assert!(
            num_bottom >= bar_top - 1e-3,
            "numerator should clear the bar, got num_bottom={num_bottom} bar_top={bar_top}"
        );
        assert!(
            den_top <= bar_bottom + 1e-3,
            "denominator should clear the bar, got den_top={den_top} bar_bottom={bar_bottom}"
        );
    }

    #[test]
    fn fraction_width_is_the_wider_of_numerator_and_denominator() {
        let mut canvas = Canvas::new(200, 200).unwrap();
        let node = MathNode::Frac {
            num: Box::new(sym('1')),
            den: Box::new(MathNode::Row(vec![sym('1'), sym('0'), sym('0')])),
        };
        let math_box = layout_math(&node, MathStyle::Text, 20.0, &mut canvas);
        let MathBoxContent::Hlist(children) = &math_box.content else {
            panic!("expected Hlist")
        };
        assert!(
            (math_box.width - children[2].math_box.width).abs() < 1e-3,
            "fraction width should match the wider denominator"
        );
    }

    #[test]
    fn radical_top_bar_clears_the_radicand_by_the_rule_11_gap() {
        let mut canvas = Canvas::new(200, 200).unwrap();
        let node = MathNode::Sqrt(Box::new(sym('x')));
        let math_box = layout_math(&node, MathStyle::Text, 20.0, &mut canvas);
        let MathBoxContent::Hlist(children) = &math_box.content else {
            panic!("expected Hlist")
        };
        let (sign, radicand) = (&children[0], &children[1]);
        let MathBoxContent::Rule(RuleKind::Radical {
            total_height,
            thickness,
            ..
        }) = &sign.math_box.content
        else {
            panic!("expected a Radical rule, got {:?}", sign.math_box.content)
        };
        let params = FontParams::for_size(20.0);
        let clearance = *total_height - radicand.math_box.height - thickness;
        let expected_clearance = params.default_rule_thickness + params.axis_height.abs() / 4.0;
        assert!(
            (clearance - expected_clearance).abs() < 1e-3,
            "expected the Rule-11 clearance gap, got {clearance}"
        );
    }

    #[test]
    fn radical_radicand_keeps_the_expressions_own_baseline() {
        let mut canvas = Canvas::new(200, 200).unwrap();
        let node = MathNode::Sqrt(Box::new(sym('x')));
        let math_box = layout_math(&node, MathStyle::Text, 20.0, &mut canvas);
        let MathBoxContent::Hlist(children) = &math_box.content else {
            panic!("expected Hlist")
        };
        assert_eq!(
            children[1].baseline_shift, 0.0,
            "the radicand should sit at the expression's own baseline, not be shifted"
        );
    }

    #[test]
    fn delimiters_grow_to_at_least_the_bodys_full_extent() {
        let mut canvas = Canvas::new(200, 200).unwrap();
        let tall_body = MathNode::Script {
            base: Box::new(sym('x')),
            sup: Some(Box::new(sym('2'))),
            sub: Some(Box::new(sym('2'))),
        };
        let node = MathNode::Delimited {
            open: Some(DelimiterKind::Paren),
            body: Box::new(tall_body),
            close: Some(DelimiterKind::Paren),
        };
        let math_box = layout_math(&node, MathStyle::Text, 20.0, &mut canvas);
        let MathBoxContent::Hlist(children) = &math_box.content else {
            panic!("expected Hlist")
        };
        assert_eq!(children.len(), 3, "expected [open, body, close]");
        let (open_delim, body, close_delim) = (&children[0], &children[1], &children[2]);
        assert!(matches!(
            open_delim.math_box.content,
            MathBoxContent::Rule(RuleKind::Delimiter {
                kind: DelimiterKind::Paren,
                is_open: _,
                ..
            })
        ));
        assert!(matches!(
            close_delim.math_box.content,
            MathBoxContent::Rule(RuleKind::Delimiter {
                kind: DelimiterKind::Paren,
                is_open: _,
                ..
            })
        ));
        assert!(
            open_delim.math_box.height + open_delim.math_box.depth
                >= body.math_box.height + body.math_box.depth,
            "delimiter should be at least as tall as the body it encloses"
        );
        assert!(
            close_delim.x_offset > body.x_offset,
            "close delimiter should be placed after the body"
        );
    }

    #[test]
    fn a_missing_side_produces_no_delimiter_box() {
        let mut canvas = Canvas::new(200, 200).unwrap();
        let node = MathNode::Delimited {
            open: None,
            body: Box::new(sym('x')),
            close: Some(DelimiterKind::Bracket),
        };
        let math_box = layout_math(&node, MathStyle::Text, 20.0, &mut canvas);
        let MathBoxContent::Hlist(children) = &math_box.content else {
            panic!("expected Hlist")
        };
        assert_eq!(
            children.len(),
            2,
            "expected [body, close] only, no open delimiter"
        );
    }

    #[test]
    fn sum_stacks_limits_in_display_style() {
        let mut canvas = Canvas::new(200, 200).unwrap();
        let node = MathNode::BigOp {
            kind: BigOpKind::Sum,
            lower: Some(Box::new(sym('1'))),
            upper: Some(Box::new(sym('n'))),
        };
        let math_box = layout_math(&node, MathStyle::Display, 20.0, &mut canvas);
        let MathBoxContent::Hlist(children) = &math_box.content else {
            panic!("expected Hlist")
        };
        assert_eq!(children.len(), 3, "expected [upper, op, lower] stacked");
        // The upper limit should be directly above the operator (roughly
        // centered, not off to the side), and the lower limit directly
        // below -- i.e. all three roughly share an x-extent, unlike
        // corner scripts which sit entirely to the operator's right.
        assert!(
            children[0].baseline_shift < 0.0,
            "upper limit should be raised above the operator"
        );
        assert!(
            children[2].baseline_shift > 0.0,
            "lower limit should be lowered below the operator"
        );
    }

    #[test]
    fn int_keeps_corner_scripts_even_in_display_style() {
        let mut canvas = Canvas::new(200, 200).unwrap();
        let node = MathNode::BigOp {
            kind: BigOpKind::Int,
            lower: Some(Box::new(sym('0'))),
            upper: Some(Box::new(sym('1'))),
        };
        let math_box = layout_math(&node, MathStyle::Display, 20.0, &mut canvas);
        let MathBoxContent::Hlist(children) = &math_box.content else {
            panic!("expected Hlist")
        };
        assert_eq!(
            children.len(),
            3,
            "expected [op, sup, sub] as corner scripts"
        );
        // Corner scripts both sit at the *same* x_offset, to the right of
        // the operator -- unlike the stacked case, they don't span the
        // operator's own width.
        assert_eq!(children[1].x_offset, children[2].x_offset);
        assert!(
            children[1].x_offset > 0.0,
            "corner scripts should be offset to the right of the operator glyph"
        );
    }

    #[test]
    fn sum_in_text_style_falls_back_to_corner_scripts() {
        let mut canvas = Canvas::new(200, 200).unwrap();
        let node = MathNode::BigOp {
            kind: BigOpKind::Sum,
            lower: Some(Box::new(sym('1'))),
            upper: Some(Box::new(sym('n'))),
        };
        let math_box = layout_math(&node, MathStyle::Text, 20.0, &mut canvas);
        let MathBoxContent::Hlist(children) = &math_box.content else {
            panic!("expected Hlist")
        };
        assert_eq!(
            children[1].x_offset, children[2].x_offset,
            "text-style limits should be corner scripts, not stacked"
        );
    }
}
