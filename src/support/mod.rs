//! Support module containing fundamental types and utilities.
//!
//! This module provides the core types used throughout the library:
//!
//! - [`point`]: Points, extents, and axis types
//! - [`rect`]: Rectangle representation and operations
//! - [`color`]: Color representation with common color constants
//! - [`circle`]: Circle representation
//! - [`canvas`]: 2D drawing context abstraction
//! - [`font`]: Font handling and text metrics
//! - [`theme`]: Theming and styling constants

pub mod point;
pub mod rect;
pub mod color;
pub mod circle;
pub mod canvas;
pub mod font;
pub mod theme;
pub mod payload;

pub use point::{Point, Extent, Axis};
pub use rect::Rect;
pub use color::Color;
pub use circle::Circle;
pub use canvas::Canvas;
pub use font::Font;
pub use theme::Theme;
