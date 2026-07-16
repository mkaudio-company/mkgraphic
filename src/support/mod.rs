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

pub mod canvas;
pub mod circle;
pub mod color;
pub mod font;
pub mod markdown;
pub mod math;
pub mod payload;
pub mod point;
pub mod rect;
pub mod theme;

pub use canvas::Canvas;
pub use circle::Circle;
pub use color::Color;
pub use font::Font;
pub use point::{Axis, Extent, Point};
pub use rect::Rect;
pub use theme::Theme;
