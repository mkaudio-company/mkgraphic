//! Theming and styling constants.

use super::color::Color;
use super::rect::Rect;
use super::font::Font;

/// Theme configuration for the UI.
#[derive(Debug, Clone)]
pub struct Theme {
    // Panel colors
    pub panel_color: Color,

    // Frame colors
    pub frame_color: Color,
    pub frame_hilite_color: Color,
    pub frame_corner_radius: f32,
    pub frame_stroke_width: f32,

    // Scrollbar
    pub scrollbar_color: Color,
    pub scrollbar_width: f32,

    // Button
    pub default_button_color: Color,
    pub button_margin: Rect,
    pub button_corner_radius: f32,
    pub button_text_icon_space: f32,

    // Slider
    pub slider_slot_color: Color,
    pub slider_slot_corner_radius: f32,
    pub slider_thumb_color: Color,
    pub slider_labels_color: Color,
    pub slider_labels_font: Font,
    pub slider_labels_font_size: f32,

    // Dial
    pub dial_color: Color,
    pub dial_indicator_color: Color,
    pub dial_gauge_color: Color,
    pub dial_gauge_width: f32,

    // Text
    pub text_box_font: Font,
    pub text_box_font_size: f32,
    pub text_box_font_color: Color,
    pub text_box_hilite_color: Color,
    pub text_box_hilite_text_color: Color,
    pub text_box_caret_color: Color,
    pub text_box_caret_width: f32,
    pub text_box_idle_color: Color,
    pub disabled_opacity: f32,

    // Labels
    pub label_font: Font,
    pub label_font_size: f32,
    pub label_font_color: Color,

    // Heading
    pub heading_font: Font,
    pub heading_font_size: f32,
    pub heading_font_color: Color,

    // Icons
    pub icon_font: Font,
    pub icon_color: Color,
    pub icon_button_color: Color,

    // Indicator
    pub indicator_color: Color,
    pub indicator_bright_color: Color,
    pub indicator_hilite_color: Color,

    // Input box
    pub input_box_color: Color,

    // Menu
    pub menu_font: Font,
    pub menu_font_size: f32,
    pub menu_font_color: Color,
    pub menu_background_color: Color,
    pub menu_background_opacity: f32,
    pub menu_item_hilite_color: Color,
    pub menu_separator_color: Color,

    // Dialog
    pub dialog_background_color: Color,
    pub dialog_button_size: f32,

    // Tabs
    pub tabs_font: Font,
    pub active_tab_color: Color,
    pub inactive_tab_color: Color,
    pub tab_hilite_color: Color,

    // Tooltip
    pub tooltip_color: Color,
    pub tooltip_text_color: Color,
    pub tooltip_font_size: f32,

    // Selection
    pub selection_hilite_color: Color,

    // Miscellaneous
    pub element_background_color: Color,
    pub element_background_opacity: f32,
    pub child_window_title_size: f32,
    pub child_window_opacity: f32,
    pub default_icon_size: f32,
}

impl Default for Theme {
    fn default() -> Self {
        Self::dark()
    }
}

