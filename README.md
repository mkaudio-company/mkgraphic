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
│   └── button.rs       # Button widgets
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
- `fontdb` / `rustybuzz` - Font handling and text shaping
- `bitflags` - Modifier key flags

### Platform-specific
- **macOS**: `objc2`, `objc2-foundation`, `objc2-app-kit`
- **Windows**: `windows` crate with Win32 features
- **Linux**: `x11rb` for X11 support

## Usage

Add to your `Cargo.toml`:

```toml
[dependencies]
mkgraphic = "0.1.0"
```

### Basic Example

```rust
use mkgraphic::element::{label, button, vtile};
use mkgraphic::element::margin::margin;

// Create a simple UI
let ui = vtile(vec![
    Box::new(label("Hello, World!")),
    Box::new(margin(10.0, button("Click Me"))),
]);
```

### Layout Example

```rust
use mkgraphic::element::{htile, vtile};
use mkgraphic::element::align::{halign, valign};
use mkgraphic::element::size::fixed_size;

// Horizontal layout with centered content
let layout = htile(vec![
    Box::new(halign(0.5, label("Centered"))),
    Box::new(fixed_size(100.0, 50.0, button("Fixed Size"))),
]);
```

## Architecture

### Element Trait

All UI components implement the `Element` trait:

```rust
pub trait Element: Send + Sync + Any {
    fn limits(&self, ctx: &BasicContext) -> ViewLimits;
    fn draw(&self, ctx: &Context);
    fn click(&mut self, ctx: &Context, btn: MouseButton) -> bool;
    fn cursor(&mut self, ctx: &Context, p: Point, status: CursorTracking) -> bool;
    // ... more methods
}
```

### Layout System

- **VTile/HTile** - Vertical and horizontal stacking
- **Align** - Horizontal and vertical alignment (0.0 = start, 0.5 = center, 1.0 = end)
- **Margin** - Spacing around elements
- **Size** - Fixed, minimum, and maximum size constraints
- **Layer/Deck** - Stacked elements with z-ordering

### Context

The `Context` provides access to:
- View information (bounds, cursor position)
- Canvas for drawing
- Element hierarchy
- Enabled state

## Platform Support

| Platform | Backend | Status |
|----------|---------|--------|
| macOS | Cocoa/AppKit via objc2 | Basic |
| Windows | Win32 API | Basic |
| Linux | X11 via x11rb | Basic |

## Building

```bash
# Check compilation
cargo check

# Build
cargo build

# Build with release optimizations
cargo build --release
```

## License

MIT

## Acknowledgments

This project is a Rust translation of the [Elements](https://github.com/cycfi/elements) C++ GUI library by Joel de Guzman and Cycfi Research.
