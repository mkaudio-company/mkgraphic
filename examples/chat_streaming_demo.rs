//! Live visual check for `ChatHistory`'s streaming + Markdown rendering
//! (see `element::chat_history` and `support::markdown`) -- simulates a
//! model streaming in token by token via `App::schedule_timer`, without
//! needing a real model or RAG index (both add real, separate, unrelated
//! cost/time), to isolate exactly the new UI code this exists to check.

use mkgraphic::prelude::*;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;

fn main() {
    let mut app = App::new();
    let mut window = Window::new("Chat Streaming Demo", Extent::new(500.0, 500.0));

    let history = Arc::new(chat_history().width(460.0).height(460.0));
    history.push_message(ChatSender::System, "MKIDE Assistant -- streaming demo");
    history.push_message(ChatSender::User, "Give me a quick DSP tip.");
    history.start_streaming_message(ChatSender::Assistant);

    window.set_content(history.clone() as ElementPtr);
    window.show();

    let thinking = "Thinking Process:\n1. **Goal:** give one concrete DSP tip.\n2. Pick something practical, like windowing.\n3. Keep the answer short.";
    let response = "Here's a quick tip:\n\n- Always **window** your FFT input (e.g. Hann) to reduce spectral leakage.\n- A `lowpass filter` with the right *cutoff* avoids aliasing before downsampling.\n\n```\ny[n] = a * x[n] + (1 - a) * y[n-1]\n```\n\nThat's a simple one-pole lowpass you can drop straight into a real-time loop.";

    let thinking_chars: Vec<char> = thinking.chars().collect();
    let response_chars: Vec<char> = response.chars().collect();
    let index = Arc::new(AtomicUsize::new(0));

    let _timer = app.schedule_timer(0.015, move || {
        let i = index.fetch_add(1, Ordering::SeqCst);
        if i < thinking_chars.len() {
            history.append_thinking(&thinking_chars[i].to_string());
            window.refresh();
        } else if i - thinking_chars.len() < response_chars.len() {
            let j = i - thinking_chars.len();
            history.append_response(&response_chars[j].to_string());
            window.refresh();
        }
    });

    app.run();
}
