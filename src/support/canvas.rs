//! Canvas abstraction for 2D drawing.
//!
//! This module provides a high-level drawing API that wraps the underlying
//! graphics backend (tiny-skia).

use std::collections::HashMap;
use std::sync::{Mutex, OnceLock};

use super::circle::Circle;
use super::color::Color;
use super::font::{Font, FontDatabase, FontStretch, FontStyle};
use super::point::Point;
use super::rect::Rect;

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

    pub const fn with_values(
        top_left: f32,
        top_right: f32,
        bottom_right: f32,
        bottom_left: f32,
    ) -> Self {
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
    clip_rect: Option<Rect>,
}

struct CanvasState {
    fill_color: Color,
    stroke_color: Color,
    line_width: f32,
    text_align: TextAlign,
    transform: tiny_skia::Transform,
    font_size: f32,
    clip_rect: Option<Rect>,
}

fn to_fontdb_style(style: FontStyle) -> fontdb::Style {
    match style {
        FontStyle::Normal => fontdb::Style::Normal,
        FontStyle::Italic => fontdb::Style::Italic,
        FontStyle::Oblique => fontdb::Style::Oblique,
    }
}

fn to_fontdb_stretch(stretch: FontStretch) -> fontdb::Stretch {
    match stretch {
        FontStretch::UltraCondensed => fontdb::Stretch::UltraCondensed,
        FontStretch::ExtraCondensed => fontdb::Stretch::ExtraCondensed,
        FontStretch::Condensed => fontdb::Stretch::Condensed,
        FontStretch::SemiCondensed => fontdb::Stretch::SemiCondensed,
        FontStretch::Normal => fontdb::Stretch::Normal,
        FontStretch::SemiExpanded => fontdb::Stretch::SemiExpanded,
        FontStretch::Expanded => fontdb::Stretch::Expanded,
        FontStretch::ExtraExpanded => fontdb::Stretch::ExtraExpanded,
        FontStretch::UltraExpanded => fontdb::Stretch::UltraExpanded,
    }
}

/// Widely-installed fonts with broad non-Latin coverage (CJK, Hangul,
/// etc.), tried - in order - before falling back to a brute-force scan of
/// every font loaded on the system. Order only matters for tie-breaking
/// when more than one would work; whichever actually has the glyph wins.
const FALLBACK_FAMILIES: &[&str] = &[
    "Noto Sans CJK SC",
    "Noto Sans CJK TC",
    "Noto Sans CJK JP",
    "Noto Sans CJK KR",
    "PingFang SC",
    "PingFang TC",
    "PingFang HK",
    "Hiragino Sans",
    "Hiragino Kaku Gothic ProN",
    "Apple SD Gothic Neo",
    "Malgun Gothic",
    "Microsoft YaHei",
    "Microsoft JhengHei",
    "SimSun",
    "Yu Gothic",
    "Meiryo",
    "Noto Sans",
    "Noto Sans Symbols",
    "Noto Sans Symbols2",
    "Arial Unicode MS",
];

/// True if `face_id` has a glyph for `ch` (i.e. isn't going to render as
/// `.notdef`/tofu).
fn face_has_glyph(font_db: &FontDatabase, face_id: fontdb::ID, ch: char) -> bool {
    font_db
        .inner()
        .with_face_data(face_id, |data, index| {
            ttf_parser::Face::parse(data, index)
                .ok()
                .and_then(|face| face.glyph_index(ch))
                .is_some()
        })
        .unwrap_or(false)
}

/// Resolves which font face to use for a single character: the primary
/// (requested/selected) font if it covers `ch`, otherwise the first one
/// that does, tried among a short list of broad-coverage candidates and
/// then every other loaded font as a last resort. Results are memoized
/// process-wide since the same characters (spaces, common CJK syllables/
/// ideographs, punctuation) recur constantly.
fn resolve_face_for_char(font_db: &FontDatabase, primary: fontdb::ID, ch: char) -> fontdb::ID {
    static CACHE: OnceLock<Mutex<HashMap<char, fontdb::ID>>> = OnceLock::new();
    let cache = CACHE.get_or_init(|| Mutex::new(HashMap::new()));

    if let Some(&id) = cache.lock().unwrap().get(&ch) {
        return id;
    }

    if face_has_glyph(font_db, primary, ch) {
        return primary;
    }

    for family in FALLBACK_FAMILIES {
        let query = fontdb::Query {
            families: &[fontdb::Family::Name(family)],
            weight: fontdb::Weight(400),
            stretch: fontdb::Stretch::Normal,
            style: fontdb::Style::Normal,
        };
        if let Some(id) = font_db.inner().query(&query) {
            if face_has_glyph(font_db, id, ch) {
                cache.lock().unwrap().insert(ch, id);
                return id;
            }
        }
    }

    for face in font_db.inner().faces() {
        if face_has_glyph(font_db, face.id, ch) {
            cache.lock().unwrap().insert(ch, face.id);
            return face.id;
        }
    }

    // Nothing on the system has this glyph; fall back to the primary font
    // (renders `.notdef`, same as if no fallback logic existed at all).
    cache.lock().unwrap().insert(ch, primary);
    primary
}

