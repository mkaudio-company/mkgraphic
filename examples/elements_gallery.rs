//! Elements Gallery - Showcase of all UI elements
//!
//! This example demonstrates all the new UI elements added to mkgraphic.

use mkgraphic::prelude::*;

fn main() {
    // Configure native menu bar before creating the app
    setup_menu_bar();

    let mut app = App::new();
    let mut window = Window::new("Elements Gallery", Extent::new(900.0, 700.0));

    // Create the main content
    let content = vtile![
        margin(
            10.0,
            label("MKGraphic Elements Gallery").with_font_size(24.0)
        ),
        margin(10.0, create_gallery()),
    ];

    // The gallery is taller (and, with the design canvas able to grow
    // wider than its constructed size now, sometimes wider) than any
    // reasonable window -- wrapped in a `ScrollView` so everything past the
    // window's edge is still reachable instead of simply cut off. No
    // `.content_size()` call: `ScrollView` auto-sizes to the content's own
    // real layout size unless told otherwise, so this tracks the gallery's
    // actual content instead of a hand-maintained guess.
    // `Auto` (both axes) is `ScrollView`'s own default -- no need to name
    // `ScrollbarVisibility` here just to ask for what it already does.
    let scrollable = scroll_view().content(content).size(900.0, 700.0);

    window.set_content(share(scrollable));
    window.show();
    app.run();
}

fn create_gallery() -> impl Element {
    vtile![create_controls_gallery(), create_editor_gallery(),]
}

fn create_controls_gallery() -> impl Element {
    let radio_group = RadioGroup::new();

    htile![
        // Left column - Basic controls
        margin(
            10.0,
            vtile![
                section_label("Sliders"),
                margin(5.0, slider().on_change(|v| println!("Slider: {:.2}", v))),
                margin(
                    5.0,
                    slider()
                        .value(0.5)
                        .on_change(|v| println!("Slider 2: {:.2}", v))
                ),
                margin(
                    5.0,
                    htile![
                        vslider().on_change(|v| println!("VSlider: {:.2}", v)),
                        vslider()
                            .value(0.75)
                            .on_change(|v| println!("VSlider 2: {:.2}", v)),
                    ]
                ),
                section_label("Checkboxes"),
                margin(
                    5.0,
                    checkbox("Option 1").on_change(|checked| println!("Checkbox 1: {}", checked))
                ),
                margin(
                    5.0,
                    checkbox("Option 2")
                        .checked(true)
                        .on_change(|checked| println!("Checkbox 2: {}", checked))
                ),
                section_label("Radio Buttons"),
                margin(
                    5.0,
                    radio_button("Choice A")
                        .group(&radio_group, 0)
                        .selected(true)
                        .on_select(|| println!("Radio A selected"))
                ),
                margin(
                    5.0,
                    radio_button("Choice B")
                        .group(&radio_group, 1)
                        .on_select(|| println!("Radio B selected"))
                ),
                section_label("Toggle Switches"),
                margin(
                    5.0,
                    htile![
                        label("Dark Mode"),
                        slide_switch().on_change(|on| println!("Switch: {}", on)),
                    ]
                ),
                margin(
                    5.0,
                    htile![
                        label("Notifications"),
                        slide_switch()
                            .on(true)
                            .on_change(|on| println!("Notifications: {}", on)),
                    ]
                ),
            ]
        ),
        // Middle column - Value controls
        margin(
            10.0,
            vtile![
                section_label("Dials"),
                margin(
                    5.0,
                    htile![
                        dial().on_change(|v| println!("Dial 1: {:.2}", v)),
                        dial_with_range(0.0, 100.0)
                            .value(50.0)
                            .on_change(|v| println!("Dial 2: {:.1}", v)),
                    ]
                ),
                section_label("Text Input"),
                margin(
                    5.0,
                    text_box()
                        .placeholder("Enter your name...")
                        .on_change(|text| println!("Text: {}", text))
                ),
                margin(5.0, text_box().placeholder("Password").password(true)),
                section_label("Progress Bars"),
                margin(5.0, progress_bar().value(0.3)),
                margin(5.0, progress_bar().value(0.7).show_percentage(true)),
                margin(
                    5.0,
                    htile![
                        circular_progress().value(0.5),
                        circular_progress().value(0.75).show_percentage(true),
                    ]
                ),
                section_label("Thumbwheel"),
                margin(
                    5.0,
                    thumbwheel().on_change(|v| println!("Thumbwheel: {:.2}", v))
                ),
            ]
        ),
        // Right column - Lists and selections
        margin(
            10.0,
            vtile![
                section_label("Dropdown"),
                margin(
                    5.0,
                    dropdown()
                        .items(vec!["Option 1", "Option 2", "Option 3"])
                        .on_select(|idx| println!("Selected: {}", idx))
                ),
                section_label("List"),
                margin(
                    5.0,
                    fixed_size(
                        200.0,
                        120.0,
                        list()
                            .items(vec![
                                ListItem::new("Item 1"),
                                ListItem::new("Item 2"),
                                ListItem::new("Item 3"),
                                ListItem::new("Item 4"),
                                ListItem::new("Item 5"),
                            ])
                            .on_select(|idx| println!("List selected: {}", idx))
                    )
                ),
                section_label("Buttons"),
                margin(
                    5.0,
                    button("Primary Button").on_click(|| println!("Primary clicked!"))
                ),
                margin(
                    5.0,
                    button("Secondary Button").on_click(|| println!("Secondary clicked!"))
                ),
            ]
        ),
    ]
}