impl Theme {
    /// Creates a dark theme.
    pub fn dark() -> Self {
        Self {
            // Panel colors
            panel_color: Color::from_rgb_u8(28, 30, 34),

            // Frame colors
            frame_color: Color::from_rgb_u8(35, 39, 46),
            frame_hilite_color: Color::from_rgb_u8(57, 72, 103),
            frame_corner_radius: 3.0,
            frame_stroke_width: 0.7,

            // Scrollbar
            scrollbar_color: Color::from_rgba_u8(80, 80, 80, 200),
            scrollbar_width: 10.0,

            // Button
            default_button_color: Color::from_rgb_u8(44, 50, 60),
            button_margin: Rect::new(10.0, 5.0, 10.0, 5.0),
            button_corner_radius: 4.0,
            button_text_icon_space: 8.0,

            // Slider
            slider_slot_color: Color::from_rgba_u8(0, 0, 0, 100),
            slider_slot_corner_radius: 3.0,
            slider_thumb_color: Color::from_rgb_u8(70, 130, 180),
            slider_labels_color: Color::from_rgba_u8(200, 200, 200, 200),
            slider_labels_font: Font::sans_serif(),
            slider_labels_font_size: 10.0,

            // Dial
            dial_color: Color::from_rgb_u8(200, 200, 200),
            dial_indicator_color: Color::from_rgb_u8(70, 130, 180),
            dial_gauge_color: Color::from_rgb_u8(70, 130, 180),
            dial_gauge_width: 5.0,

            // Text
            text_box_font: Font::monospace(),
            text_box_font_size: 14.0,
            text_box_font_color: Color::from_rgb_u8(200, 200, 200),
            text_box_hilite_color: Color::from_rgba_u8(70, 130, 180, 180),
            text_box_hilite_text_color: Color::from_rgb_u8(255, 255, 255),
            text_box_caret_color: Color::from_rgb_u8(200, 200, 200),
            text_box_caret_width: 1.0,
            text_box_idle_color: Color::from_rgba_u8(200, 200, 200, 100),
            disabled_opacity: 0.35,

            // Labels
            label_font: Font::sans_serif(),
            label_font_size: 14.0,
            label_font_color: Color::from_rgb_u8(200, 200, 200),

            // Heading
            heading_font: Font::sans_serif(),
            heading_font_size: 18.0,
            heading_font_color: Color::from_rgb_u8(200, 200, 200),

            // Icons
            icon_font: Font::sans_serif(),
            icon_color: Color::from_rgb_u8(200, 200, 200),
            icon_button_color: Color::from_rgb_u8(44, 50, 60),

            // Indicator
            indicator_color: Color::from_rgb_u8(100, 100, 100),
            indicator_bright_color: Color::from_rgb_u8(70, 180, 130),
            indicator_hilite_color: Color::from_rgb_u8(200, 200, 200),

            // Input box
            input_box_color: Color::from_rgba_u8(0, 0, 0, 80),

            // Menu
            menu_font: Font::sans_serif(),
            menu_font_size: 14.0,
            menu_font_color: Color::from_rgb_u8(200, 200, 200),
            menu_background_color: Color::from_rgb_u8(36, 40, 48),
            menu_background_opacity: 0.95,
            menu_item_hilite_color: Color::from_rgb_u8(57, 72, 103),
            menu_separator_color: Color::from_rgb_u8(100, 100, 100),

            // Dialog
            dialog_background_color: Color::from_rgba_u8(0, 0, 0, 150),
            dialog_button_size: 1.0,

            // Tabs
            tabs_font: Font::sans_serif(),
            active_tab_color: Color::from_rgb_u8(70, 130, 180),
            inactive_tab_color: Color::from_rgba_u8(35, 39, 46, 200),
            tab_hilite_color: Color::from_rgb_u8(57, 72, 103),

            // Tooltip
            tooltip_color: Color::from_rgb_u8(50, 55, 65),
            tooltip_text_color: Color::from_rgb_u8(200, 200, 200),
            tooltip_font_size: 12.0,

            // Selection
            selection_hilite_color: Color::from_rgba_u8(70, 130, 180, 100),

            // Miscellaneous
            element_background_color: Color::from_rgb_u8(35, 39, 46),
            element_background_opacity: 0.95,
            child_window_title_size: 18.0,
            child_window_opacity: 0.95,
            default_icon_size: 1.0,
        }
    }

