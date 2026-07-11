[![](https://img.shields.io/crates/v/mkgraphic.svg)](https://crates.io/crates/mkgraphic)
[![](https://img.shields.io/crates/l/mkgraphic.svg)](https://crates.io/crates/mkgraphic)
[![](https://docs.rs/mkgraphic/badge.svg)](https://docs.rs/mkgraphic/)

# mkgraphic

A Rust GUI framework that started as a port of the
[cycfi/elements](https://github.com/cycfi/elements) C++ framework and has since grown past it: alongside the original element/layout models, and the ability to embed externally-managed native content in an mkgraphic-owned window.

## Overview

mkgraphic is a lightweight, modular GUI framework for Rust that provides an element-based architecture for building user interfaces. It follows the design principles of the original Elements library while leveraging Rust's safety guarantees and modern ecosystem - and where a use case calls for something the original didn't have (a real code editor, a free-form design surface, native interop), mkgraphic adds it as its own primitive rather than staying a strict port.

## Features

- **Element-based architecture** - Composable UI elements with a hierarchical tree structure
- **Pure Rust graphics** - Uses tiny-skia for 2D rendering (no C++ dependencies for graphics)
- **Cross-platform** - Native platform integration for macOS, Windows, and Linux
- **Layout system** - Flexible layouts with tiles, alignment, margins, and size constraints
- **Theming** - Built-in support for dark and light themes
- **Event handling** - Mouse, keyboard, focus, and drag-and-drop support
- **Text rendering** - Full text shaping with rustybuzz and proper text measurement
- **Code editing** - Multi-line editor with a line-number gutter, undo/redo, and tree-sitter syntax highlighting
- **Visual design surface** - Free-form canvas for absolute positioning, drag-to-move/resize, and edge/sibling snap guides
- **Native window embedding** - Get the platform's real window handle (e.g. `NSWindow*` on macOS) to host externally-managed content alongside mkgraphic's own element tree

## Widgets

- **Label** - Text display with customizable font, color, and alignment
- **Button** - Clickable button with hover and pressed states
- **TextBox** - Single-line text input with cursor, selection, and clipboard support
- **Slider** - Horizontal/vertical value slider with customizable track and thumb
- **Dial** - Rotary knob control with angular mouse interaction
- **Checkbox** - Toggle checkbox with label
- **RadioButton** - Radio button for exclusive selection
- **SlideSwitch** - iOS-style toggle switch
- **Thumbwheel** - Scrollable value wheel control
- **ProgressBar** - Linear and circular progress indicators
- **List** - Scrollable list with single/multiple selection
- **Dropdown** - Dropdown menu selection
- **TabBar** - Tab-based navigation
- **ScrollView** - Scrollable container with horizontal/vertical scrollbars
- **Tooltip** - Hover tooltips for elements
- **StatusBar** - Status bar with segments
- **Grid** - Grid layout container
- **NativeMenuBar** - Native OS menu bar integration
- **CodeEditor** - Multi-line code editor with line numbers, undo/redo, and tree-sitter syntax highlighting (Rust grammar out of the box)
- **DesignCanvas** - Free-form container for absolute-positioned children with click-to-select, drag-to-move, corner/edge resize handles, and snap guides against sibling edges

## Project Structure

```
src/
├── lib.rs              # Library entry point
├── support/            # Core utilities
│   ├── point.rs        # Point, Extent, Axis types
│   ├── rect.rs         # Rectangle geometry
│   ├── circle.rs       # Circle geometry
│   ├── color.rs        # RGBA colors
│   ├── canvas.rs       # 2D drawing abstraction
│   ├── font.rs         # Font handling
│   ├── theme.rs        # Theming system
│   └── payload.rs      # Drag-and-drop payload data
├── element/            # UI element system
│   ├── mod.rs          # Element trait
│   ├── context.rs      # Render/event context
│   ├── proxy.rs        # Proxy elements (wrap/delegate to a subject)
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
│   ├── switch.rs       # Toggle switches
│   ├── thumbwheel.rs   # Thumbwheel control
│   ├── progress.rs     # Progress indicators
│   ├── list.rs         # List and dropdown
│   ├── menu.rs         # Menus and native menu bar
│   ├── tabs.rs         # Tab bar
│   ├── tooltip.rs      # Tooltips
│   ├── status_bar.rs   # Status bar
│   ├── grid.rs         # Grid layout
│   ├── floating.rs     # Floating elements
│   ├── scroll.rs       # Scroll view
│   ├── code_editor.rs  # Multi-line code editor + tree-sitter highlighting
│   └── design_canvas.rs # Free-form visual layout surface
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
- `tree-sitter` / `tree-sitter-rust` / `streaming-iterator` - Incremental parsing and syntax highlighting for `CodeEditor` (Rust grammar bundled; the parsing setup is generic enough to add other languages' grammars later)

### Platform-specific
- **macOS**: `objc2`, `objc2-foundation`, `objc2-app-kit`
- **Windows**: `windows` crate with Win32 features
- **Linux**: `x11rb` for X11 support

## Usage

Add to your `Cargo.toml`:

```toml
[dependencies]
mkgraphic = "0.3"
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

### Native Menu Bar Example

```rust
use mkgraphic::prelude::*;

fn main() {
    // Configure menu bar before creating app
    set_native_menu_bar(
        native_menu_bar()
            .app_name("My App")
            .add_menu(native_menu("File")
                .add_item(native_menu_item("New")
                    .shortcut_cmd('n')
                    .on_select(|| println!("New")))
                .add_item(native_menu_item("Open...")
                    .shortcut_cmd('o'))
                .add_item(native_separator())
                .add_item(native_menu_item("Save")
                    .shortcut_cmd('s')))
            .add_menu(native_menu("View")
                .add_item(native_menu_item("Zoom In")
                    .shortcut_cmd('+'))
                .add_item(native_menu_item("Zoom Out")
                    .shortcut_cmd('-')))
            .include_app_menu(true)
            .include_edit_menu(true)
            .include_window_menu(true)
    );

    let mut app = App::new();
    // ... create windows
    app.run();
}
```

### Code Editor Example

```rust
use mkgraphic::element::{code_editor, margin, share};

let editor = margin(5.0, code_editor()
    .width(420.0)
    .height(240.0)
    .text("fn main() {\n    println!(\"Hello!\");\n}\n")
    .on_change(|text| println!("Buffer changed ({} bytes)", text.len())));
```

Ships with Rust highlighting; undo/redo are wired to Cmd/Ctrl-Z and
Cmd/Ctrl-Shift-Z (or Ctrl-Y) out of the box.

### Design Canvas Example

```rust
use mkgraphic::element::{button, design_canvas, label};
use mkgraphic::support::rect::Rect;

let mut canvas = design_canvas(420.0, 240.0)
    .on_selection_changed(|index| println!("Selected: {:?}", index))
    .on_layout_changed(|| println!("Layout changed"));

canvas.add_child(button("Gain").on_click(|| println!("Gain clicked")), Rect::new(20.0, 20.0, 140.0, 60.0));
canvas.add_child(label("Output"), Rect::new(160.0, 20.0, 260.0, 60.0));
```

Children are positioned absolutely (not flow-laid-out); drag a child to
move it, drag a corner/edge handle to resize it, and dragging near a
sibling's edge snaps to it with a guide line.

### Native Window Embedding Example

```rust
use mkgraphic::prelude::*;

let window = Window::new("Host Window", Extent::new(800.0, 600.0));

// Real platform pointer (NSWindow* on macOS) for attaching content another
// library manages itself, instead of mkgraphic's own element tree.
if let Some(handle) = window.handle() {
    // e.g. hand `handle` to a library that wants an NSWindow* to attach
    // its own NSView-backed content to.
}
```

`Window::handle()` no longer always returns `None` - it returns the real
native window handle so an mkgraphic-created window can host content that
another library renders and manages itself. mkgraphic still owns the
window's lifetime either way.

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

A minimal starter example:

```bash
cargo run --example hello
```

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

## Packaging

A `cargo xtask` (see [xtask/](xtask/)) turns a release build of any example
(or your own app depending on mkgraphic in the same style) into a
distributable package.

### macOS: signed .app bundle

```bash
# One square source PNG (1024x1024 recommended) -> AppIcon.icns + icon.ico
cargo xtask make-icons --source path/to/icon.png --out-dir path/to/icons

cargo xtask bundle-mac \
    --example elements_gallery \
    --icon path/to/icons/AppIcon.icns \
    --name "Elements Gallery" \
    --identity "Developer ID Application: Your Name (TEAMID)"
```

Produces `target/bundle/Elements Gallery.app`. `--identity` is a signing
identity from **your own** Keychain (`security find-identity -v -p codesigning`
lists what's available) - this tool never hardcodes or assumes anyone's
identity, since every user packaging their own app needs to sign with their
own certificate. Omit `--identity` to fall back to ad-hoc signing, which runs
locally but won't pass Gatekeeper if you distribute the app to another Mac.

### Windows: MSI installer

Must run on Windows, with the [WiX Toolset v3](https://wixtoolset.org/)
(`candle`/`light`) on `PATH`:

```powershell
cargo xtask bundle-windows `
    --example elements_gallery `
    --icon path/to/icons/icon.ico `
    --name "Elements Gallery" `
    --upgrade-code "<a GUID you generate once and keep for this app>"
```

Embeds the icon into the .exe (via a `build.rs` + `winres`, gated behind the
`MKGRAPHIC_APP_ICON` env var so a plain `cargo build` is unaffected) and
produces `target/bundle/Elements Gallery.msi`. Generate the upgrade code once
per app (e.g. `[guid]::NewGuid()` in PowerShell) and keep it constant across
releases - it's what lets the MSI upgrade a previous install instead of
conflicting with it.

## License

MIT

## Acknowledgments

This project began as a Rust translation of the [Elements](https://github.com/cycfi/elements) C++ GUI library by Joel de Guzman and Cycfi Research, and its core element/layout model still follows Elements' design. It has since grown beyond a strict port - the code editor, design canvas, and native window embedding described above have no equivalent in the original library and are mkgraphic's own additions.

Also uses [tree-sitter](https://github.com/tree-sitter/tree-sitter) and its [Rust grammar](https://github.com/tree-sitter/tree-sitter-rust) for `CodeEditor`'s syntax highlighting.