fn create_editor_gallery() -> impl Element {
    htile![
        margin(
            10.0,
            vtile![
                // `section_label` is a plain `Label` with no line-wrapping,
                // so its reported min-width is its *entire* text at a
                // fixed chars-per-point estimate -- keep these short. A
                // long enough caption here previously bloated this whole
                // page's computed content width well past what it visually
                // needed (confirmed: ~89 characters alone pushed it from
                // ~900pt to ~1229pt), distorting how much space every
                // stretchy column absorbed.
                section_label("Code Editor (tree-sitter)"),
                margin(
                    5.0,
                    code_editor()
                        .width(420.0)
                        .height(240.0)
                        .text(SAMPLE_RUST_SNIPPET)
                        .on_change(|text| println!("Code editor changed ({} bytes)", text.len()))
                ),
            ]
        ),
        margin(
            10.0,
            vtile![
                // Full instructions: drag to move, corner/edge handles to
                // resize, drag near a sibling to snap. Kept short here --
                // see the `Code Editor` section label's comment for why.
                section_label("Design Canvas"),
                margin(5.0, create_design_canvas_demo()),
            ]
        ),
    ]
}

const SAMPLE_RUST_SNIPPET: &str = r#"// Sample file loaded into the code editor.
use mkgraphic::prelude::*;

fn greet(name: &str) -> String {
    format!("Hello, {name}!")
}

struct Counter {
    value: i32,
}

impl Counter {
    fn increment(&mut self) {
        self.value += 1;
    }
}
"#;

fn create_design_canvas_demo() -> impl Element {
    let canvas = design_canvas(420.0, 240.0)
        .on_selection_changed(|index| println!("Design canvas selection: {:?}", index))
        .on_layout_changed(|| println!("Design canvas layout changed"));

    canvas.add_child(
        button("Gain").on_click(|| println!("Gain knob placeholder clicked")),
        Rect::new(20.0, 20.0, 140.0, 60.0),
    );
    canvas.add_child(label("Output"), Rect::new(160.0, 20.0, 260.0, 60.0));
    canvas.add_child(
        button("Bypass").on_click(|| println!("Bypass placeholder clicked")),
        Rect::new(20.0, 100.0, 140.0, 140.0),
    );

    canvas
}

fn section_label(text: &str) -> impl Element {
    margin_top(15.0, margin_bottom(5.0, label(text).with_font_size(14.0)))
}

fn setup_menu_bar() {
    // Configure the native menu bar with custom menus
    set_native_menu_bar(
        native_menu_bar()
            .app_name("Elements Gallery")
            // Add a custom File menu
            .add_menu(
                native_menu("File")
                    .add_item(
                        native_menu_item("New")
                            .shortcut_cmd('n')
                            .on_select(|| println!("File > New")),
                    )
                    .add_item(
                        native_menu_item("Open...")
                            .shortcut_cmd('o')
                            .on_select(|| println!("File > Open")),
                    )
                    .add_item(native_separator())
                    .add_item(
                        native_menu_item("Save")
                            .shortcut_cmd('s')
                            .on_select(|| println!("File > Save")),
                    )
                    .add_item(
                        native_menu_item("Save As...")
                            .shortcut(MenuShortcut::cmd_shift('s'))
                            .on_select(|| println!("File > Save As")),
                    )
                    .add_item(native_separator())
                    .add_item(
                        native_menu_item("Export...")
                            .shortcut_cmd('e')
                            .on_select(|| println!("File > Export")),
                    ),
            )
            // Add a custom View menu
            .add_menu(
                native_menu("View")
                    .add_item(
                        native_menu_item("Zoom In")
                            .shortcut_cmd('+')
                            .on_select(|| println!("View > Zoom In")),
                    )
                    .add_item(
                        native_menu_item("Zoom Out")
                            .shortcut_cmd('-')
                            .on_select(|| println!("View > Zoom Out")),
                    )
                    .add_item(
                        native_menu_item("Actual Size")
                            .shortcut_cmd('0')
                            .on_select(|| println!("View > Actual Size")),
                    )
                    .add_item(native_separator())
                    .add_item(
                        native_menu_item("Toggle Sidebar")
                            .shortcut(MenuShortcut::cmd_shift('s'))
                            .on_select(|| println!("View > Toggle Sidebar")),
                    ),
            )
            // Add a custom Help menu
            .add_menu(
                native_menu("Help")
                    .add_item(
                        native_menu_item("Documentation")
                            .on_select(|| println!("Help > Documentation")),
                    )
                    .add_item(
                        native_menu_item("About Elements Gallery")
                            .on_select(|| println!("Help > About")),
                    ),
            )
            // Include standard OS menus
            .include_app_menu(true)
            .include_edit_menu(true)
            .include_window_menu(true),
    );
}
