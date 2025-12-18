use mkgraphic::prelude::*;

fn main() {
    let mut app = App::new();
    let mut window = Window::new("Hello MKGraphic", Extent::new(800.0, 600.0));

    let content = vtile![
        label("Hello, World!"),
        button("Click me!").on_click(|| println!("Clicked!")),
    ];

    window.set_content(share(content));
    window.show();
    app.run();
}