/// Splits `text` into maximal runs of consecutive characters that resolve
/// to the same font face, so mixed-script strings (e.g. English text with
/// a Korean word in it) render correctly even though no single font may
/// cover every character.
fn text_runs(font_db: &FontDatabase, primary: fontdb::ID, text: &str) -> Vec<(fontdb::ID, String)> {
    let mut runs: Vec<(fontdb::ID, String)> = Vec::new();
    for ch in text.chars() {
        let face_id = resolve_face_for_char(font_db, primary, ch);
        match runs.last_mut() {
            Some((id, s)) if *id == face_id => s.push(ch),
            _ => runs.push((face_id, ch.to_string())),
        }
    }
    runs
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
            clip_rect: None,
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
            clip_rect: None,
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

    /// Resets the transform to identity, discarding any accumulated
    /// translate/rotate/scale. Used to establish a clean base transform
    /// (e.g. a HiDPI scale factor) at the start of a frame.
    pub fn reset_transform(&mut self) {
        self.transform = tiny_skia::Transform::identity();
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

    /// Draws a quadratic Bezier curve to `to`, using `ctrl` as the
    /// control point. Added for `support::math`'s radical-sign hook (the
    /// first caller needing a real curve rather than the straight-line-only
    /// paths every other element in this codebase has used so far).
    pub fn quad_to(&mut self, ctrl: Point, to: Point) {
        if let Some(ref mut pb) = self.path_builder {
            pb.quad_to(ctrl.x, ctrl.y, to.x, to.y);
        }
    }

    /// Draws a cubic Bezier curve to `to`, using `ctrl1`/`ctrl2` as the
    /// two control points. See `quad_to`'s doc comment for why this
    /// exists now.
    pub fn cubic_to(&mut self, ctrl1: Point, ctrl2: Point, to: Point) {
        if let Some(ref mut pb) = self.path_builder {
            pb.cubic_to(ctrl1.x, ctrl1.y, ctrl2.x, ctrl2.y, to.x, to.y);
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
        paint.set_color(
            tiny_skia::Color::from_rgba(color.red, color.green, color.blue, color.alpha)
                .unwrap_or(tiny_skia::Color::BLACK),
        );
        paint.anti_alias = true;
        paint
    }

    /// Creates a clip mask for the current clip_rect.
    fn create_clip_mask(&self) -> Option<tiny_skia::Mask> {
        self.clip_rect.and_then(|clip| {
            let mut mask = tiny_skia::Mask::new(self.pixmap.width(), self.pixmap.height())?;
            let clip_path = {
                let mut pb = tiny_skia::PathBuilder::new();
                pb.push_rect(tiny_skia::Rect::from_ltrb(
                    clip.left,
                    clip.top,
                    clip.right,
                    clip.bottom,
                )?);
                pb.finish()?
            };
            // Use the current content transform (not identity) so the mask
            // lines up with the same coordinate space as the paths it clips.
            // clip_rect is expressed in the same logical units as draw calls,
            // and at scale != 1.0 (HiDPI) identity would misalign the mask
            // against the physical-pixel-resolution pixmap.
            mask.fill_path(
                &clip_path,
                tiny_skia::FillRule::Winding,
                true,
                self.transform,
            );
            Some(mask)
        })
    }

    /// Fills the current path.
    pub fn fill(&mut self) {
        if let Some(pb) = self.path_builder.take() {
            if let Some(path) = pb.finish() {
                let paint = Self::color_to_paint(self.fill_color);
                let clip_mask = self.create_clip_mask();
                self.pixmap.fill_path(
                    &path,
                    &paint,
                    tiny_skia::FillRule::Winding,
                    self.transform,
                    clip_mask.as_ref(),
                );
            }
        }
    }

    /// Fills the current path and preserves it.
    pub fn fill_preserve(&mut self) {
        if let Some(ref pb) = self.path_builder {
            if let Some(path) = pb.clone().finish() {
                let paint = Self::color_to_paint(self.fill_color);
                let clip_mask = self.create_clip_mask();
                self.pixmap.fill_path(
                    &path,
                    &paint,
                    tiny_skia::FillRule::Winding,
                    self.transform,
                    clip_mask.as_ref(),
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
                let clip_mask = self.create_clip_mask();
                self.pixmap
                    .stroke_path(&path, &paint, &stroke, self.transform, clip_mask.as_ref());
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
                let clip_mask = self.create_clip_mask();
                self.pixmap
                    .stroke_path(&path, &paint, &stroke, self.transform, clip_mask.as_ref());
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
            clip_rect: self.clip_rect,
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
            self.clip_rect = state.clip_rect;
        }
    }

    /// Sets the clipping rectangle.
    pub fn set_clip_rect(&mut self, rect: Option<Rect>) {
        self.clip_rect = rect;
    }

    /// Gets the current clipping rectangle.
    pub fn clip_rect(&self) -> Option<Rect> {
        self.clip_rect
    }

    /// Intersects the current clip rect with the given rect.
    pub fn clip(&mut self, rect: Rect) {
        self.clip_rect = Some(match self.clip_rect {
            Some(existing) => existing
                .intersection(rect)
                .unwrap_or(Rect::new(0.0, 0.0, 0.0, 0.0)),
            None => rect,
        });
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

    /// Measures text width using the font system.
    pub fn measure_text(&self, text: &str) -> TextMetrics {
        let width = self.text_width(text);
        TextMetrics {
            ascent: self.font_size * 0.8,
            descent: self.font_size * 0.2,
            leading: self.font_size * 0.1,
            width,
            height: self.font_size,
        }
    }

    /// The currently selected font size (see [`Canvas::font_size`]) --
    /// `support::math`'s layout needs this back out to size a math run at
    /// exactly the font size the surrounding text run is using.
    pub fn current_font_size(&self) -> f32 {
        self.font_size
    }

    /// Real font metrics for the currently selected font (see
    /// [`Canvas::font`]), read from the actual resolved face's
    /// `hhea`/`OS/2` ascender/descender/line-gap -- unlike
    /// [`Canvas::measure_text`]'s `ascent`/`descent`, which are just fixed
    /// `0.8`/`0.2` fractions of `font_size` (fine for rough line-height
    /// bookkeeping, but not precise enough for `support::math`'s
    /// superscript/subscript clearance rules, which need the font's real
    /// x-height-relative geometry). Falls back to that same synthetic
    /// split if no face resolves at all (matching `text_width`'s own
    /// graceful-degradation convention).
    pub fn font_metrics(&self) -> FontMetrics {
        let fallback = FontMetrics {
            ascent: self.font_size * 0.8,
            descent: self.font_size * 0.2,
            height: self.font_size,
            leading: self.font_size * 0.1,
        };

        static FONT_DB: OnceLock<FontDatabase> = OnceLock::new();
        let font_db = FONT_DB.get_or_init(FontDatabase::with_system_fonts);

        let Some(primary) = self.primary_font_id(font_db) else {
            return fallback;
        };

        let mut result = fallback;
        font_db
            .inner()
            .with_face_data(primary, |font_data_ref, face_index| {
                let Ok(face) = ttf_parser::Face::parse(font_data_ref, face_index) else {
                    return;
                };
                let units_per_em = face.units_per_em() as f32;
                if units_per_em <= 0.0 {
                    return;
                }
                let scale = self.font_size / units_per_em;
                let ascent = face.ascender() as f32 * scale;
                // `ttf_parser`'s `descender()` is negative (below the
                // baseline); `FontMetrics::descent` is the positive extent.
                let descent = -(face.descender() as f32) * scale;
                let leading = face.line_gap() as f32 * scale;
                result = FontMetrics {
                    ascent,
                    descent,
                    height: ascent + descent + leading,
                    leading,
                };
            });
        result
    }

    /// Resolves the fontdb face matching the canvas's currently selected
    /// font (see [`Canvas::font`]), falling back to generic sans-serif at
    /// regular weight if none was set or the requested family isn't
    /// installed. This is the "primary" font for a piece of text; any
    /// characters it doesn't cover (e.g. Korean/Japanese/Chinese text with
    /// a Latin-only primary font) fall back per-character - see
    /// [`resolve_face_for_char`].
    fn primary_font_id(&self, font_db: &FontDatabase) -> Option<fontdb::ID> {
        let weight = self
            .current_font
            .as_ref()
            .map(|f| fontdb::Weight(f.weight().value()))
            .unwrap_or(fontdb::Weight(400));
        let stretch = self
            .current_font
            .as_ref()
            .map(|f| to_fontdb_stretch(f.stretch()))
            .unwrap_or(fontdb::Stretch::Normal);
        let style = self
            .current_font
            .as_ref()
            .map(|f| to_fontdb_style(f.style()))
            .unwrap_or(fontdb::Style::Normal);

        let family = match self.current_font.as_ref().map(|f| f.family()) {
            Some(f) if f.eq_ignore_ascii_case("serif") => fontdb::Family::Serif,
            Some(f) if f.eq_ignore_ascii_case("monospace") => fontdb::Family::Monospace,
            Some(f) if f.eq_ignore_ascii_case("cursive") => fontdb::Family::Cursive,
            Some(f) if f.eq_ignore_ascii_case("fantasy") => fontdb::Family::Fantasy,
            Some(f) if !f.eq_ignore_ascii_case("sans-serif") => fontdb::Family::Name(f),
            _ => fontdb::Family::SansSerif,
        };

        let query = fontdb::Query {
            families: &[family],
            weight,
            stretch,
            style,
        };

        font_db.inner().query(&query)
    }

    /// Returns the width of the given text in pixels.
    pub fn text_width(&self, text: &str) -> f32 {
        if text.is_empty() {
            return 0.0;
        }

        static FONT_DB: OnceLock<FontDatabase> = OnceLock::new();
        let font_db = FONT_DB.get_or_init(FontDatabase::with_system_fonts);

        let Some(primary) = self.primary_font_id(font_db) else {
            // Fallback: estimate width
            return text.chars().count() as f32 * self.font_size * 0.6;
        };

        let mut total_width = 0.0f32;
        let mut measured_any = false;

        for (face_id, run) in text_runs(font_db, primary, text) {
            font_db
                .inner()
                .with_face_data(face_id, |font_data_ref, face_index| {
                    let Ok(face) = ttf_parser::Face::parse(font_data_ref, face_index) else {
                        return;
                    };

                    let Some(buzz_face) = rustybuzz::Face::from_slice(font_data_ref, face_index)
                    else {
                        return;
                    };

                    let mut buffer = rustybuzz::UnicodeBuffer::new();
                    buffer.push_str(&run);
                    let output = rustybuzz::shape(&buzz_face, &[], buffer);

                    let units_per_em = face.units_per_em() as f32;
                    let scale = self.font_size / units_per_em;

                    for pos in output.glyph_positions() {
                        total_width += (pos.x_advance as f32) * scale;
                    }
                    measured_any = true;
                });
        }

        if !measured_any || total_width == 0.0 {
            // Fallback if measurement failed
            text.chars().count() as f32 * self.font_size * 0.6
        } else {
            total_width
        }
    }

    /// Returns the width of a substring (for cursor positioning).
    pub fn text_width_to_position(&self, text: &str, char_position: usize) -> f32 {
        if char_position == 0 || text.is_empty() {
            return 0.0;
        }

        let prefix: String = text.chars().take(char_position).collect();
        self.text_width(&prefix)
    }

    /// Fills text at the given position.
    ///
    /// Handles mixed-script text (e.g. English mixed with Korean/Japanese/
    /// Chinese) by resolving each character to a font that actually has a
    /// glyph for it, grouping consecutive same-font characters into runs -
    /// see [`text_runs`]. A single font rarely covers every script, so this
    /// is required for anything beyond the primary font's own coverage.
    pub fn fill_text(&mut self, text: &str, p: Point) {
        // Get or initialize the global font database
        static FONT_DB: OnceLock<FontDatabase> = OnceLock::new();
        let font_db = FONT_DB.get_or_init(FontDatabase::with_system_fonts);

        let Some(primary) = self.primary_font_id(font_db) else {
            return;
        };

        let runs = text_runs(font_db, primary, text);
        let clip_mask = self.create_clip_mask();

        let mut x_pos = p.x;
        let y_pos = p.y;

        for (face_id, run) in runs {
            font_db
                .inner()
                .with_face_data(face_id, |font_data_ref, face_index| {
                    // Parse the font
                    let Ok(face) = ttf_parser::Face::parse(font_data_ref, face_index) else {
                        return;
                    };

                    // Create rustybuzz face
                    let Some(buzz_face) = rustybuzz::Face::from_slice(font_data_ref, face_index)
                    else {
                        return;
                    };

                    // Shape the run
                    let mut buffer = rustybuzz::UnicodeBuffer::new();
                    buffer.push_str(&run);
                    let output = rustybuzz::shape(&buzz_face, &[], buffer);

                    // Calculate scale factor
                    let units_per_em = face.units_per_em() as f32;
                    let scale = self.font_size / units_per_em;

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
                            clip_mask.as_ref(),
                        );

                        // Advance position
                        x_pos += (pos.x_advance as f32) * scale;
                    }
                });
        }
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
    #[allow(clippy::too_many_arguments)]
    fn render_glyph_static(
        pixmap: &mut tiny_skia::Pixmap,
        face: &ttf_parser::Face,
        glyph_id: ttf_parser::GlyphId,
        x: f32,
        y: f32,
        scale: f32,
        fill_color: Color,
        transform: tiny_skia::Transform,
        clip_mask: Option<&tiny_skia::Mask>,
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
                    clip_mask,
                );
            }
        }
    }

    /// Clears the canvas with the given color.
    pub fn clear(&mut self, color: Color) {
        self.pixmap.fill(
            tiny_skia::Color::from_rgba(color.red, color.green, color.blue, color.alpha)
                .unwrap_or(tiny_skia::Color::WHITE),
        );
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

#[cfg(test)]
mod text_tests {
    use super::*;

    fn test_font_db() -> &'static FontDatabase {
        static FONT_DB: OnceLock<FontDatabase> = OnceLock::new();
        FONT_DB.get_or_init(FontDatabase::with_system_fonts)
    }

    /// `None` on a system with no sans-serif font installed at all (e.g. a
    /// bare-minimum container image) - a real, if unusual, environment that
    /// tests should skip gracefully in rather than hard-panic over, since
    /// it's not something this fix can control.
    fn default_sans_serif(font_db: &FontDatabase) -> Option<fontdb::ID> {
        let query = fontdb::Query {
            families: &[fontdb::Family::SansSerif],
            weight: fontdb::Weight(400),
            stretch: fontdb::Stretch::Normal,
            style: fontdb::Style::Normal,
        };
        font_db.inner().query(&query)
    }

    // These assert actual glyph coverage on the *resolved* face, not just
    // that something painted - a missing glyph still renders a visible
    // `.notdef` tofu box, so "some pixels got painted" alone can't tell
    // real script support from silent tofu.

    #[test]
    fn resolve_face_for_char_finds_real_korean_coverage() {
        let font_db = test_font_db();
        let Some(primary) = default_sans_serif(font_db) else {
            eprintln!("skipping: no sans-serif font installed on this system");
            return;
        };
        let resolved = resolve_face_for_char(font_db, primary, '안');
        assert!(
            face_has_glyph(font_db, resolved, '안'),
            "resolved face has no real glyph for '안' - would render as tofu"
        );
    }

    #[test]
    fn resolve_face_for_char_finds_real_japanese_coverage() {
        let font_db = test_font_db();
        let Some(primary) = default_sans_serif(font_db) else {
            eprintln!("skipping: no sans-serif font installed on this system");
            return;
        };
        let resolved = resolve_face_for_char(font_db, primary, 'こ');
        assert!(
            face_has_glyph(font_db, resolved, 'こ'),
            "resolved face has no real glyph for 'こ' - would render as tofu"
        );
    }

    #[test]
    fn resolve_face_for_char_finds_real_chinese_coverage() {
        let font_db = test_font_db();
        let Some(primary) = default_sans_serif(font_db) else {
            eprintln!("skipping: no sans-serif font installed on this system");
            return;
        };
        let resolved = resolve_face_for_char(font_db, primary, '你');
        assert!(
            face_has_glyph(font_db, resolved, '你'),
            "resolved face has no real glyph for '你' - would render as tofu"
        );
    }

    #[test]
    fn text_width_is_positive_for_non_latin_scripts() {
        let mut canvas = Canvas::new(400, 100).unwrap();
        canvas.font_size(24.0);

        for text in [
            "Hello",
            "안녕하세요",
            "こんにちは",
            "你好",
            "Hello 안녕 こんにちは",
        ] {
            let width = canvas.text_width(text);
            assert!(
                width > 0.0,
                "expected positive width for {text:?}, got {width}"
            );
        }
    }

    #[test]
    fn mixed_script_text_measures_wider_than_its_latin_prefix() {
        let mut canvas = Canvas::new(500, 100).unwrap();
        canvas.font_size(24.0);

        let latin_only_width = canvas.text_width("Hello ");
        let mixed_width = canvas.text_width("Hello 안녕");

        assert!(
            mixed_width > latin_only_width,
            "mixed-script text should measure wider than its Latin prefix alone"
        );
    }

    #[test]
    fn respects_explicitly_selected_font_family() {
        // `Canvas::font` was previously stored but never actually consulted
        // when querying/shaping - this would have queried plain
        // `fontdb::Family::SansSerif` regardless of what was set here.
        let mut canvas = Canvas::new(400, 100).unwrap();
        canvas.font(Font::monospace());
        canvas.font_size(24.0);

        let font_db = test_font_db();
        let Some(primary) = canvas.primary_font_id(font_db) else {
            eprintln!("skipping: no monospace font installed on this system");
            return;
        };
        let Some(monospace_query_id) = font_db.inner().query(&fontdb::Query {
            families: &[fontdb::Family::Monospace],
            weight: fontdb::Weight(400),
            stretch: fontdb::Stretch::Normal,
            style: fontdb::Style::Normal,
        }) else {
            eprintln!("skipping: no monospace font installed on this system");
            return;
        };
        assert_eq!(
            primary, monospace_query_id,
            "selecting a monospace font should actually query for one"
        );
    }

    #[test]
    fn quad_to_and_cubic_to_actually_extend_the_current_path() {
        let mut canvas = Canvas::new(200, 200).unwrap();
        canvas.begin_path();
        canvas.move_to(Point::new(10.0, 10.0));
        canvas.quad_to(Point::new(50.0, 10.0), Point::new(50.0, 50.0));
        canvas.cubic_to(
            Point::new(50.0, 90.0),
            Point::new(90.0, 90.0),
            Point::new(90.0, 50.0),
        );
        canvas.close_path();
        canvas.fill_style(Color::new(0.0, 0.0, 0.0, 1.0));
        canvas.fill();

        let path_builder = canvas.path_builder.take();
        assert!(
            path_builder.is_none(),
            "fill() should have consumed/cleared the in-progress path"
        );

        // Re-check bounds via a fresh path built the same way, since
        // `fill()` already consumed the one above -- `quad_to`/`cubic_to`
        // are no-ops if `path_builder` is `None` (mirroring `line_to`'s
        // own guard), so this also confirms they aren't silently dropped.
        canvas.begin_path();
        canvas.move_to(Point::new(10.0, 10.0));
        canvas.quad_to(Point::new(50.0, 10.0), Point::new(50.0, 50.0));
        canvas.cubic_to(
            Point::new(50.0, 90.0),
            Point::new(90.0, 90.0),
            Point::new(90.0, 50.0),
        );
        let path = canvas
            .path_builder
            .as_ref()
            .unwrap()
            .clone()
            .finish()
            .expect("path should be buildable");
        let bounds = path.bounds();
        assert!(
            bounds.width() > 1.0 && bounds.height() > 1.0,
            "expected a real, non-degenerate curved path, got {bounds:?}"
        );
    }

    #[test]
    fn font_metrics_returns_real_face_data_not_just_the_synthetic_fallback() {
        let mut canvas = Canvas::new(400, 100).unwrap();
        canvas.font(Font::sans_serif());
        canvas.font_size(20.0);

        let font_db = test_font_db();
        if canvas.primary_font_id(font_db).is_none() {
            eprintln!("skipping: no sans-serif font installed on this system");
            return;
        }

        let metrics = canvas.font_metrics();
        assert!(
            metrics.ascent > 0.0,
            "expected a positive real ascent, got {}",
            metrics.ascent
        );
        assert!(
            metrics.descent > 0.0,
            "expected a positive real descent, got {}",
            metrics.descent
        );
        // Not a strict requirement of every real font, but the synthetic
        // fallback is exactly `font_size * 0.8` / `* 0.2` -- if a real face
        // resolved, its ascent/descent split need not match that ratio
        // exactly, so just confirm this isn't silently returning the
        // fallback's *exact* values by coincidence every time.
        assert_ne!(
            (metrics.ascent, metrics.descent),
            (20.0 * 0.8, 20.0 * 0.2),
            "got exactly the synthetic fallback values -- font_metrics may not be reading the real face"
        );
    }

    #[test]
    fn current_font_size_reflects_the_last_set_size() {
        let mut canvas = Canvas::new(100, 100).unwrap();
        canvas.font_size(31.0);
        assert_eq!(canvas.current_font_size(), 31.0);
    }
}
