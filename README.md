[![](https://img.shields.io/crates/v/mkgraphic.svg)](https://crates.io/crates/mkgraphic)
[![](https://img.shields.io/crates/l/mkgraphic.svg)](https://crates.io/crates/mkgraphic)
[![](https://docs.rs/mkgraphic/badge.svg)](https://docs.rs/mkgraphic/)

# mkgraphic

A Rust port of the [cycfi/elements](https://github.com/cycfi/elements) C++ GUI framework.

## Overview

mkgraphic is a lightweight, modular GUI framework for Rust that provides an element-based architecture for building user interfaces. It follows the design principles of the original Elements library while leveraging Rust's safety guarantees and modern ecosystem.

## Features

- **Element-based architecture** - Composable UI elements with a hierarchical tree structure
- **Pure Rust graphics** - Uses tiny-skia for 2D rendering (no C++ dependencies for graphics)
- **Cross-platform** - Native platform integration for macOS, Windows, and Linux
- **Layout system** - Flexible layouts with tiles, alignment, margins, and size constraints
- **Theming** - Built-in support for dark and light themes
- **Event handling** - Mouse, keyboard, focus, and drag-and-drop support
- **Text rendering** - Full text shaping with rustybuzz and proper text measurement

## Widgets

- **Label** - Text display with customizable font, color, and alignment
- **Button** - Clickable button with hover and pressed states
- **TextBox** - Single-line text input with cursor, selection, and clipboard support
- **Slider** - Horizontal/vertical value slider with customizable track and thumb
- **Dial** - Rotary knob control with angular mouse interaction
- **Checkbox** - Toggle checkbox with label
- **RadioButton** - Radio button for exclusive selection
- **List** - Scrollable list with single/multiple selection
- **Dropdown** - Dropdown menu selection
- **Menu** - Context/popup menu with items and separators
- **ScrollView** - Scrollable container with horizontal/vertical scrollbars

## Project Structure

```
src/
├── lib.rs              # Library entry point
├── support/            # Core utilities
│   ├── point.rs        # Point, Extent, Axis types
│   ├── rect.rs         # Rectangle geometry
│   ├── color.rs        # RGBA colors
│   ├── canvas.rs       # 2D drawing abstraction
│   ├── font.rs         # Font handling
│   └── theme.rs        # Theming system
├── element/            # UI element system
│   ├── mod.rs          # Element trait
│   ├── context.rs      # Render/event context
│   ├── composite.rs    # Container elements
│   ├── tile.rs         # VTile/HTile layouts
│   ├── align.rs        # Alignment elements
│   ├── margin.rs       # Margin elements
│   ├── size.rs         # Size constraints
│   ├── layer.rs        # Layer/Deck stacking
│   ├── label.rs        # Text labels
│   ├── button.rs       # Button widgets
│   ├── text_box.rs     # Text input
│   ├── slider.rs       # Slider control
│   ├── dial.rs         # Rotary dial/knob
│   ├── checkbox.rs     # Checkbox and radio buttons
│   ├── list.rs         # List selection
│   ├── dropdown.rs     # Dropdown selection
│   ├── menu.rs         # Context menus
│   └── scroll.rs       # Scroll view
├── view/               # View management
│   └── mod.rs          # Events and input handling
└── host/               # Platform layer
    ├── macos.rs        # macOS (objc2)
    ├── windows.rs      # Windows (Win32)
    └── linux.rs        # Linux (X11)
```

## Dependencies

### Core
- `tiny-skia` - Pure Rust 2D graphics
- `fontdb` / `rustybuzz` / `ttf-parser` - Font handling and text shaping
- `bitflags` - Modifier key flags

### Platform-specific
- **macOS**: `objc2`, `objc2-foundation`, `objc2-app-kit`
- **Windows**: `windows` crate with Win32 features
- **Linux**: `x11rb` for X11 support

## Usage

Add to your `Cargo.toml`:

```toml
[dependencies]
mkgraphic = "0.2"
```

### Basic Example

```rust
use mkgraphic::element::{label, button, vtile, share};
use mkgraphic::element::margin::margin;

// Create a simple UI
let ui = vtile![
    label("Hello, World!"),
    margin(10.0, button("Click Me").on_click(|| println!("Clicked!"))),
];
```

### Interactive Widgets Example

```rust
use mkgraphic::element::{vtile, htile, share};
use mkgraphic::element::text_box::text_box;
use mkgraphic::element::slider::slider;
use mkgraphic::element::dial::dial;

// Create interactive controls
let ui = vtile![
    text_box()
        .placeholder("Enter text...")
        .on_change(|text| println!("Text: {}", text)),
    htile![
        slider().on_change(|v| println!("Slider: {:.2}", v)),
        dial().on_change(|v| println!("Dial: {:.2}", v)),
    ],
];
```

### Layout Example

```rust
use mkgraphic::element::{htile, vtile, share};
use mkgraphic::element::align::{halign, valign};
use mkgraphic::element::size::fixed_size;
use mkgraphic::element::label::label;
use mkgraphic::element::button::button;

// Horizontal layout with centered content
let layout = htile![
    halign(0.5, label("Centered")),
    fixed_size(100.0, 50.0, button("Fixed Size")),
];
```

## Architecture

### Element Trait

All UI components implement the `Element` trait:

```rust
pub trait Element: Send + Sync + Any {
    fn limits(&self, ctx: &BasicContext) -> ViewLimits;
    fn draw(&self, ctx: &Context);
    fn hit_test(&self, ctx: &Context, p: Point, leaf: bool, control: bool) -> Option<&dyn Element>;
    fn handle_click(&self, ctx: &Context, btn: MouseButton) -> bool;
    fn handle_drag(&self, ctx: &Context, btn: MouseButton);
    fn handle_key(&self, ctx: &Context, k: KeyInfo) -> bool;
    fn handle_text(&self, ctx: &Context, info: TextInfo) -> bool;
    fn handle_scroll(&self, ctx: &Context, dir: Point, p: Point) -> bool;
    // ... more methods
}
```

### Layout System

- **VTile/HTile** - Vertical and horizontal stacking
- **Align** - Horizontal and vertical alignment (0.0 = start, 0.5 = center, 1.0 = end)
- **Margin** - Spacing around elements
- **Size** - Fixed, minimum, and maximum size constraints
- **Stretch** - Control how elements expand to fill available space
- **Layer/Deck** - Stacked elements with z-ordering

### Context

The `Context` provides access to:
- View information (bounds, cursor position)
- Canvas for drawing
- Element hierarchy
- Enabled state

### Focus Management

Elements can receive keyboard focus through the focus system:
- `wants_focus()` - Whether the element can receive focus
- `begin_focus()` / `end_focus()` - Focus lifecycle
- `clear_focus()` - Clears focus from all elements (used when clicking elsewhere)

## Platform Support

| Platform | Backend | Status |
|----------|---------|--------|
| macOS | Cocoa/AppKit via objc2 | Working |
| Windows | Win32 API | Basic |
| Linux | X11 via x11rb | Basic |

## Examples

Run the elements gallery to see all available widgets:

```bash
cargo run --example elements_gallery
```

## Building

```bash
# Check compilation
cargo check

# Build
cargo build

# Build with release optimizations
cargo build --release

# Run tests
cargo test
```

## License

MIT

## Acknowledgments

This project is a Rust translation of the [Elements](https://github.com/cycfi/elements) C++ GUI library by Joel de Guzman and Cycfi Research.
