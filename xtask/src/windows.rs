//! Builds a release example with its icon embedded, then packages an MSI
//! installer via the WiX Toolset (v3: candle + light). Must run on Windows
//! with WiX on PATH - there is no cross-compiled path to an MSI from macOS
//! or Linux.

use std::path::{Path, PathBuf};
use std::process::Command;

use crate::Args;

/// Fixed placeholder upgrade code. MSI upgrades key off this GUID staying
/// constant across releases of the *same* app - generate your own (e.g.
/// `[System.Guid]::NewGuid()` in PowerShell) for a real release and pass it
/// via --upgrade-code every time you rebuild that app's installer.
const PLACEHOLDER_UPGRADE_CODE: &str = "12345678-1234-1234-1234-123456789ABC";

pub fn run(args: &Args) -> Result<(), String> {
    if cfg!(not(target_os = "windows")) {
        eprintln!(
            "warning: bundle-windows produces an MSI via the native WiX Toolset \
             (candle/light), which only exists on Windows. Continuing so the \
             build step can still be sanity-checked, but MSI packaging will \
             fail here."
        );
    }

    let example = args.require("example")?;
    let icon = args.require("icon")?;
    let display_name = args.get("name").unwrap_or(example).to_string();
    let version = args
        .get("version")
        .unwrap_or(env!("CARGO_PKG_VERSION"))
        .to_string();
    let upgrade_code = args.get("upgrade-code").unwrap_or(PLACEHOLDER_UPGRADE_CODE);
    if upgrade_code == PLACEHOLDER_UPGRADE_CODE {
        eprintln!(
            "note: using the placeholder --upgrade-code. Generate a stable GUID \
             for this app and pass it explicitly on every release, or upgrades \
             won't work correctly for your users."
        );
    }

    let icon_path = std::fs::canonicalize(icon).map_err(|e| format!("resolving icon path: {e}"))?;

    println!("building {example} (release) with icon embedded...");
    let status = Command::new("cargo")
        .args(["build", "--release", "--example", example])
        .env("MKGRAPHIC_APP_ICON", &icon_path)
        .status()
        .map_err(|e| format!("running cargo build: {e}"))?;
    if !status.success() {
        return Err("cargo build failed".to_string());
    }

    let workspace_root = workspace_root()?;
    let exe_path = workspace_root
        .join("target/release/examples")
        .join(format!("{example}.exe"));
    if !exe_path.exists() {
        return Err(format!(
            "built exe not found at {exe_path:?} - is '{example}' an example target, \
             and is this running on Windows?"
        ));
    }

    let wix_out_dir = workspace_root.join("target/wix-bundle");
    std::fs::create_dir_all(&wix_out_dir).map_err(|e| format!("creating {wix_out_dir:?}: {e}"))?;

    let wxs_path = wix_out_dir.join("main.wxs");
    std::fs::write(
        &wxs_path,
        render_wxs(&display_name, &version, upgrade_code, &exe_path, &icon_path),
    )
    .map_err(|e| format!("writing {wxs_path:?}: {e}"))?;

    let wixobj_path = wix_out_dir.join("main.wixobj");
    run_tool(
        "candle",
        &[
            "-nologo",
            "-out",
            wixobj_path.to_str().ok_or("non-UTF8 path")?,
            wxs_path.to_str().ok_or("non-UTF8 path")?,
        ],
    )?;

    let msi_path = workspace_root
        .join("target/bundle")
        .join(format!("{display_name}.msi"));
    std::fs::create_dir_all(msi_path.parent().unwrap())
        .map_err(|e| format!("creating target/bundle: {e}"))?;

    run_tool(
        "light",
        &[
            "-nologo",
            "-ext",
            "WixUIExtension",
            "-out",
            msi_path.to_str().ok_or("non-UTF8 path")?,
            wixobj_path.to_str().ok_or("non-UTF8 path")?,
        ],
    )?;

    println!("installer ready: {}", msi_path.display());
    Ok(())
}

fn run_tool(name: &str, tool_args: &[&str]) -> Result<(), String> {
    println!("running: {name} {}", tool_args.join(" "));
    let status = Command::new(name).args(tool_args).status().map_err(|e| {
        format!(
            "running {name}: {e} - is the WiX Toolset v3 installed and on PATH? \
             (https://wixtoolset.org/, or `choco install wixtoolset`)"
        )
    })?;
    if !status.success() {
        return Err(format!("{name} failed"));
    }
    Ok(())
}

