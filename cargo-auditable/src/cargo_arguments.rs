use std::ffi::OsString;

use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
/// Includes only the cargo arguments we care about
pub struct CargoArgs {
    pub offline: bool,
    pub locked: bool,
    pub frozen: bool,
    pub config: Vec<String>,
}

impl CargoArgs {
    /// Extracts Cargo flags from the arguments to the current process
    pub fn from_args() -> CargoArgs {
        // we .skip(3) to get over `cargo auditable build` and to the start of the flags
        let raw_args: Vec<OsString> = std::env::args_os().skip(3).collect();
        Self::from_args_vec(raw_args)
    }

    /// Split into its own function for unit testing
    fn from_args_vec(mut raw_args: Vec<OsString>) -> CargoArgs {
        // if there is a -- in the invocation somewhere, only parse up to it
        if let Some(position) = raw_args.iter().position(|s| s == "--") {
            raw_args.truncate(position);
        }
        let mut parser = pico_args::Arguments::from_vec(raw_args);

        CargoArgs {
            config: parser.values_from_str("--config").unwrap(),
            offline: parser.contains("--offline"),
            locked: parser.contains("--locked"),
            frozen: parser.contains("--frozen"),
        }
    }

    /// Recovers `SerializedCargoArgs` from an environment variable (if it was exported earlier)
    pub fn from_env() -> Result<Self, std::env::VarError> {
        let json_args = std::env::var("CARGO_AUDITABLE_ORIG_ARGS")?;
        // We unwrap here because we've serialized these args ourselves and they should roundtrip cleanly.
        // Deserialization would only fail if someone tampered with them in transit.
        Ok(serde_json::from_str(&json_args).unwrap())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_basic_parsing() {
        let input = [
            "cargo",
            "auditable",
            "build",
            "--locked",
            "--config",
            "net.git-fetch-with-cli=true",
            "--offline",
        ];
        let raw_args = input.iter().map(|s| OsString::from(s)).collect();
        let args = CargoArgs::from_args_vec(raw_args);
        assert_eq!(args.locked, true);
        assert_eq!(args.offline, true);
        assert_eq!(args.frozen, false);
        assert_eq!(args.config, vec!["net.git-fetch-with-cli=true"]);
    }

    #[test]
    fn test_double_dash_to_ignore_args() {
        let input = [
            "cargo",
            "auditable",
            "run",
            "--release",
            "--config",
            "net.git-fetch-with-cli=true",
            "--",
            "--offline",
        ];
        let raw_args = input.iter().map(|s| OsString::from(s)).collect();
        let args = CargoArgs::from_args_vec(raw_args);
        assert_eq!(args.offline, false);
        assert_eq!(args.config, vec!["net.git-fetch-with-cli=true"]);
    }
}
