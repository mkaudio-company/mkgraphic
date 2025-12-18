//! Canvas abstraction for 2D drawing.
//!
//! This module provides a high-level drawing API that wraps the underlying
//! graphics backend (tiny-skia).

use std::sync::OnceLock;

use super::color::Color;
use super::point::Point;
use super::rect::Rect;
use super::circle::Circle;
use super::font::{Font, FontDatabase};

/// Text alignment options.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct TextAlign {
    pub horizontal: HorizontalAlign,
    pub vertical: VerticalAlign,
}

/// Horizontal text alignment.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum HorizontalAlign {
    #[default]
    Left,
    Center,
    Right,
}

/// Vertical text alignment.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum VerticalAlign {
    Top,
    #[default]
    Baseline,
    Middle,
    Bottom,
}

/// Fill rule for complex paths.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum FillRule {
    #[default]
    NonZero,
    EvenOdd,
}

/// Line cap style.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum LineCap {
    #[default]
    Butt,
    Round,
    Square,
}

/// Line join style.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum LineJoin {
    #[default]
    Miter,
    Round,
    Bevel,
}

/// A color stop for gradients.
#[derive(Debug, Clone, Copy)]
pub struct ColorStop {
    pub offset: f32,
    pub color: Color,
}

/// A linear gradient.
#[derive(Debug, Clone)]
pub struct LinearGradient {
    pub start: Point,
    pub end: Point,
    pub stops: Vec<ColorStop>,
}

impl LinearGradient {
    pub fn new(start: Point, end: Point) -> Self {
        Self {
            start,
            end,
            stops: Vec::new(),
        }
    }

    pub fn add_stop(&mut self, offset: f32, color: Color) {
        self.stops.push(ColorStop { offset, color });
    }
}

/// A radial gradient.
#[derive(Debug, Clone)]
pub struct RadialGradient {
    pub center1: Point,
    pub radius1: f32,
    pub center2: Point,
    pub radius2: f32,
    pub stops: Vec<ColorStop>,
}

impl RadialGradient {
    pub fn new(center: Point, inner_radius: f32, outer_radius: f32) -> Self {
        Self {
            center1: center,
            radius1: inner_radius,
            center2: center,
            radius2: outer_radius,
            stops: Vec::new(),
        }
    }

    pub fn add_stop(&mut self, offset: f32, color: Color) {
        self.stops.push(ColorStop { offset, color });
    }
}

/// Text metrics returned from measuring text.
#[derive(Debug, Clone, Copy, Default)]
pub struct TextMetrics {
    pub ascent: f32,
    pub descent: f32,
    pub leading: f32,
    pub width: f32,
    pub height: f32,
}

/// Font metrics.
#[derive(Debug, Clone, Copy, Default)]
pub struct FontMetrics {
    pub ascent: f32,
    pub descent: f32,
    pub height: f32,
    pub leading: f32,
}

/// Corner radii for rounded rectangles.
#[derive(Debug, Clone, Copy, Default)]
pub struct CornerRadii {
    pub top_left: f32,
    pub top_right: f32,
    pub bottom_right: f32,
    pub bottom_left: f32,
}

impl CornerRadii {
    pub const fn new(radius: f32) -> Self {
        Self {
            top_left: radius,
            top_right: radius,
            bottom_right: radius,
            bottom_left: radius,
        }
    }

    pub const fn with_values(top_left: f32, top_right: f32, bottom_right: f32, bottom_left: f32) -> Self {
        Self {
            top_left,
            top_right,
            bottom_right,
            bottom_left,
        }
    }
}

/// The canvas provides 2D drawing operations.
///
/// This is a wrapper around the underlying graphics backend (tiny-skia)
/// providing a similar API to the Cairo-based C++ version.
pub struct Canvas {
    pixmap: tiny_skia::Pixmap,
    path_builder: Option<tiny_skia::PathBuilder>,
    fill_color: Color,
    stroke_color: Color,
    line_width: f32,
    text_align: TextAlign,
    transform: tiny_skia::Transform,
    save_stack: Vec<CanvasState>,
    current_font: Option<Font>,
    font_size: f32,
}

