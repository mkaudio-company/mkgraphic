//! Demonstrates `TreeView`: an expandable/collapsible hierarchical list,
//! e.g. for a project file-tree sidebar.

use mkgraphic::prelude::*;

fn main() {
    let mut app = App::new();
    let mut window = Window::new("Tree Example", Extent::new(400.0, 400.0));

    let nodes = vec![
        tree_node("src").children(vec![
            tree_node("element").children(vec![
                tree_node("tree.rs").with_data("src/element/tree.rs"),
                tree_node("tabs.rs").with_data("src/element/tabs.rs"),
                tree_node("scroll.rs").with_data("src/element/scroll.rs"),
            ]),
            tree_node("lib.rs").with_data("src/lib.rs"),
        ]),
        tree_node("Cargo.toml").with_data("Cargo.toml"),
    ];

    let tree = tree_view()
        .nodes(nodes)
        .size(400.0, 400.0)
        .on_select(|data| println!("selected: {data}"));

    window.set_content(share(tree));
    window.show();
    app.run();
}
