use std::{error::Error, ffi::OsStr, process::Command};

use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct RustcTargetJson {
    #[serde(rename = "linker-flavor")]
    pub linker_flavor: String,
}

/// Queries target defaults, without applying command-line linker overrides.
///
/// This function relies on nightly-only rustc functionality and may break.
/// This kind of linker target querying is really only needed for obscure
/// embedded platforms, and is not essential to the core functionality.
pub fn rustc_target_json(
    rustc_path: &OsStr,
    target_triple: &str,
) -> Result<RustcTargetJson, Box<dyn Error>> {
    let output = Command::new(rustc_path)
        // Enable this unstable query only for the child process, including on stable rustc.
        .env("RUSTC_BOOTSTRAP", "1")
        .args(["-Z", "unstable-options", "--print", "target-spec-json"])
        .arg(format!("--target={target_triple}")) //not being parsed by the shell, so not a vulnerability
        .output()
        .map_err(|error| {
            format!("Failed to invoke rustc to get target-spec-json for '{target_triple}': {error}")
        })?;

    if !output.status.success() {
        return Err(format!(
            "rustc returned an error when asked for target-spec-json for '{target_triple}' ({}): {}",
            output.status,
            String::from_utf8_lossy(&output.stderr)
        )
        .into());
    }

    serde_json::from_slice(&output.stdout).map_err(|error| {
        format!("Failed to parse rustc target specification for '{target_triple}': {error}").into()
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn query_linker_flavor() {
        let rustc = std::env::var_os("RUSTC").unwrap_or_else(|| "rustc".into());
        for (target, expected_flavors) in [
            (
                "x86_64-unknown-linux-gnu",
                &["gcc", "gnu-cc", "gnu-lld-cc"][..],
            ),
            ("thumbv7em-none-eabihf", &["ld.lld", "gnu-lld"][..]),
        ] {
            let spec = rustc_target_json(&rustc, target).unwrap();
            assert!(
                expected_flavors.contains(&spec.linker_flavor.as_str()),
                "unexpected linker flavor for {target}: {}",
                spec.linker_flavor
            );
        }
    }
}
