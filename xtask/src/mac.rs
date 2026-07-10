//! Builds a signed .app bundle for a release-mode example binary.

use std::path::Path;
use std::process::Command;

use crate::Args;

pub fn run(args: &Args) -> Result<(), String> {
    let example = args.require("example")?;
    let icon = args.require("icon")?;
    let display_name = args.get("name").unwrap_or(example).to_string();
    let identifier = args
        .get("identifier")
        .map(str::to_string)
        .unwrap_or_else(|| format!("com.mkgraphic.{example}"));
    let version = match args.get("version") {
        Some(v) => v.to_string(),
        None => package_version(&workspace_root()?)?,
    };

    println!("building {example} (release)...");
    let status = Command::new("cargo")
        .args(["build", "--release", "--example", example])
        .status()
        .map_err(|e| format!("running cargo build: {e}"))?;
    if !status.success() {
        return Err("cargo build failed".to_string());
    }

    let workspace_root = workspace_root()?;
    let binary_path = workspace_root.join("target/release/examples").join(example);
    if !binary_path.exists() {
        return Err(format!(
            "built binary not found at {binary_path:?} - is '{example}' an example target?"
        ));
    }

    let bundle_dir = workspace_root
        .join("target/bundle")
        .join(format!("{display_name}.app"));
    let contents = bundle_dir.join("Contents");
    let macos_dir = contents.join("MacOS");
    let resources_dir = contents.join("Resources");

    if bundle_dir.exists() {
        std::fs::remove_dir_all(&bundle_dir).map_err(|e| format!("clearing old bundle: {e}"))?;
    }
    std::fs::create_dir_all(&macos_dir).map_err(|e| format!("creating {macos_dir:?}: {e}"))?;
    std::fs::create_dir_all(&resources_dir)
        .map_err(|e| format!("creating {resources_dir:?}: {e}"))?;

    let exe_dest = macos_dir.join(&display_name);
    std::fs::copy(&binary_path, &exe_dest).map_err(|e| format!("copying binary: {e}"))?;

    install_icon(Path::new(icon), &resources_dir)?;

    std::fs::write(
        contents.join("Info.plist"),
        info_plist(&display_name, &identifier, &version),
    )
    .map_err(|e| format!("writing Info.plist: {e}"))?;

    if args.flag("no-sign") {
        println!("skipping codesign (--no-sign)");
    } else {
        sign(&bundle_dir, args.get("identity"))?;
    }

    println!("bundle ready: {}", bundle_dir.display());
    Ok(())
}

/// Copies a ready-made .icns, or converts a source PNG into one via sips/iconutil.
fn install_icon(icon: &Path, resources_dir: &Path) -> Result<(), String> {
    if !icon.exists() {
        return Err(format!("icon not found: {icon:?}"));
    }
    let dest = resources_dir.join("AppIcon.icns");
    match icon.extension().and_then(|e| e.to_str()) {
        Some("icns") => {
            std::fs::copy(icon, &dest).map_err(|e| format!("copying icon: {e}"))?;
        }
        _ => {
            // Delegate to the same conversion used by `cargo xtask make-icons`.
            crate::icons::icns_from_png(icon, &dest)?;
        }
    }
    Ok(())
}

/// Runs `codesign` against the bundle.
///
/// `identity` should be a "Developer ID Application: ..." string from the
/// *caller's own* Keychain - this tool never bundles or assumes a specific
/// signing identity, since anyone building with mkgraphic needs to sign with
/// their own certificate. Without one, falls back to ad-hoc signing, which
/// is enough to run locally but will not satisfy Gatekeeper on another Mac.
fn sign(bundle_dir: &Path, identity: Option<&str>) -> Result<(), String> {
    let identity = identity.unwrap_or("-");
    println!("codesigning with identity: {identity}");
    if identity == "-" {
        eprintln!(
            "note: ad-hoc signing (no --identity given) - fine for local testing, \
             but the app will not pass Gatekeeper if you distribute it. Pass \
             --identity \"Developer ID Application: Your Name (TEAMID)\" from \
             your own Keychain to produce a distributable build."
        );
    }

    let status = Command::new("codesign")
        .args([
            "--force",
            "--deep",
            "--options",
            "runtime",
            "--sign",
            identity,
        ])
        .arg(bundle_dir)
        .status()
        .map_err(|e| format!("running codesign: {e}"))?;

    if !status.success() {
        return Err("codesign failed".to_string());
    }
    Ok(())
}

fn info_plist(display_name: &str, identifier: &str, version: &str) -> String {
    format!(
        r#"<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
    <key>CFBundleName</key>
    <string>{display_name}</string>
    <key>CFBundleDisplayName</key>
    <string>{display_name}</string>
    <key>CFBundleIdentifier</key>
    <string>{identifier}</string>
    <key>CFBundleExecutable</key>
    <string>{display_name}</string>
    <key>CFBundleIconFile</key>
    <string>AppIcon</string>
    <key>CFBundlePackageType</key>
    <string>APPL</string>
    <key>CFBundleShortVersionString</key>
    <string>{version}</string>
    <key>CFBundleVersion</key>
    <string>{version}</string>
    <key>LSMinimumSystemVersion</key>
    <string>11.0</string>
    <key>NSHighResolutionCapable</key>
    <true/>
</dict>
</plist>
"#,
        display_name = display_name,
        identifier = identifier,
        version = version,
    )
}

fn workspace_root() -> Result<std::path::PathBuf, String> {
    // xtask always runs with its manifest dir at <root>/xtask.
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .map(Path::to_path_buf)
        .ok_or_else(|| "could not resolve workspace root".to_string())
}

/// Reads `version = "..."` out of the workspace root's Cargo.toml. A tiny
/// hand-rolled parser to avoid pulling in a full TOML dependency just for
/// this - good enough since it only needs to read one well-formed line.
fn package_version(workspace_root: &Path) -> Result<String, String> {
    let manifest = std::fs::read_to_string(workspace_root.join("Cargo.toml"))
        .map_err(|e| format!("reading Cargo.toml for version: {e}"))?;
    manifest
        .lines()
        .find_map(|line| {
            let rest = line.trim().strip_prefix("version")?.trim_start();
            let rest = rest.strip_prefix('=')?.trim_start();
            let rest = rest.strip_prefix('"')?;
            let end = rest.find('"')?;
            Some(rest[..end].to_string())
        })
        .ok_or_else(|| "could not find `version = \"...\"` in Cargo.toml".to_string())
}
