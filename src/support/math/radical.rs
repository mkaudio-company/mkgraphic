//! The radical (√) sign for `\sqrt{}` -- drawn as a single *stroked*
//! vector path (a check-mark hook plus a horizontal vinculum), computed
//! proportionally for whatever height is actually needed.
//!
//! The plan this was built from anticipated picking from a small table
//! of discrete pre-drawn sizes -- the usual workaround when a radical
//! sign comes from a font's fixed-size glyph variants. Since this one is
//! a hand-drawn vector path rather than a font glyph, computing the hook
//! proportionally for the *exact* needed height is simpler and avoids
//! discrete-size seams entirely, so that's what this does instead.

use crate::support::canvas::Canvas;
use crate::support::color::Color;
use crate::support::point::Point;

/// How far the hook extends to the left of the vinculum -- callers need
/// this to know where the radicand box itself should start.
pub fn tick_width(total_height: f32) -> f32 {
    total_height * 0.32
}

/// Draws the radical sign: `origin` is the point at the main baseline
/// where the hook's tail begins; the hook rises `total_height` above
/// that (screen Y decreasing = up), and the vinculum extends
/// `radicand_width` to the right from the hook's peak.
pub fn draw_radical(
    canvas: &mut Canvas,
    origin: Point,
    total_height: f32,
    radicand_width: f32,
    thickness: f32,
    color: Color,
) {
    let tail = Point::new(origin.x, origin.y - total_height * 0.45);
    let dip = Point::new(
        origin.x + total_height * 0.12,
        origin.y - total_height * 0.25,
    );
    let hook_ctrl = Point::new(
        origin.x + total_height * 0.18,
        origin.y - total_height * 0.05,
    );
    let peak = Point::new(origin.x + tick_width(total_height), origin.y - total_height);
    let bar_end = Point::new(peak.x + radicand_width, peak.y);

    canvas.begin_path();
    canvas.move_to(tail);
    canvas.line_to(dip);
    canvas.quad_to(hook_ctrl, peak);
    canvas.line_to(bar_end);
    canvas.stroke_style(color);
    canvas.line_width(thickness);
    canvas.stroke();
}
