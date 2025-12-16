//! Color types and common color constants.

use std::ops::{Add, Sub, Mul};

/// An RGBA color with floating-point components (0.0 to 1.0).
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct Color {
    pub red: f32,
    pub green: f32,
    pub blue: f32,
    pub alpha: f32,
}

impl Color {
    /// Creates a new color with the given RGBA components.
    #[inline]
    pub const fn new(red: f32, green: f32, blue: f32, alpha: f32) -> Self {
        Self { red, green, blue, alpha }
    }

    /// Creates a new opaque color (alpha = 1.0).
    #[inline]
    pub const fn rgb(red: f32, green: f32, blue: f32) -> Self {
        Self { red, green, blue, alpha: 1.0 }
    }

    /// Creates a color from a 24-bit RGB integer (0xRRGGBB).
    #[inline]
    pub const fn from_rgb_u32(rgb: u32) -> Self {
        Self {
            red: ((rgb >> 16) & 0xff) as f32 / 255.0,
            green: ((rgb >> 8) & 0xff) as f32 / 255.0,
            blue: (rgb & 0xff) as f32 / 255.0,
            alpha: 1.0,
        }
    }

    /// Creates a color from a 32-bit RGBA integer (0xRRGGBBAA).
    #[inline]
    pub const fn from_rgba_u32(rgba: u32) -> Self {
        Self {
            red: ((rgba >> 24) & 0xff) as f32 / 255.0,
            green: ((rgba >> 16) & 0xff) as f32 / 255.0,
            blue: ((rgba >> 8) & 0xff) as f32 / 255.0,
            alpha: (rgba & 0xff) as f32 / 255.0,
        }
    }

    /// Creates a color from 8-bit RGB components.
    #[inline]
    pub const fn from_rgb_u8(r: u8, g: u8, b: u8) -> Self {
        Self {
            red: r as f32 / 255.0,
            green: g as f32 / 255.0,
            blue: b as f32 / 255.0,
            alpha: 1.0,
        }
    }

    /// Creates a color from 8-bit RGBA components.
    #[inline]
    pub const fn from_rgba_u8(r: u8, g: u8, b: u8, a: u8) -> Self {
        Self {
            red: r as f32 / 255.0,
            green: g as f32 / 255.0,
            blue: b as f32 / 255.0,
            alpha: a as f32 / 255.0,
        }
    }

    /// Returns a new color with the given opacity (alpha).
    #[inline]
    pub const fn with_alpha(self, alpha: f32) -> Self {
        Self { alpha, ..self }
    }

    /// Returns a new color with brightness adjusted by the given factor.
    #[inline]
    pub fn level(self, amount: f32) -> Self {
        Self {
            red: self.red * amount,
            green: self.green * amount,
            blue: self.blue * amount,
            alpha: self.alpha,
        }
    }

    /// Converts to 8-bit RGBA.
    #[inline]
    pub fn to_rgba_u8(self) -> (u8, u8, u8, u8) {
        (
            (self.red * 255.0).clamp(0.0, 255.0) as u8,
            (self.green * 255.0).clamp(0.0, 255.0) as u8,
            (self.blue * 255.0).clamp(0.0, 255.0) as u8,
            (self.alpha * 255.0).clamp(0.0, 255.0) as u8,
        )
    }

    /// Converts to a 32-bit RGBA integer.
    #[inline]
    pub fn to_rgba_u32(self) -> u32 {
        let (r, g, b, a) = self.to_rgba_u8();
        ((r as u32) << 24) | ((g as u32) << 16) | ((b as u32) << 8) | (a as u32)
    }

    /// Linearly interpolates between two colors.
    #[inline]
    pub fn lerp(self, other: Color, t: f32) -> Self {
        Self {
            red: self.red + (other.red - self.red) * t,
            green: self.green + (other.green - self.green) * t,
            blue: self.blue + (other.blue - self.blue) * t,
            alpha: self.alpha + (other.alpha - self.alpha) * t,
        }
    }
}

impl Add for Color {
    type Output = Self;

    fn add(self, other: Self) -> Self {
        Self {
            red: self.red + other.red,
            green: self.green + other.green,
            blue: self.blue + other.blue,
            alpha: self.alpha + other.alpha * (1.0 - self.alpha),
        }
    }
}

impl Sub for Color {
    type Output = Self;

    fn sub(self, other: Self) -> Self {
        Self {
            red: self.red - other.red,
            green: self.green - other.green,
            blue: self.blue - other.blue,
            alpha: self.alpha + other.alpha * (1.0 - self.alpha),
        }
    }
}

impl Mul<f32> for Color {
    type Output = Self;

