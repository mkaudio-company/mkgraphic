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

pub mod element;
pub mod host;
pub mod support;
pub mod view;

/// Prelude module for convenient imports
pub mod prelude {
    pub use crate::element::{
        align::*,
        button::{button, BasicButton},
        checkbox::{checkbox, radio_button, Checkbox, RadioButton, RadioGroup},
        code_editor::{code_editor, CodeEditor, Diagnostic, DiagnosticSeverity},
        composite::{Composite, CompositeBase},
        context::{BasicContext, Context},
        design_canvas::{design_canvas, DesignCanvas},
        dial::{dial, dial_with_range, Dial},
        floating::{floating, Floating},
        grid::{grid, Grid},
        label::{label, Label},
        layer::*,
        list::{dropdown, list, Dropdown, List, ListItem},
        margin::*,
        menu::{
            get_native_menu_bar, menu, menu_item, menu_separator, native_menu, native_menu_bar,
            native_menu_item, native_separator, popup, set_native_menu_bar, Menu, MenuItem,
            MenuModifiers, MenuShortcut, NativeMenu, NativeMenuBar, NativeMenuItem, Popup,
            StandardAction,
        },
        progress::{
            circular_progress, indeterminate_progress, progress_bar, ProgressBar, ProgressStyle,
        },
        proxy::Proxy,
        scroll::{scroll_view, ScrollView},
        share,
        size::*,
        slider::{slider, vslider, Slider, SliderOrientation},
        splitter::{splitter, vsplitter, Splitter, SplitterOrientation},
        status_bar::{status_bar, StatusBar, StatusSegment},
        switch::{slide_switch, SlideSwitch},
        tabs::{tab_bar, Tab, TabBar},
        text_box::{text_box, TextBox},
        thumbwheel::{thumbwheel, Thumbwheel},
        tile::{htile, vtile, HTile, VTile},
        tooltip::{tooltip, Tooltip},
        tree::{tree_node, tree_view, TreeNode, TreeView},
        Element, ElementPtr, ViewLimits, ViewStretch, WeakElementPtr,
    };
    pub use crate::host::{App, CloseBehavior, Window, WindowBuilder, WindowStyle};
    #[cfg(target_os = "macos")]
    pub use crate::host::{choose_file_to_open, choose_file_to_save, choose_folder};
    pub use crate::support::{
        canvas::Canvas,
        color::{colors, Color},
        point::{Axis, Extent, Point},
        rect::Rect,
    };
    pub use crate::view::{
        BaseView, CursorTracking, CursorType, DropInfo, KeyAction, KeyCode, KeyInfo, MouseButton,
        MouseButtonState, TextInfo, View,
    };
    pub use crate::{htile, vtile};
}
