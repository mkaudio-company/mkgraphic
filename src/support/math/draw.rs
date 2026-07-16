//! Paints an already-laid-out [`MathBox`] at a given origin. Purely a
//! tree walk -- all the actual positioning decisions were already made by
//! [`super::layout::layout_math`]; this just converts each box's
//! `baseline_shift`/`x_offset` into absolute `Canvas` coordinates and
//! dispatches to `fill_text` (for [`MathBoxContent::Glyphs`]) or simple
//! path fills (for [`MathBoxContent::Rule`]).

use crate::support::canvas::Canvas;
use crate::support::color::Color;
use crate::support::point::Point;

use super::layout::{MathBox, MathBoxContent, RuleKind};

/// Draws `math_box` with its own baseline at `origin` (matching
/// `draw_runs`'s existing baseline-cursor convention in
/// `support::markdown`).
pub fn draw_math_box(canvas: &mut Canvas, math_box: &MathBox, origin: Point, color: Color) {
    match &math_box.content {
        MathBoxContent::Glyphs {
            text,
            font,
            font_size,
        } => {
            canvas.font(font.clone());
            canvas.font_size(*font_size);
            canvas.fill_style(color);
            canvas.fill_text(text, origin);
        }
        MathBoxContent::Hlist(children) => {
            for child in children {
                let child_origin =
                    Point::new(origin.x + child.x_offset, origin.y + child.baseline_shift);
                draw_math_box(canvas, &child.math_box, child_origin, color);
            }
        }
        MathBoxContent::Rule(rule) => draw_rule(canvas, rule, math_box, origin, color),
    }
}

fn draw_rule(
    canvas: &mut Canvas,
    rule: &RuleKind,
    math_box: &MathBox,
    origin: Point,
    color: Color,
) {
    match rule {
        RuleKind::HorizontalBar { width, thickness } => {
            canvas.fill_style(color);
            canvas.fill_rect(crate::support::rect::Rect::new(
                origin.x,
                origin.y - thickness / 2.0,
                origin.x + width,
                origin.y + thickness / 2.0,
            ));
        }
        RuleKind::Radical {
            total_height,
            radicand_width,
            thickness,
        } => {
            super::radical::draw_radical(
                canvas,
                origin,
                *total_height,
                *radicand_width,
                *thickness,
                color,
            );
        }
        RuleKind::Delimiter {
            kind,
            thickness,
            is_open,
        } => {
            let top_left = Point::new(origin.x, origin.y - math_box.height);
            super::delimiter::draw_delimiter(
                canvas,
                *kind,
                *is_open,
                super::delimiter::DelimiterGeometry {
                    origin: top_left,
                    width: math_box.width,
                    height: math_box.height + math_box.depth,
                    thickness: *thickness,
                    color,
                },
            );
        }
    }
}
