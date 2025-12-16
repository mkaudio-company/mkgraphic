//! Font handling and text metrics.

use std::path::Path;

/// Font weight.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum FontWeight {
    Thin,
    ExtraLight,
    Light,
    #[default]
    Regular,
    Medium,
    SemiBold,
    Bold,
    ExtraBold,
    Black,
}

impl FontWeight {
    /// Returns the numeric weight value (100-900).
    pub fn value(&self) -> u16 {
        match self {
            FontWeight::Thin => 100,
            FontWeight::ExtraLight => 200,
            FontWeight::Light => 300,
            FontWeight::Regular => 400,
            FontWeight::Medium => 500,
            FontWeight::SemiBold => 600,
            FontWeight::Bold => 700,
            FontWeight::ExtraBold => 800,
            FontWeight::Black => 900,
        }
    }
}

/// Font style (normal or italic).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum FontStyle {
    #[default]
    Normal,
    Italic,
    Oblique,
}

/// Font stretch.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum FontStretch {
    UltraCondensed,
    ExtraCondensed,
    Condensed,
    SemiCondensed,
    #[default]
    Normal,
    SemiExpanded,
    Expanded,
    ExtraExpanded,
    UltraExpanded,
}

/// A font descriptor.
#[derive(Debug, Clone)]
pub struct Font {
    family: String,
    weight: FontWeight,
    style: FontStyle,
    stretch: FontStretch,
}

impl Font {
    /// Creates a new font with the given family name.
    pub fn new(family: impl Into<String>) -> Self {
        Self {
            family: family.into(),
            weight: FontWeight::default(),
            style: FontStyle::default(),
            stretch: FontStretch::default(),
        }
    }

    /// Creates a font with the default sans-serif family.
    pub fn sans_serif() -> Self {
        Self::new("sans-serif")
    }

    /// Creates a font with the default serif family.
    pub fn serif() -> Self {
        Self::new("serif")
    }

    /// Creates a font with a monospace family.
    pub fn monospace() -> Self {
        Self::new("monospace")
    }

    /// Returns the font family name.
    pub fn family(&self) -> &str {
        &self.family
    }

    /// Returns the font weight.
    pub fn weight(&self) -> FontWeight {
        self.weight
    }

    /// Sets the font weight.
    pub fn with_weight(mut self, weight: FontWeight) -> Self {
        self.weight = weight;
        self
    }

    /// Returns the font style.
    pub fn style(&self) -> FontStyle {
        self.style
    }

    /// Sets the font style.
    pub fn with_style(mut self, style: FontStyle) -> Self {
        self.style = style;
        self
    }

    /// Returns the font stretch.
    pub fn stretch(&self) -> FontStretch {
        self.stretch
    }

    /// Sets the font stretch.
    pub fn with_stretch(mut self, stretch: FontStretch) -> Self {
        self.stretch = stretch;
        self
    }

    /// Returns a bold variant of this font.
    pub fn bold(self) -> Self {
        self.with_weight(FontWeight::Bold)
    }

    /// Returns an italic variant of this font.
    pub fn italic(self) -> Self {
        self.with_style(FontStyle::Italic)
    }
}

impl Default for Font {
    fn default() -> Self {
        Self::sans_serif()
    }
}

/// A font database for managing loaded fonts.
pub struct FontDatabase {
    db: fontdb::Database,
}

impl FontDatabase {
    /// Creates a new empty font database.
    pub fn new() -> Self {
        Self {
            db: fontdb::Database::new(),
        }
    }

    /// Creates a font database with system fonts loaded.
    pub fn with_system_fonts() -> Self {
        let mut db = fontdb::Database::new();
        db.load_system_fonts();
        Self { db }
    }

    /// Loads a font from a file.
    pub fn load_font_file(&mut self, path: impl AsRef<Path>) -> Result<(), std::io::Error> {
        let data = std::fs::read(path)?;
        self.db.load_font_data(data);
        Ok(())
    }

    /// Loads a font from memory.
    pub fn load_font_data(&mut self, data: Vec<u8>) {
        self.db.load_font_data(data);
    }

    /// Returns the number of loaded font faces.
    pub fn len(&self) -> usize {
        self.db.len()
    }

    /// Returns true if no fonts are loaded.
    pub fn is_empty(&self) -> bool {
        self.db.is_empty()
    }

    /// Returns the underlying fontdb database.
    pub fn inner(&self) -> &fontdb::Database {
        &self.db
    }

    /// Returns a mutable reference to the underlying fontdb database.
    pub fn inner_mut(&mut self) -> &mut fontdb::Database {
        &mut self.db
    }
}

impl Default for FontDatabase {
    fn default() -> Self {
        Self::with_system_fonts()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_font_creation() {
        let font = Font::new("Helvetica").bold().italic();
        assert_eq!(font.family(), "Helvetica");
        assert_eq!(font.weight(), FontWeight::Bold);
        assert_eq!(font.style(), FontStyle::Italic);
    }

    #[test]
    fn test_font_weight_values() {
        assert_eq!(FontWeight::Regular.value(), 400);
        assert_eq!(FontWeight::Bold.value(), 700);
    }
}
