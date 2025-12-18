//! Elements Gallery - Showcase of all UI elements
//!
//! This example demonstrates all the new UI elements added to mkgraphic.

use mkgraphic::prelude::*;

fn main() {
    let mut app = App::new();
    let mut window = Window::new("Elements Gallery", Extent::new(900.0, 700.0));

    // Create the main content
    let content = vtile![
        margin(10.0, label("MKGraphic Elements Gallery").with_font_size(24.0)),
        margin(10.0, create_gallery()),
    ];

    window.set_content(share(content));
    window.show();
    app.run();
}

fn create_gallery() -> impl Element {
    htile![
        // Left column - Basic controls
        margin(10.0, vtile![
            section_label("Sliders"),
            margin(5.0, slider().on_change(|v| println!("Slider: {:.2}", v))),
            margin(5.0, slider().value(0.5).on_change(|v| println!("Slider 2: {:.2}", v))),

            section_label("Checkboxes"),
            margin(5.0, checkbox("Option 1").on_change(|checked| println!("Checkbox 1: {}", checked))),
            margin(5.0, checkbox("Option 2").checked(true).on_change(|checked| println!("Checkbox 2: {}", checked))),

            section_label("Radio Buttons"),
            margin(5.0, radio_button("Choice A").on_select(|| println!("Radio A selected"))),
            margin(5.0, radio_button("Choice B").on_select(|| println!("Radio B selected"))),

            section_label("Toggle Switches"),
            margin(5.0, htile![
                label("Dark Mode"),
                slide_switch().on_change(|on| println!("Switch: {}", on)),
            ]),
            margin(5.0, htile![
                label("Notifications"),
                slide_switch().on(true).on_change(|on| println!("Notifications: {}", on)),
            ]),
        ]),

        // Middle column - Value controls
        margin(10.0, vtile![
            section_label("Dials"),
            margin(5.0, htile![
                dial().on_change(|v| println!("Dial 1: {:.2}", v)),
                dial_with_range(0.0, 100.0).value(50.0).on_change(|v| println!("Dial 2: {:.1}", v)),
            ]),

            section_label("Text Input"),
            margin(5.0, text_box()
                .placeholder("Enter your name...")
                .on_change(|text| println!("Text: {}", text))),
            margin(5.0, text_box()
                .placeholder("Password")
                .password(true)),

            section_label("Progress Bars"),
            margin(5.0, progress_bar().value(0.3)),
            margin(5.0, progress_bar().value(0.7).show_percentage(true)),
            margin(5.0, htile![
                circular_progress().value(0.5),
                circular_progress().value(0.75).show_percentage(true),
            ]),

            section_label("Thumbwheel"),
            margin(5.0, thumbwheel().on_change(|v| println!("Thumbwheel: {:.2}", v))),
        ]),

        // Right column - Lists and selections
        margin(10.0, vtile![
            section_label("Dropdown"),
            margin(5.0, dropdown()
                .items(vec!["Option 1", "Option 2", "Option 3"])
                .on_select(|idx| println!("Selected: {}", idx))),

            section_label("List"),
            margin(5.0, fixed_size(200.0, 120.0, list()
                .items(vec![
                    ListItem::new("Item 1"),
                    ListItem::new("Item 2"),
                    ListItem::new("Item 3"),
                    ListItem::new("Item 4"),
                    ListItem::new("Item 5"),
                ])
                .on_select(|idx| println!("List selected: {}", idx)))),

            section_label("Buttons"),
            margin(5.0, button("Primary Button").on_click(|| println!("Primary clicked!"))),
            margin(5.0, button("Secondary Button").on_click(|| println!("Secondary clicked!"))),
        ]),
    ]
}

fn section_label(text: &str) -> impl Element {
    margin_top(15.0, margin_bottom(5.0, label(text).with_font_size(14.0)))
}
