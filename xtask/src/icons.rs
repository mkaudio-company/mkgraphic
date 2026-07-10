//! Icon generation: one square source PNG -> macOS .icns and Windows .ico.

use std::path::{Path, PathBuf};
use std::process::Command;

use crate::Args;

pub fn run(args: &Args) -> Result<(), String> {
    let source = PathBuf::from(args.require("source")?);
    let out_dir = PathBuf::from(args.require("out-dir")?);
    std::fs::create_dir_all(&out_dir).map_err(|e| format!("creating {out_dir:?}: {e}"))?;

    if !source.exists() {
        return Err(format!("source image not found: {source:?}"));
    }

    let img = image::open(&source).map_err(|e| format!("reading {source:?}: {e}"))?;
    if img.width() != img.height() {
        eprintln!(
            "warning: source image is {}x{}, not square - icons will be stretched",
            img.width(),
            img.height()
        );
    }

    make_ico(&img, &out_dir.join("icon.ico"))?;
    println!("wrote {:?}", out_dir.join("icon.ico"));

    if cfg!(target_os = "macos") {
        icns_from_png(&source, &out_dir.join("AppIcon.icns"))?;
        println!("wrote {:?}", out_dir.join("AppIcon.icns"));
    } else {
        eprintln!("skipping .icns (requires macOS's sips/iconutil) - run make-icons on a Mac to produce it");
    }

    Ok(())
}

/// Encodes a Windows .ico containing the standard set of sizes, resized
/// from `img` with a simple triangle filter.
fn make_ico(img: &image::DynamicImage, out_path: &Path) -> Result<(), String> {
    let mut dir = ico::IconDir::new(ico::ResourceType::Icon);
    for size in [16u32, 32, 48, 64, 128, 256] {
        let resized = img.resize_exact(size, size, image::imageops::FilterType::Triangle);
        let rgba = resized.to_rgba8();
        let icon_image = ico::IconImage::from_rgba_data(size, size, rgba.into_raw());
        let entry = ico::IconDirEntry::encode(&icon_image)
            .map_err(|e| format!("encoding {size}x{size} ico entry: {e}"))?;
        dir.add_entry(entry);
    }
    let file = std::fs::File::create(out_path).map_err(|e| format!("creating {out_path:?}: {e}"))?;
    dir.write(file).map_err(|e| format!("writing {out_path:?}: {e}"))
}

/// Builds a .iconset via `sips` and packs it with `iconutil`. macOS-only.
/// Also used directly by `cargo xtask bundle-mac` when given a PNG icon.
pub fn icns_from_png(source: &Path, out_path: &Path) -> Result<(), String> {
    let tmp = std::env::temp_dir().join(format!("mkgraphic-iconset-{}", std::process::id()));
    let iconset = tmp.join("AppIcon.iconset");
    std::fs::create_dir_all(&iconset).map_err(|e| format!("creating {iconset:?}: {e}"))?;

    // (file name, pixel size)
    let sizes: &[(&str, u32)] = &[
        ("icon_16x16.png", 16),
        ("icon_16x16@2x.png", 32),
        ("icon_32x32.png", 32),
        ("icon_32x32@2x.png", 64),
        ("icon_128x128.png", 128),
        ("icon_128x128@2x.png", 256),
        ("icon_256x256.png", 256),
        ("icon_256x256@2x.png", 512),
        ("icon_512x512.png", 512),
        ("icon_512x512@2x.png", 1024),
    ];

    for (name, size) in sizes {
        let dest = iconset.join(name);
        let status = Command::new("sips")
            .args([
                "-z",
                &size.to_string(),
                &size.to_string(),
                source.to_str().ok_or("source path is not valid UTF-8")?,
                "--out",
                dest.to_str().ok_or("temp path is not valid UTF-8")?,
            ])
            .status()
            .map_err(|e| format!("running sips (is it installed? macOS only): {e}"))?;
        if !status.success() {
            return Err(format!("sips failed resizing to {size}x{size}"));
        }
    }

    let status = Command::new("iconutil")
        .args([
            "-c",
            "icns",
            iconset.to_str().ok_or("iconset path is not valid UTF-8")?,
            "-o",
            out_path.to_str().ok_or("output path is not valid UTF-8")?,
        ])
        .status()
        .map_err(|e| format!("running iconutil: {e}"))?;

    let _ = std::fs::remove_dir_all(&tmp);

    if !status.success() {
        return Err("iconutil failed to build .icns".to_string());
    }
    Ok(())
}