    /// Creates a light theme.
    pub fn light() -> Self {
        Self {
            // Panel colors
            panel_color: Color::from_rgb_u8(240, 240, 245),

            // Frame colors
            frame_color: Color::from_rgb_u8(220, 220, 225),
            frame_hilite_color: Color::from_rgb_u8(70, 130, 180),
            frame_corner_radius: 3.0,
            frame_stroke_width: 0.7,

            // Scrollbar
            scrollbar_color: Color::from_rgba_u8(150, 150, 150, 200),
            scrollbar_width: 10.0,

            // Button
            default_button_color: Color::from_rgb_u8(200, 200, 210),
            button_margin: Rect::new(10.0, 5.0, 10.0, 5.0),
            button_corner_radius: 4.0,
            button_text_icon_space: 8.0,

            // Slider
            slider_slot_color: Color::from_rgba_u8(0, 0, 0, 40),
            slider_slot_corner_radius: 3.0,
            slider_thumb_color: Color::from_rgb_u8(70, 130, 180),
            slider_labels_color: Color::from_rgba_u8(60, 60, 60, 200),
            slider_labels_font: Font::sans_serif(),
            slider_labels_font_size: 10.0,

            // Dial
            dial_color: Color::from_rgb_u8(60, 60, 60),
            dial_indicator_color: Color::from_rgb_u8(70, 130, 180),
            dial_gauge_color: Color::from_rgb_u8(70, 130, 180),
            dial_gauge_width: 5.0,

            // Text
            text_box_font: Font::monospace(),
            text_box_font_size: 14.0,
            text_box_font_color: Color::from_rgb_u8(40, 40, 40),
            text_box_hilite_color: Color::from_rgba_u8(70, 130, 180, 180),
            text_box_hilite_text_color: Color::from_rgb_u8(255, 255, 255),
            text_box_caret_color: Color::from_rgb_u8(40, 40, 40),
            text_box_caret_width: 1.0,
            text_box_idle_color: Color::from_rgba_u8(100, 100, 100, 150),
            disabled_opacity: 0.35,

            // Labels
            label_font: Font::sans_serif(),
            label_font_size: 14.0,
            label_font_color: Color::from_rgb_u8(40, 40, 40),

            // Heading
            heading_font: Font::sans_serif(),
            heading_font_size: 18.0,
            heading_font_color: Color::from_rgb_u8(40, 40, 40),

            // Icons
            icon_font: Font::sans_serif(),
            icon_color: Color::from_rgb_u8(60, 60, 60),
            icon_button_color: Color::from_rgb_u8(200, 200, 210),

            // Indicator
            indicator_color: Color::from_rgb_u8(160, 160, 160),
            indicator_bright_color: Color::from_rgb_u8(70, 180, 130),
            indicator_hilite_color: Color::from_rgb_u8(80, 80, 80),

            // Input box
            input_box_color: Color::from_rgba_u8(255, 255, 255, 200),

            // Menu
            menu_font: Font::sans_serif(),
            menu_font_size: 14.0,
            menu_font_color: Color::from_rgb_u8(40, 40, 40),
            menu_background_color: Color::from_rgb_u8(250, 250, 252),
            menu_background_opacity: 0.98,
            menu_item_hilite_color: Color::from_rgb_u8(70, 130, 180),
            menu_separator_color: Color::from_rgb_u8(180, 180, 180),

            // Dialog
            dialog_background_color: Color::from_rgba_u8(0, 0, 0, 100),
            dialog_button_size: 1.0,

            // Tabs
            tabs_font: Font::sans_serif(),
            active_tab_color: Color::from_rgb_u8(70, 130, 180),
            inactive_tab_color: Color::from_rgba_u8(200, 200, 210, 200),
            tab_hilite_color: Color::from_rgb_u8(70, 130, 180),

            // Tooltip
            tooltip_color: Color::from_rgb_u8(50, 55, 65),
            tooltip_text_color: Color::from_rgb_u8(240, 240, 240),
            tooltip_font_size: 12.0,

            // Selection
            selection_hilite_color: Color::from_rgba_u8(70, 130, 180, 80),

            // Miscellaneous
            element_background_color: Color::from_rgb_u8(250, 250, 252),
            element_background_opacity: 0.98,
            child_window_title_size: 18.0,
            child_window_opacity: 0.98,
            default_icon_size: 1.0,
        }
    }
}

use std::sync::RwLock;

static CURRENT_THEME: RwLock<Option<Theme>> = RwLock::new(None);

/// Returns a reference to the current theme.
pub fn get_theme() -> Theme {
    CURRENT_THEME
        .read()
        .unwrap()
        .clone()
        .unwrap_or_else(Theme::default)
}

/// Sets the current theme.
pub fn set_theme(theme: Theme) {
    *CURRENT_THEME.write().unwrap() = Some(theme);
}
