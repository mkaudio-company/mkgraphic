//! # MKGraphic
//!
//! A Rust port of the [cycfi/elements](https://github.com/cycfi/elements) C++ GUI framework.
//!
//! MKGraphic provides a lightweight, fine-grained, resolution-independent, modular GUI library.
//! The library is designed to be:
//!
//! - **Lightweight**: Elements are light-weight objects with minimal memory footprint
//! - **Composable**: Elements can be combined and nested to create complex UIs
//! - **Resolution-independent**: Automatically adapts to different screen densities
//! - **Cross-platform**: Supports macOS, Windows, and Linux
//!
//! ## Architecture
//!
//! The library is organized into several main modules:
//!
//! - [`support`]: Core types like Point, Rect, Color, and Canvas
//! - [`element`]: The Element trait and base element types
//! - [`view`]: Window and view management
//! - [`host`]: Platform-specific implementations
//!
//! ## Example
//!
//! ```rust,no_run
//! use mkgraphic::prelude::*;
//!
//! fn main() {
//!     let mut app = App::new();
//!     let mut window = Window::new("Hello MKGraphic", Extent::new(800.0, 600.0));
//!
//!     let content = vtile![
//!         label("Hello, World!"),
//!         button("Click me!").on_click(|| println!("Clicked!")),
//!     ];
//!
//!     window.set_content(share(content));
//!     window.show();
//!     app.run();
//! }
//! ```

#![allow(dead_code)]
#![allow(unused_variables)]

pub mod support;
pub mod element;
pub mod view;
pub mod host;

/// Prelude module for convenient imports
pub mod prelude {
    pub use crate::support::{
        point::{Point, Extent, Axis},
        rect::Rect,
        color::{Color, colors},
        canvas::Canvas,
    };
    pub use crate::element::{
        Element, ElementPtr, WeakElementPtr,
        ViewLimits, ViewStretch,
        share,
        context::{BasicContext, Context},
        proxy::Proxy,
        composite::{Composite, CompositeBase},
        tile::{vtile, htile, VTile, HTile},
        align::*,
        margin::*,
        size::*,
        layer::*,
        label::{label, Label},
        button::{button, BasicButton},
    };
    pub use crate::view::{
        View, BaseView,
        MouseButton, MouseButtonState,
        KeyCode, KeyAction, KeyInfo,
        CursorTracking, CursorType,
        TextInfo, DropInfo,
    };
    pub use crate::host::{App, Window};
    pub use crate::{vtile, htile};
}