struct CanvasState {
    fill_color: Color,
    stroke_color: Color,
    line_width: f32,
    text_align: TextAlign,
    transform: tiny_skia::Transform,
    font_size: f32,
}

impl Canvas {
    /// Creates a new canvas with the given dimensions.
    pub fn new(width: u32, height: u32) -> Option<Self> {
        let pixmap = tiny_skia::Pixmap::new(width, height)?;
        Some(Self {
            pixmap,
            path_builder: None,
            fill_color: Color::new(0.0, 0.0, 0.0, 1.0),
            stroke_color: Color::new(0.0, 0.0, 0.0, 1.0),
            line_width: 1.0,
            text_align: TextAlign::default(),
            transform: tiny_skia::Transform::identity(),
            save_stack: Vec::new(),
            current_font: None,
            font_size: 12.0,
        })
    }

    /// Creates a canvas from an existing pixmap.
    pub fn from_pixmap(pixmap: tiny_skia::Pixmap) -> Self {
        Self {
            pixmap,
            path_builder: None,
            fill_color: Color::new(0.0, 0.0, 0.0, 1.0),
            stroke_color: Color::new(0.0, 0.0, 0.0, 1.0),
            line_width: 1.0,
            text_align: TextAlign::default(),
            transform: tiny_skia::Transform::identity(),
            save_stack: Vec::new(),
            current_font: None,
            font_size: 12.0,
        }
    }

    /// Returns the width of the canvas.
    pub fn width(&self) -> u32 {
        self.pixmap.width()
    }

    /// Returns the height of the canvas.
    pub fn height(&self) -> u32 {
        self.pixmap.height()
    }

    /// Returns the underlying pixmap.
    pub fn pixmap(&self) -> &tiny_skia::Pixmap {
        &self.pixmap
    }

    /// Returns a mutable reference to the underlying pixmap.
    pub fn pixmap_mut(&mut self) -> &mut tiny_skia::Pixmap {
        &mut self.pixmap
    }

    // --- Transforms ---

    /// Translates the canvas.
    pub fn translate(&mut self, p: Point) {
        self.transform = self.transform.pre_translate(p.x, p.y);
    }

    /// Rotates the canvas by the given angle in radians.
    pub fn rotate(&mut self, radians: f32) {
        self.transform = self.transform.pre_rotate(radians.to_degrees());
    }

    /// Scales the canvas.
    pub fn scale(&mut self, sx: f32, sy: f32) {
        self.transform = self.transform.pre_scale(sx, sy);
    }

    // --- Paths ---

    /// Begins a new path.
    pub fn begin_path(&mut self) {
        self.path_builder = Some(tiny_skia::PathBuilder::new());
    }

    /// Closes the current path.
    pub fn close_path(&mut self) {
        if let Some(ref mut pb) = self.path_builder {
            pb.close();
        }
    }

    /// Moves to a point.
    pub fn move_to(&mut self, p: Point) {
        if let Some(ref mut pb) = self.path_builder {
            pb.move_to(p.x, p.y);
        }
    }

    /// Draws a line to a point.
    pub fn line_to(&mut self, p: Point) {
        if let Some(ref mut pb) = self.path_builder {
            pb.line_to(p.x, p.y);
        }
    }

