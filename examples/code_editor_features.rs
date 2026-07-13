//! Demonstrates the code editor's read-only mode, diagnostic markers, and
//! find/highlight-matches.

use mkgraphic::prelude::*;

fn main() {
    let mut app = App::new();
    let mut window = Window::new("Code Editor Features", Extent::new(700.0, 500.0));

    let editor = code_editor().width(700.0).height(500.0).text(
        "fn main() {\n    let x = 1;\n    let y = 2;\n    println!(\"{}\", x + y);\n}\n"
            .to_string(),
    );

    editor.set_diagnostics(vec![
        Diagnostic {
            line: 1,
            severity: DiagnosticSeverity::Warning,
            message: "unused variable `x`".to_string(),
        },
        Diagnostic {
            line: 3,
            severity: DiagnosticSeverity::Error,
            message: "mismatched types".to_string(),
        },
    ]);
    editor.find("let");

    window.set_content(share(editor));
    window.show();
    app.run();
}
