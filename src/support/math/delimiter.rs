//! Growing `\left`/`\right` delimiters. No extensible font glyph variants
//! are available (this crate has no OTF MATH table), so -- per the same
//! reasoning as `radical.rs` -- each delimiter is a hand-drawn vector
//! path with a fixed-size cap/foot and a straight middle segment that
//! stretches to whatever height is actually needed.

use crate::support::canvas::Canvas;
use crate::support::color::Color;
use crate::support::point::Point;

use super::ast::DelimiterKind;

/// Draws `kind` inside the box `[origin.x, origin.y] .. [origin.x +
/// width, origin.y + height]` (top-left origin, screen Y down), stroked
/// at `thickness`. `is_open` mirrors every shape below horizontally
/// (just by swapping which of `left`/`right` is numerically which,
/// since every shape is defined purely in terms of those two plus
/// `top`/`bottom`/`mid`) -- except `AngleLeft`/`AngleRight`, `Bar`, and
/// `DoubleBar`, which are either already direction-specific via their own
/// `DelimiterKind` or symmetric, so mirroring them would be a no-op or
/// wrong.
pub fn draw_delimiter(
    canvas: &mut Canvas,
    kind: DelimiterKind,
    is_open: bool,
    origin: Point,
    width: f32,
    height: f32,
    thickness: f32,
    color: Color,
) {
    canvas.stroke_style(color);
    canvas.line_width(thickness);
    let top = origin.y;
    let bottom = origin.y + height;
    let mirror = !is_open
        && !matches!(
            kind,
            DelimiterKind::AngleLeft
                | DelimiterKind::AngleRight
                | DelimiterKind::Bar
                | DelimiterKind::DoubleBar
        );
    let (left, right) = if mirror {
        (origin.x + width, origin.x)
    } else {
        (origin.x, origin.x + width)
    };
    let cap = (height * 0.18).min(width * 2.0).max(width * 0.5);

    canvas.begin_path();
    match kind {
        DelimiterKind::Paren => {
            // A single bowed curve from top to bottom, bulging toward
            // `right` at its middle -- `Paren` covers both `(` and `)`;
            // which literal glyph it represents is just which side
            // (open/close) it was placed on, not a separate shape here.
            canvas.move_to(Point::new(right, top));
            canvas.cubic_to(
                Point::new(left, top + cap),
                Point::new(left, bottom - cap),
                Point::new(right, bottom),
            );
        }
        DelimiterKind::Bracket => {
            canvas.move_to(Point::new(right, top));
            canvas.line_to(Point::new(left, top));
            canvas.line_to(Point::new(left, bottom));
            canvas.line_to(Point::new(right, bottom));
        }
        DelimiterKind::Brace => {
            let mid = (top + bottom) / 2.0;
            canvas.move_to(Point::new(right, top));
            canvas.quad_to(Point::new(left, top), Point::new(left, top + cap));
            canvas.line_to(Point::new(left, mid - cap * 0.5));
            canvas.quad_to(
                Point::new(left - cap * 0.5, mid),
                Point::new(left, mid + cap * 0.5),
            );
            canvas.line_to(Point::new(left, bottom - cap));
            canvas.quad_to(Point::new(left, bottom), Point::new(right, bottom));
        }
        DelimiterKind::Floor => {
            canvas.move_to(Point::new(left, top));
            canvas.line_to(Point::new(left, bottom));
            canvas.line_to(Point::new(right, bottom));
        }
        DelimiterKind::Ceil => {
            canvas.move_to(Point::new(left, bottom));
            canvas.line_to(Point::new(left, top));
            canvas.line_to(Point::new(right, top));
        }
        DelimiterKind::Bar => {
            let x = (left + right) / 2.0;
            canvas.move_to(Point::new(x, top));
            canvas.line_to(Point::new(x, bottom));
        }
        DelimiterKind::DoubleBar => {
            let gap = width * 0.3;
            let x1 = (left + right) / 2.0 - gap / 2.0;
            let x2 = (left + right) / 2.0 + gap / 2.0;
            canvas.move_to(Point::new(x1, top));
            canvas.line_to(Point::new(x1, bottom));
            canvas.move_to(Point::new(x2, top));
            canvas.line_to(Point::new(x2, bottom));
        }
        DelimiterKind::AngleLeft => {
            let mid = (top + bottom) / 2.0;
            canvas.move_to(Point::new(right, top));
            canvas.line_to(Point::new(left, mid));
            canvas.line_to(Point::new(right, bottom));
        }
        DelimiterKind::AngleRight => {
            let mid = (top + bottom) / 2.0;
            canvas.move_to(Point::new(left, top));
            canvas.line_to(Point::new(right, mid));
            canvas.line_to(Point::new(left, bottom));
        }
    }
    canvas.stroke();
}

/// A reasonable fixed width for any delimiter at a given target height --
/// real TeX's extensible delimiters have per-glyph natural widths; a
/// single proportional estimate is close enough for a hand-drawn path.
pub fn delimiter_width(target_height: f32) -> f32 {
    (target_height * 0.18).max(4.0)
}
