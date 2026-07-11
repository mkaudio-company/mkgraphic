//! Demonstrates `App::schedule_timer`: a status bar that updates on its own
//! every second, without waiting for a mouse/key event to trigger a redraw.

use mkgraphic::prelude::*;
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::Arc;

fn main() {
    let mut app = App::new();
    let mut window = Window::new("Timer Example", Extent::new(400.0, 200.0));

    let status = Arc::new(status_bar().text("seconds elapsed: 0".to_string()));
    window.set_content(status.clone() as ElementPtr);
    window.show();

    let seconds = Arc::new(AtomicU32::new(0));
    // Kept alive as `_timer` for the rest of `main` (including through
    // `app.run()`'s blocking event loop): dropping the handle invalidates
    // the timer, so it must outlive however long it should keep firing.
    // `window` moves into this closure too (it's not needed in `main`
    // after `show()`) so its `refresh()` can be called after each update --
    // without an explicit `refresh()`, the new text is set but nothing
    // repaints, since a timer firing isn't a mouse/key event and mkgraphic
    // only requests a redraw from inside those.
    let _timer = app.schedule_timer(1.0, move || {
        let n = seconds.fetch_add(1, Ordering::SeqCst) + 1;
        status.set_text(format!("seconds elapsed: {n}"));
        window.refresh();
    });

    app.run();
}
