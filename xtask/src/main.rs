//! Packaging tasks for apps built with mkgraphic: icon generation, macOS
//! .app bundling + code signing, and a Windows MSI installer.
//!
//! Run `cargo xtask help` for usage.

mod icons;
mod mac;
mod windows;

use std::collections::HashMap;
use std::process::ExitCode;

/// Parsed `--flag value` pairs plus any leftover positional args.
pub struct Args {
    flags: HashMap<String, String>,
    switches: std::collections::HashSet<String>,
}

impl Args {
    fn parse(raw: &[String]) -> Self {
        let mut flags = HashMap::new();
        let mut switches = std::collections::HashSet::new();
        let mut i = 0;
        while i < raw.len() {
            let arg = &raw[i];
            if let Some(name) = arg.strip_prefix("--") {
                if let Some(value) = raw.get(i + 1) {
                    if value.starts_with("--") {
                        switches.insert(name.to_string());
                        i += 1;
                        continue;
                    }
                    flags.insert(name.to_string(), value.clone());
                    i += 2;
                    continue;
                } else {
                    switches.insert(name.to_string());
                }
            }
            i += 1;
        }
        Self { flags, switches }
    }

    pub fn get(&self, name: &str) -> Option<&str> {
        self.flags.get(name).map(String::as_str)
    }

    pub fn require(&self, name: &str) -> Result<&str, String> {
        self.get(name)
            .ok_or_else(|| format!("missing required --{name} argument"))
    }

    pub fn flag(&self, name: &str) -> bool {
        self.switches.contains(name)
    }
}

fn main() -> ExitCode {
    let raw: Vec<String> = std::env::args().skip(1).collect();
    let Some(command) = raw.first().cloned() else {
        print_usage();
        return ExitCode::FAILURE;
    };
    let args = Args::parse(&raw[1..]);

    let result = match command.as_str() {
        "make-icons" => icons::run(&args),
        "bundle-mac" => mac::run(&args),
        "bundle-windows" => windows::run(&args),
        "help" | "--help" | "-h" => {
            print_usage();
            Ok(())
        }
        other => Err(format!(
            "unknown command '{other}' - run `cargo xtask help`"
        )),
    };

    match result {
        Ok(()) => ExitCode::SUCCESS,
        Err(err) => {
            eprintln!("error: {err}");
            ExitCode::FAILURE
        }
    }
}

fn print_usage() {
    eprintln!(
        r#"cargo xtask <command> [options]

Commands:
  make-icons --source <square.png> --out-dir <dir>
      Generates Resources/AppIcon.icns (macOS, requires running on macOS)
      and icon.ico (Windows, cross-platform) from one square source PNG
      (1024x1024 recommended).

  bundle-mac --example <name> --icon <path.png|.icns> [options]
      Builds the example in release mode and wraps it in a signed .app
      bundle at target/bundle/<Name>.app.
        --name <DisplayName>     Defaults to the example name.
        --identifier <bundle.id> Defaults to com.mkgraphic.<example>.
        --version <x.y.z>        Defaults to the version in Cargo.toml.
        --identity <identity>    A "Developer ID Application: ..." string
                                  from *your own* Keychain. Never hardcode
                                  someone else's identity - every user of
                                  this tool signs with their own certificate.
        --no-sign                Skip codesign entirely.
                                  Without --identity, signing falls back to
                                  ad-hoc (`codesign -s -`), which runs
                                  locally but will not pass Gatekeeper for
                                  distribution to other machines.

  bundle-windows --example <name> --icon <path.ico> [options]
      Builds the example in release mode with the icon embedded in the
      .exe, then produces an MSI installer via the WiX Toolset. Must be
      run on Windows with WiX v3 (candle/light) on PATH.
        --name <DisplayName>       Defaults to the example name.
        --upgrade-code <GUID>      Stable GUID for MSI upgrades; generate
                                    once per app and keep it constant across
                                    releases. Defaults to a fixed placeholder
                                    - override this for a real release.
        --version <x.y.z>          Defaults to the crate version.
"#
    );
}