    fn mul(self, scalar: f32) -> Self {
        Self {
            red: self.red * scalar,
            green: self.green * scalar,
            blue: self.blue * scalar,
            alpha: self.alpha,
        }
    }
}

impl Mul<Color> for f32 {
    type Output = Color;

    fn mul(self, color: Color) -> Color {
        color * self
    }
}

/// Common color constants.
pub mod colors {
    use super::Color;

    pub const ALICE_BLUE: Color = Color::from_rgb_u8(240, 248, 255);
    pub const ANTIQUE_WHITE: Color = Color::from_rgb_u8(250, 235, 215);
    pub const AQUAMARINE: Color = Color::from_rgb_u8(50, 191, 193);
    pub const AZURE: Color = Color::from_rgb_u8(240, 255, 255);
    pub const BEIGE: Color = Color::from_rgb_u8(245, 245, 220);
    pub const BISQUE: Color = Color::from_rgb_u8(255, 228, 196);
    pub const BLACK: Color = Color::from_rgb_u8(0, 0, 0);
    pub const BLANCHED_ALMOND: Color = Color::from_rgb_u8(255, 235, 205);
    pub const BLUE: Color = Color::from_rgb_u8(0, 0, 255);
    pub const BLUE_VIOLET: Color = Color::from_rgb_u8(138, 43, 226);
    pub const BROWN: Color = Color::from_rgb_u8(165, 42, 42);
    pub const BURLY_WOOD: Color = Color::from_rgb_u8(222, 184, 135);
    pub const CADET_BLUE: Color = Color::from_rgb_u8(95, 146, 158);
    pub const CHARTREUSE: Color = Color::from_rgb_u8(127, 255, 0);
    pub const CHOCOLATE: Color = Color::from_rgb_u8(210, 105, 30);
    pub const CORAL: Color = Color::from_rgb_u8(255, 114, 86);
    pub const CORNFLOWER_BLUE: Color = Color::from_rgb_u8(34, 34, 152);
    pub const CORN_SILK: Color = Color::from_rgb_u8(255, 248, 220);
    pub const CYAN: Color = Color::from_rgb_u8(0, 255, 255);
    pub const DARK_GOLDENROD: Color = Color::from_rgb_u8(184, 134, 11);
    pub const DARK_GREEN: Color = Color::from_rgb_u8(0, 86, 45);
    pub const DARK_KHAKI: Color = Color::from_rgb_u8(189, 183, 107);
    pub const DARK_OLIVE_GREEN: Color = Color::from_rgb_u8(85, 86, 47);
    pub const DARK_ORANGE: Color = Color::from_rgb_u8(255, 140, 0);
    pub const DARK_ORCHID: Color = Color::from_rgb_u8(139, 32, 139);
    pub const DARK_SALMON: Color = Color::from_rgb_u8(233, 150, 122);
    pub const DARK_SEA_GREEN: Color = Color::from_rgb_u8(143, 188, 143);
    pub const DARK_SLATE_BLUE: Color = Color::from_rgb_u8(56, 75, 102);
    pub const DARK_SLATE_GRAY: Color = Color::from_rgb_u8(47, 79, 79);
    pub const DARK_TURQUOISE: Color = Color::from_rgb_u8(0, 166, 166);
    pub const DARK_VIOLET: Color = Color::from_rgb_u8(148, 0, 211);
    pub const DEEP_PINK: Color = Color::from_rgb_u8(255, 20, 147);
    pub const DEEP_SKY_BLUE: Color = Color::from_rgb_u8(0, 191, 255);
    pub const DIM_GRAY: Color = Color::from_rgb_u8(84, 84, 84);
    pub const DODGER_BLUE: Color = Color::from_rgb_u8(30, 144, 255);
    pub const FIREBRICK: Color = Color::from_rgb_u8(142, 35, 35);
    pub const FLORAL_WHITE: Color = Color::from_rgb_u8(255, 250, 240);
    pub const FOREST_GREEN: Color = Color::from_rgb_u8(80, 159, 105);
    pub const GAINS_BORO: Color = Color::from_rgb_u8(220, 220, 220);
    pub const GHOST_WHITE: Color = Color::from_rgb_u8(248, 248, 255);
    pub const GOLD: Color = Color::from_rgb_u8(218, 170, 0);
    pub const GOLDENROD: Color = Color::from_rgb_u8(239, 223, 132);
    pub const GREEN: Color = Color::from_rgb_u8(0, 255, 0);
    pub const GREEN_YELLOW: Color = Color::from_rgb_u8(173, 255, 47);
    pub const HONEYDEW: Color = Color::from_rgb_u8(240, 255, 240);
    pub const HOT_PINK: Color = Color::from_rgb_u8(255, 105, 180);
    pub const INDIAN_RED: Color = Color::from_rgb_u8(107, 57, 57);
    pub const IVORY: Color = Color::from_rgb_u8(255, 255, 240);
    pub const KHAKI: Color = Color::from_rgb_u8(179, 179, 126);
    pub const LAVENDER: Color = Color::from_rgb_u8(230, 230, 250);
    pub const LAVENDER_BLUSH: Color = Color::from_rgb_u8(255, 240, 245);
    pub const LAWN_GREEN: Color = Color::from_rgb_u8(124, 252, 0);
    pub const LEMON_CHIFFON: Color = Color::from_rgb_u8(255, 250, 205);
    pub const LIGHT_BLUE: Color = Color::from_rgb_u8(176, 226, 255);
    pub const LIGHT_CORAL: Color = Color::from_rgb_u8(240, 128, 128);
    pub const LIGHT_CYAN: Color = Color::from_rgb_u8(224, 255, 255);
    pub const LIGHT_GOLDENROD: Color = Color::from_rgb_u8(238, 221, 130);
    pub const LIGHT_GOLDENROD_YELLOW: Color = Color::from_rgb_u8(250, 250, 210);
    pub const LIGHT_GRAY: Color = Color::from_rgb_u8(168, 168, 168);
    pub const LIGHT_PINK: Color = Color::from_rgb_u8(255, 182, 193);
    pub const LIGHT_SALMON: Color = Color::from_rgb_u8(255, 160, 122);
    pub const LIGHT_SEA_GREEN: Color = Color::from_rgb_u8(32, 178, 170);
    pub const LIGHT_SKY_BLUE: Color = Color::from_rgb_u8(135, 206, 250);
    pub const LIGHT_SLATE_BLUE: Color = Color::from_rgb_u8(132, 112, 255);
    pub const LIGHT_SLATE_GRAY: Color = Color::from_rgb_u8(119, 136, 153);
    pub const LIGHT_STEEL_BLUE: Color = Color::from_rgb_u8(124, 152, 211);
    pub const LIGHT_YELLOW: Color = Color::from_rgb_u8(255, 255, 224);
    pub const LIME_GREEN: Color = Color::from_rgb_u8(0, 175, 20);
    pub const LINEN: Color = Color::from_rgb_u8(250, 240, 230);
    pub const MAGENTA: Color = Color::from_rgb_u8(255, 0, 255);
    pub const MAROON: Color = Color::from_rgb_u8(143, 0, 82);
    pub const MEDIUM_AQUAMARINE: Color = Color::from_rgb_u8(0, 147, 143);
    pub const MEDIUM_BLUE: Color = Color::from_rgb_u8(50, 50, 204);
    pub const MEDIUM_FOREST_GREEN: Color = Color::from_rgb_u8(50, 129, 75);
    pub const MEDIUM_GOLDENROD: Color = Color::from_rgb_u8(209, 193, 102);
    pub const MEDIUM_ORCHID: Color = Color::from_rgb_u8(189, 82, 189);
    pub const MEDIUM_PURPLE: Color = Color::from_rgb_u8(147, 112, 219);
    pub const MEDIUM_SEA_GREEN: Color = Color::from_rgb_u8(52, 119, 102);
    pub const MEDIUM_SLATE_BLUE: Color = Color::from_rgb_u8(106, 106, 141);
    pub const MEDIUM_SPRING_GREEN: Color = Color::from_rgb_u8(35, 142, 35);
    pub const MEDIUM_TURQUOISE: Color = Color::from_rgb_u8(0, 210, 210);
    pub const MEDIUM_VIOLET_RED: Color = Color::from_rgb_u8(213, 32, 121);
    pub const MIDNIGHT_BLUE: Color = Color::from_rgb_u8(47, 47, 100);
    pub const MINT_CREAM: Color = Color::from_rgb_u8(245, 255, 250);
    pub const MISTY_ROSE: Color = Color::from_rgb_u8(255, 228, 225);
    pub const MOCCASIN: Color = Color::from_rgb_u8(255, 228, 181);
    pub const NAVAJO_WHITE: Color = Color::from_rgb_u8(255, 222, 173);
    pub const NAVY: Color = Color::from_rgb_u8(35, 35, 117);
    pub const NAVY_BLUE: Color = Color::from_rgb_u8(35, 35, 117);
    pub const OLD_LACE: Color = Color::from_rgb_u8(253, 245, 230);
    pub const OLIVE_DRAB: Color = Color::from_rgb_u8(107, 142, 35);
    pub const ORANGE: Color = Color::from_rgb_u8(255, 135, 0);
    pub const ORANGE_RED: Color = Color::from_rgb_u8(255, 69, 0);
    pub const ORCHID: Color = Color::from_rgb_u8(239, 132, 239);
    pub const PALE_GOLDENROD: Color = Color::from_rgb_u8(238, 232, 170);
    pub const PALE_GREEN: Color = Color::from_rgb_u8(115, 222, 120);
    pub const PALE_TURQUOISE: Color = Color::from_rgb_u8(175, 238, 238);
    pub const PALE_VIOLET_RED: Color = Color::from_rgb_u8(219, 112, 147);
    pub const PAPAYA_WHIP: Color = Color::from_rgb_u8(255, 239, 213);
    pub const PEACH_PUFF: Color = Color::from_rgb_u8(255, 218, 185);
    pub const PERU: Color = Color::from_rgb_u8(205, 133, 63);
    pub const PINK: Color = Color::from_rgb_u8(255, 181, 197);
    pub const PLUM: Color = Color::from_rgb_u8(197, 72, 155);
    pub const POWDER_BLUE: Color = Color::from_rgb_u8(176, 224, 230);
    pub const PURPLE: Color = Color::from_rgb_u8(160, 32, 240);
    pub const RED: Color = Color::from_rgb_u8(255, 0, 0);
    pub const ROSY_BROWN: Color = Color::from_rgb_u8(188, 143, 143);
    pub const ROYAL_BLUE: Color = Color::from_rgb_u8(65, 105, 225);
    pub const SADDLE_BROWN: Color = Color::from_rgb_u8(139, 69, 19);
    pub const SALMON: Color = Color::from_rgb_u8(233, 150, 122);
    pub const SANDY_BROWN: Color = Color::from_rgb_u8(244, 164, 96);
    pub const SEA_GREEN: Color = Color::from_rgb_u8(82, 149, 132);
    pub const SEA_SHELL: Color = Color::from_rgb_u8(255, 245, 238);
    pub const SIENNA: Color = Color::from_rgb_u8(150, 82, 45);
    pub const SKY_BLUE: Color = Color::from_rgb_u8(114, 159, 255);
    pub const SLATE_BLUE: Color = Color::from_rgb_u8(126, 136, 171);
    pub const SLATE_GRAY: Color = Color::from_rgb_u8(112, 128, 144);
    pub const SNOW: Color = Color::from_rgb_u8(255, 250, 250);
    pub const SPRING_GREEN: Color = Color::from_rgb_u8(65, 172, 65);
    pub const STEEL_BLUE: Color = Color::from_rgb_u8(84, 112, 170);
    pub const TAN: Color = Color::from_rgb_u8(222, 184, 135);
    pub const THISTLE: Color = Color::from_rgb_u8(216, 191, 216);
    pub const TOMATO: Color = Color::from_rgb_u8(255, 99, 71);
    pub const TRANSPARENT: Color = Color::new(0.0, 0.0, 0.0, 0.0);
    pub const TURQUOISE: Color = Color::from_rgb_u8(25, 204, 223);
    pub const VIOLET: Color = Color::from_rgb_u8(156, 62, 206);
    pub const VIOLET_RED: Color = Color::from_rgb_u8(243, 62, 150);
    pub const WHEAT: Color = Color::from_rgb_u8(245, 222, 179);
    pub const WHITE: Color = Color::from_rgb_u8(255, 255, 255);
    pub const WHITE_SMOKE: Color = Color::from_rgb_u8(245, 245, 245);
    pub const YELLOW: Color = Color::from_rgb_u8(255, 255, 0);
    pub const YELLOW_GREEN: Color = Color::from_rgb_u8(50, 216, 56);

    /// Gray scale colors from 0 (black) to 100 (white).
    pub const fn gray(level: u8) -> Color {
        let v = (level as f32 / 100.0 * 255.0) as u8;
        Color::from_rgb_u8(v, v, v)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_color_from_u32() {
        let c = Color::from_rgb_u32(0xFF8040);
        assert!((c.red - 1.0).abs() < 0.01);
        assert!((c.green - 0.502).abs() < 0.01);
        assert!((c.blue - 0.251).abs() < 0.01);
    }

    #[test]
    fn test_color_to_u8() {
        let c = Color::new(1.0, 0.5, 0.25, 1.0);
        let (r, g, b, a) = c.to_rgba_u8();
        assert_eq!(r, 255);
        assert_eq!(g, 127);
        assert_eq!(b, 63);
        assert_eq!(a, 255);
    }

    #[test]
    fn test_color_lerp() {
        let black = colors::BLACK;
        let white = colors::WHITE;
        let gray = black.lerp(white, 0.5);
        assert!((gray.red - 0.5).abs() < 0.01);
        assert!((gray.green - 0.5).abs() < 0.01);
        assert!((gray.blue - 0.5).abs() < 0.01);
    }
}