    /// Draws an arc.
    pub fn arc(&mut self, center: Point, radius: f32, start_angle: f32, end_angle: f32, ccw: bool) {
        if let Some(ref mut pb) = self.path_builder {
            // Convert angles to degrees and use arc_to approximation
            let sweep = if ccw {
                start_angle - end_angle
            } else {
                end_angle - start_angle
            };

            // For simplicity, approximate with bezier curves
            // This is a simplified implementation
            let start_x = center.x + radius * start_angle.cos();
            let start_y = center.y + radius * start_angle.sin();
            let end_x = center.x + radius * end_angle.cos();
            let end_y = center.y + radius * end_angle.sin();

            pb.line_to(start_x, start_y);
            // Use quadratic bezier approximation for the arc
            let mid_angle = (start_angle + end_angle) / 2.0;
            let ctrl_x = center.x + radius * 1.3 * mid_angle.cos();
            let ctrl_y = center.y + radius * 1.3 * mid_angle.sin();
            pb.quad_to(ctrl_x, ctrl_y, end_x, end_y);
        }
    }

    /// Adds a rectangle to the path.
    pub fn add_rect(&mut self, r: Rect) {
        if let Some(ref mut pb) = self.path_builder {
            pb.push_rect(
                tiny_skia::Rect::from_ltrb(r.left, r.top, r.right, r.bottom)
                    .unwrap_or(tiny_skia::Rect::from_xywh(0.0, 0.0, 1.0, 1.0).unwrap()),
            );
        }
    }

    /// Adds a rounded rectangle to the path.
    pub fn add_round_rect(&mut self, r: Rect, radius: f32) {
        self.add_round_rect_varying(r, CornerRadii::new(radius));
    }

    /// Adds a rounded rectangle with varying corner radii.
    pub fn add_round_rect_varying(&mut self, r: Rect, radii: CornerRadii) {
        if let Some(ref mut pb) = self.path_builder {
            // Start at top-left, after the corner
            pb.move_to(r.left + radii.top_left, r.top);

            // Top edge and top-right corner
            pb.line_to(r.right - radii.top_right, r.top);
            if radii.top_right > 0.0 {
                pb.quad_to(r.right, r.top, r.right, r.top + radii.top_right);
            }

            // Right edge and bottom-right corner
            pb.line_to(r.right, r.bottom - radii.bottom_right);
            if radii.bottom_right > 0.0 {
                pb.quad_to(r.right, r.bottom, r.right - radii.bottom_right, r.bottom);
            }

            // Bottom edge and bottom-left corner
            pb.line_to(r.left + radii.bottom_left, r.bottom);
            if radii.bottom_left > 0.0 {
                pb.quad_to(r.left, r.bottom, r.left, r.bottom - radii.bottom_left);
            }

            // Left edge and top-left corner
            pb.line_to(r.left, r.top + radii.top_left);
            if radii.top_left > 0.0 {
                pb.quad_to(r.left, r.top, r.left + radii.top_left, r.top);
            }

            pb.close();
        }
    }

    /// Adds a circle to the path.
    pub fn add_circle(&mut self, c: Circle) {
        if let Some(ref mut pb) = self.path_builder {
            pb.push_circle(c.center.x, c.center.y, c.radius);
        }
    }

    // --- Styles ---

    /// Sets the fill color.
    pub fn fill_style(&mut self, color: Color) {
        self.fill_color = color;
    }

    /// Sets the stroke color.
    pub fn stroke_style(&mut self, color: Color) {
        self.stroke_color = color;
    }

    /// Sets the line width.
    pub fn line_width(&mut self, width: f32) {
        self.line_width = width;
    }

    // --- Drawing ---

