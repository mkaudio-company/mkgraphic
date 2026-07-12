//! Verifies `CloseBehavior::KeepRunning`: closing the window should leave
//! the process alive with no visible windows (checkable via `ps`), unlike
//! the previous default where a closed window couldn't come back at all.

use mkgraphic::prelude::*;

fn build_window() -> Window {
    let mut window = Window::new("Close Keep Running", Extent::new(400.0, 200.0));
    window.set_content(share(label("closing in 1s (app should stay alive)")));
    window
}

fn main() {
    let mut app = App::new();
    let mut window = build_window();
    window.show();

    app.set_close_behavior(CloseBehavior::KeepRunning(Box::new(build_window)));

    let _timer = app.schedule_timer(1.0, move || {
        window.close();
    });

    app.run();
}
