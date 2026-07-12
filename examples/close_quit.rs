//! Verifies `CloseBehavior::QuitApp`: closing the window should terminate
//! the process. Closes itself via a timer instead of a real click, so this
//! can be checked by watching whether the process exits.

use mkgraphic::prelude::*;

fn main() {
    let mut app = App::new();
    let mut window = Window::new("Close Quit", Extent::new(400.0, 200.0));
    window.set_content(share(label("closing in 1s...")));
    window.show();

    app.set_close_behavior(CloseBehavior::QuitApp);

    let _timer = app.schedule_timer(1.0, move || {
        window.close();
    });

    app.run();
    println!("app.run() returned; process should exit now");
}
