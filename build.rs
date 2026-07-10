//! Embeds a Windows .exe icon when `MKGRAPHIC_APP_ICON` is set.
//!
//! No-op on every other platform, and a no-op on Windows too unless the env
//! var is set - packaging tooling (`cargo xtask bundle-windows`) sets it
//! before invoking `cargo build`; a plain `cargo build`/`cargo run` is
//! unaffected.

fn main() {
    #[cfg(target_os = "windows")]
    {
        if let Ok(icon_path) = std::env::var("MKGRAPHIC_APP_ICON") {
            println!("cargo:rerun-if-env-changed=MKGRAPHIC_APP_ICON");
            println!("cargo:rerun-if-changed={icon_path}");
            let mut res = winres::WindowsResource::new();
            res.set_icon(&icon_path);
            res.compile()
                .expect("failed to embed Windows icon resource");
        }
    }
}