fn render_wxs(
    display_name: &str,
    version: &str,
    upgrade_code: &str,
    exe_path: &Path,
    icon_path: &Path,
) -> String {
    // Minimal single-file WiX v3 source: installs the exe to
    // Program Files\<DisplayName>\, adds a Start Menu shortcut, and wires up
    // the standard WixUI_InstallDir dialog set.
    format!(
        r#"<?xml version="1.0" encoding="UTF-8"?>
<Wix xmlns="http://schemas.microsoft.com/wix/2006/wi">
  <Product Id="*"
           Name="{display_name}"
           Language="1033"
           Version="{version}"
           Manufacturer="{display_name}"
           UpgradeCode="{upgrade_code}">
    <Package InstallerVersion="500" Compressed="yes" InstallScope="perMachine" />
    <MajorUpgrade DowngradeErrorMessage="A newer version of {display_name} is already installed." />
    <MediaTemplate EmbedCab="yes" />

    <Icon Id="AppIcon.ico" SourceFile="{icon_path}" />
    <Property Id="ARPPRODUCTICON" Value="AppIcon.ico" />

    <Directory Id="TARGETDIR" Name="SourceDir">
      <Directory Id="ProgramFilesFolder">
        <Directory Id="INSTALLFOLDER" Name="{display_name}" />
      </Directory>
      <Directory Id="ProgramMenuFolder">
        <Directory Id="ApplicationProgramsFolder" Name="{display_name}" />
      </Directory>
    </Directory>

    <DirectoryRef Id="INSTALLFOLDER">
      <Component Id="MainExecutable" Guid="*">
        <File Id="MainExe" Source="{exe_path}" KeyPath="yes" />
      </Component>
    </DirectoryRef>

    <DirectoryRef Id="ApplicationProgramsFolder">
      <Component Id="ApplicationShortcut" Guid="*">
        <Shortcut Id="ApplicationStartMenuShortcut"
                  Name="{display_name}"
                  Target="[INSTALLFOLDER]{exe_name}"
                  WorkingDirectory="INSTALLFOLDER"
                  Icon="AppIcon.ico" />
        <RemoveFolder Id="RemoveApplicationProgramsFolder" On="uninstall" />
        <RegistryValue Root="HKCU"
                        Key="Software\{display_name}"
                        Name="installed"
                        Type="integer"
                        Value="1"
                        KeyPath="yes" />
      </Component>
    </DirectoryRef>

    <Feature Id="MainFeature" Title="{display_name}" Level="1">
      <ComponentRef Id="MainExecutable" />
      <ComponentRef Id="ApplicationShortcut" />
    </Feature>

    <UIRef Id="WixUI_InstallDir" />
    <Property Id="WIXUI_INSTALLDIR" Value="INSTALLFOLDER" />
  </Product>
</Wix>
"#,
        display_name = display_name,
        version = version,
        upgrade_code = upgrade_code,
        icon_path = icon_path.display(),
        exe_path = exe_path.display(),
        exe_name = exe_path
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("app.exe"),
    )
}

fn workspace_root() -> Result<PathBuf, String> {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .map(Path::to_path_buf)
        .ok_or_else(|| "could not resolve workspace root".to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Can't build/run WiX on non-Windows CI, but a malformed template would
    /// still be a real bug - this at least catches broken placeholders,
    /// unbalanced tags, and unescaped paths before anyone hits them on
    /// Windows.
    #[test]
    fn wxs_template_is_well_formed_xml() {
        let xml = render_wxs(
            "Test App",
            "1.2.3",
            PLACEHOLDER_UPGRADE_CODE,
            Path::new(r"C:\work\target\release\examples\test_app.exe"),
            Path::new(r"C:\work\icons\icon.ico"),
        );

        assert!(xml.contains("Test App"));
        assert!(xml.contains("1.2.3"));
        assert!(xml.contains(PLACEHOLDER_UPGRADE_CODE));
        assert!(xml.contains("test_app.exe"));
        assert_balanced_tags(&xml);
    }

    /// Minimal well-formedness check: every opening tag has a matching
    /// closing tag (or is self-closing), in a stack-balanced order.
    fn assert_balanced_tags(xml: &str) {
        let mut stack = Vec::new();
        let mut rest = xml;
        while let Some(start) = rest.find('<') {
            let end = rest[start..].find('>').expect("unterminated tag") + start;
            let tag = &rest[start + 1..end];
            rest = &rest[end + 1..];

            if tag.starts_with("?xml") || tag.starts_with('!') {
                continue;
            }
            if tag.ends_with('/') {
                continue; // self-closing
            }
            if let Some(name) = tag.strip_prefix('/') {
                let name = name.split_whitespace().next().unwrap_or(name);
                let top = stack.pop().unwrap_or_else(|| {
                    panic!("closing tag </{name}> with no matching open tag")
                });
                assert_eq!(top, name, "mismatched closing tag: expected </{top}>, got </{name}>");
            } else {
                let name = tag.split_whitespace().next().unwrap_or(tag);
                stack.push(name.to_string());
            }
        }
        assert!(stack.is_empty(), "unclosed tags: {stack:?}");
    }
}
