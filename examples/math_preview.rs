//! Throwaway visual sanity check for `support::math` (see the plan at
//! `.claude/plans/glimmering-drifting-possum.md`). Renders a fixed set
//! of representative expressions to PNG for manual inspection after each
//! phase. Deleted once math rendering is fully integrated and verified
//! live in the real chat UI (Phase 9).

use mkgraphic::support::canvas::Canvas;
use mkgraphic::support::color::Color;
use mkgraphic::support::math::{draw, layout, parser, style::MathStyle};
use mkgraphic::support::point::Point;

fn render(source: &str, out_path: &str) {
    let node = parser::parse_math(source).expect("should parse");
    let mut canvas = Canvas::new(400, 150).unwrap();
    canvas.clear(Color::new(1.0, 1.0, 1.0, 1.0));
    let math_box = layout::layout_math(&node, MathStyle::Display, 24.0, &mut canvas);
    let origin = Point::new(20.0, 75.0);
    draw::draw_math_box(
        &mut canvas,
        &math_box,
        origin,
        Color::new(0.0, 0.0, 0.0, 1.0),
    );
    canvas.pixmap().save_png(out_path).expect("should save png");
    println!(
        "wrote {out_path} (box: {}x(+{}/-{}))",
        math_box.width, math_box.height, math_box.depth
    );
}

fn main() {
    let dir = "/private/tmp/claude-501/-Users-minjaekim-Plugins-mkapk/c30b5ae9-d0be-4d62-84ce-8203079e191c/scratchpad";
    render("x^2 + y_i", &format!("{dir}/math_preview_phase2.png"));
    render("\\frac{1}{2}", &format!("{dir}/math_preview_phase3.png"));
    render("\\sqrt{x+1}", &format!("{dir}/math_preview_phase4.png"));
    render(
        "\\left(\\frac{1}{2}\\right)",
        &format!("{dir}/math_preview_phase5.png"),
    );
    render(
        "\\sum_{i=1}^{n} i",
        &format!("{dir}/math_preview_phase6_sum.png"),
    );
    render(
        "\\int_0^1 x^2",
        &format!("{dir}/math_preview_phase6_int.png"),
    );
}