    fn color_to_paint(color: Color) -> tiny_skia::Paint<'static> {
        let mut paint = tiny_skia::Paint::default();
        paint.set_color(tiny_skia::Color::from_rgba(
            color.red,
            color.green,
            color.blue,
            color.alpha,
        ).unwrap_or(tiny_skia::Color::BLACK));
        paint.anti_alias = true;
        paint
    }

    /// Fills the current path.
    pub fn fill(&mut self) {
        if let Some(pb) = self.path_builder.take() {
            if let Some(path) = pb.finish() {
                let paint = Self::color_to_paint(self.fill_color);
                self.pixmap.fill_path(
                    &path,
                    &paint,
                    tiny_skia::FillRule::Winding,
                    self.transform,
                    None,
                );
            }
        }
    }

    /// Fills the current path and preserves it.
    pub fn fill_preserve(&mut self) {
        if let Some(ref pb) = self.path_builder {
            if let Some(path) = pb.clone().finish() {
                let paint = Self::color_to_paint(self.fill_color);
                self.pixmap.fill_path(
                    &path,
                    &paint,
                    tiny_skia::FillRule::Winding,
                    self.transform,
                    None,
                );
            }
        }
    }

    /// Strokes the current path.
    pub fn stroke(&mut self) {
        if let Some(pb) = self.path_builder.take() {
            if let Some(path) = pb.finish() {
                let paint = Self::color_to_paint(self.stroke_color);
                let stroke = tiny_skia::Stroke {
                    width: self.line_width,
                    ..Default::default()
                };
                self.pixmap.stroke_path(&path, &paint, &stroke, self.transform, None);
            }
        }
    }

    /// Strokes the current path and preserves it.
    pub fn stroke_preserve(&mut self) {
        if let Some(ref pb) = self.path_builder {
            if let Some(path) = pb.clone().finish() {
                let paint = Self::color_to_paint(self.stroke_color);
                let stroke = tiny_skia::Stroke {
                    width: self.line_width,
                    ..Default::default()
                };
                self.pixmap.stroke_path(&path, &paint, &stroke, self.transform, None);
            }
        }
    }

    // --- Rectangle shortcuts ---

    /// Fills a rectangle.
    pub fn fill_rect(&mut self, r: Rect) {
        self.begin_path();
        self.add_rect(r);
        self.fill();
    }

    /// Fills a rounded rectangle.
    pub fn fill_round_rect(&mut self, r: Rect, radius: f32) {
        self.begin_path();
        self.add_round_rect(r, radius);
        self.fill();
    }

    /// Strokes a rectangle.
    pub fn stroke_rect(&mut self, r: Rect) {
        self.begin_path();
        self.add_rect(r);
        self.stroke();
    }

    /// Strokes a rounded rectangle.
    pub fn stroke_round_rect(&mut self, r: Rect, radius: f32) {
        self.begin_path();
        self.add_round_rect(r, radius);
        self.stroke();
    }

    // --- State management ---

    /// Saves the current canvas state.
    pub fn save(&mut self) {
        self.save_stack.push(CanvasState {
            fill_color: self.fill_color,
            stroke_color: self.stroke_color,
            line_width: self.line_width,
            text_align: self.text_align,
            transform: self.transform,
            font_size: self.font_size,
        });
    }

    /// Restores the previously saved canvas state.
    pub fn restore(&mut self) {
        if let Some(state) = self.save_stack.pop() {
            self.fill_color = state.fill_color;
            self.stroke_color = state.stroke_color;
            self.line_width = state.line_width;
            self.text_align = state.text_align;
            self.transform = state.transform;
            self.font_size = state.font_size;
        }
    }

    // --- Font and text ---

    /// Sets the current font.
    pub fn font(&mut self, font: Font) {
        self.current_font = Some(font);
    }

    /// Sets the font size.
    pub fn font_size(&mut self, size: f32) {
        self.font_size = size;
    }

    /// Sets the text alignment.
    pub fn text_align(&mut self, align: TextAlign) {
        self.text_align = align;
    }

    /// Measures text (placeholder - requires text shaping integration).
    pub fn measure_text(&self, _text: &str) -> TextMetrics {
        // TODO: Integrate with rustybuzz for text shaping
        TextMetrics {
            ascent: self.font_size * 0.8,
            descent: self.font_size * 0.2,
            leading: self.font_size * 0.1,
            width: 0.0, // Would need actual text measurement
            height: self.font_size,
        }
    }

    /// Fills text at the given position.
    pub fn fill_text(&mut self, text: &str, p: Point) {
        // Get or initialize the global font database
        static FONT_DB: OnceLock<FontDatabase> = OnceLock::new();
        let font_db = FONT_DB.get_or_init(FontDatabase::with_system_fonts);

        // Find a suitable font
        let query = fontdb::Query {
            families: &[fontdb::Family::SansSerif],
            weight: fontdb::Weight(400),
            stretch: fontdb::Stretch::Normal,
            style: fontdb::Style::Normal,
        };

        let Some(font_id) = font_db.inner().query(&query) else {
            return;
        };

        // Use with_face_data to access the font bytes directly
        let mut rendered = false;
        font_db.inner().with_face_data(font_id, |font_data_ref, face_index| {
            // Parse the font
            let Ok(face) = ttf_parser::Face::parse(font_data_ref, face_index) else {
                return;
            };

            // Create rustybuzz face
            let Some(buzz_face) = rustybuzz::Face::from_slice(font_data_ref, face_index) else {
                return;
            };

            // Shape the text
            let mut buffer = rustybuzz::UnicodeBuffer::new();
            buffer.push_str(text);
            let output = rustybuzz::shape(&buzz_face, &[], buffer);

            // Calculate scale factor
            let units_per_em = face.units_per_em() as f32;
            let scale = self.font_size / units_per_em;

            // Render each glyph
            let mut x_pos = p.x;
            let y_pos = p.y;

            let glyph_infos = output.glyph_infos();
            let glyph_positions = output.glyph_positions();

            for (info, pos) in glyph_infos.iter().zip(glyph_positions.iter()) {
                let glyph_id = ttf_parser::GlyphId(info.glyph_id as u16);

                let glyph_x = x_pos + (pos.x_offset as f32) * scale;
                let glyph_y = y_pos + (pos.y_offset as f32) * scale;

                // Render the glyph using outline
                Self::render_glyph_static(
                    &mut self.pixmap,
                    &face,
                    glyph_id,
                    glyph_x,
                    glyph_y,
                    scale,
                    self.fill_color,
                    self.transform,
                );

                // Advance position
                x_pos += (pos.x_advance as f32) * scale;
            }
            rendered = true;
        });

        // If rendering failed inside the closure, nothing more to do
        let _ = rendered;
    }

    /// Renders a single glyph at the given position.
    fn render_glyph(
        &mut self,
        face: &ttf_parser::Face,
        glyph_id: ttf_parser::GlyphId,
        x: f32,
        y: f32,
        scale: f32,
    ) {
        struct GlyphOutlineBuilder {
            path: tiny_skia::PathBuilder,
            x: f32,
            y: f32,
            scale: f32,
        }

        impl ttf_parser::OutlineBuilder for GlyphOutlineBuilder {
            fn move_to(&mut self, px: f32, py: f32) {
                let tx = self.x + px * self.scale;
                let ty = self.y - py * self.scale; // Flip Y axis
                self.path.move_to(tx, ty);
            }

            fn line_to(&mut self, px: f32, py: f32) {
                let tx = self.x + px * self.scale;
                let ty = self.y - py * self.scale;
                self.path.line_to(tx, ty);
            }

            fn quad_to(&mut self, x1: f32, y1: f32, px: f32, py: f32) {
                let tx1 = self.x + x1 * self.scale;
                let ty1 = self.y - y1 * self.scale;
                let tx = self.x + px * self.scale;
                let ty = self.y - py * self.scale;
                self.path.quad_to(tx1, ty1, tx, ty);
            }

            fn curve_to(&mut self, x1: f32, y1: f32, x2: f32, y2: f32, px: f32, py: f32) {
                let tx1 = self.x + x1 * self.scale;
                let ty1 = self.y - y1 * self.scale;
                let tx2 = self.x + x2 * self.scale;
                let ty2 = self.y - y2 * self.scale;
                let tx = self.x + px * self.scale;
                let ty = self.y - py * self.scale;
                self.path.cubic_to(tx1, ty1, tx2, ty2, tx, ty);
            }

            fn close(&mut self) {
                self.path.close();
            }
        }

        let mut builder = GlyphOutlineBuilder {
            path: tiny_skia::PathBuilder::new(),
            x,
            y,
            scale,
        };

        if face.outline_glyph(glyph_id, &mut builder).is_some() {
            if let Some(path) = builder.path.finish() {
                let paint = Self::color_to_paint(self.fill_color);
                self.pixmap.fill_path(
                    &path,
                    &paint,
                    tiny_skia::FillRule::Winding,
                    self.transform,
                    None,
                );
            }
        }
    }

    /// Renders a single glyph at the given position (static version for use in closures).
    fn render_glyph_static(
        pixmap: &mut tiny_skia::Pixmap,
        face: &ttf_parser::Face,
        glyph_id: ttf_parser::GlyphId,
        x: f32,
        y: f32,
        scale: f32,
        fill_color: Color,
        transform: tiny_skia::Transform,
    ) {
        struct GlyphOutlineBuilder {
            path: tiny_skia::PathBuilder,
            x: f32,
            y: f32,
            scale: f32,
        }

        impl ttf_parser::OutlineBuilder for GlyphOutlineBuilder {
            fn move_to(&mut self, px: f32, py: f32) {
                let tx = self.x + px * self.scale;
                let ty = self.y - py * self.scale; // Flip Y axis
                self.path.move_to(tx, ty);
            }

            fn line_to(&mut self, px: f32, py: f32) {
                let tx = self.x + px * self.scale;
                let ty = self.y - py * self.scale;
                self.path.line_to(tx, ty);
            }

            fn quad_to(&mut self, x1: f32, y1: f32, px: f32, py: f32) {
                let tx1 = self.x + x1 * self.scale;
                let ty1 = self.y - y1 * self.scale;
                let tx = self.x + px * self.scale;
                let ty = self.y - py * self.scale;
                self.path.quad_to(tx1, ty1, tx, ty);
            }

            fn curve_to(&mut self, x1: f32, y1: f32, x2: f32, y2: f32, px: f32, py: f32) {
                let tx1 = self.x + x1 * self.scale;
                let ty1 = self.y - y1 * self.scale;
                let tx2 = self.x + x2 * self.scale;
                let ty2 = self.y - y2 * self.scale;
                let tx = self.x + px * self.scale;
                let ty = self.y - py * self.scale;
                self.path.cubic_to(tx1, ty1, tx2, ty2, tx, ty);
            }

            fn close(&mut self) {
                self.path.close();
            }
        }

        let mut builder = GlyphOutlineBuilder {
            path: tiny_skia::PathBuilder::new(),
            x,
            y,
            scale,
        };

        if face.outline_glyph(glyph_id, &mut builder).is_some() {
            if let Some(path) = builder.path.finish() {
                let paint = Self::color_to_paint(fill_color);
                pixmap.fill_path(
                    &path,
                    &paint,
                    tiny_skia::FillRule::Winding,
                    transform,
                    None,
                );
            }
        }
    }

    /// Clears the canvas with the given color.
    pub fn clear(&mut self, color: Color) {
        self.pixmap.fill(tiny_skia::Color::from_rgba(
            color.red,
            color.green,
            color.blue,
            color.alpha,
        ).unwrap_or(tiny_skia::Color::WHITE));
    }
}

/// A RAII guard that saves canvas state on creation and restores it on drop.
pub struct CanvasStateGuard<'a> {
    canvas: &'a mut Canvas,
}

impl<'a> CanvasStateGuard<'a> {
    pub fn new(canvas: &'a mut Canvas) -> Self {
        canvas.save();
        Self { canvas }
    }
}

impl<'a> Drop for CanvasStateGuard<'a> {
    fn drop(&mut self) {
        self.canvas.restore();
    }
}

impl<'a> std::ops::Deref for CanvasStateGuard<'a> {
    type Target = Canvas;

    fn deref(&self) -> &Self::Target {
        self.canvas
    }
}

impl<'a> std::ops::DerefMut for CanvasStateGuard<'a> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.canvas
    }
}
