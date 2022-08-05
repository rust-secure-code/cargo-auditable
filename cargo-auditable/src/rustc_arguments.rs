//! Parses rustc arguments to extract the info not provided via environment variables.

use std::{ffi::OsString, path::PathBuf};

// We use pico-args because we only need to extract a few specific arguments out of a larger set,
// and other parsers (rustc's `getopts`, cargo's `clap`) make that difficult.
//
// We also intentionally do very little validation, to avoid rejecting new configurations
// that may be added to rustc in the future.
//
// For reference, the rustc argument parsing code is at
// https://github.com/rust-lang/rust/blob/26ecd44160f54395b3bd5558cc5352f49cb0a0ba/compiler/rustc_session/src/config.rs

/// Includes only the rustc arguments we care about
pub struct RustcArgs {
    pub crate_name: String,
    pub crate_types: Vec<String>,
    pub cfg: Vec<String>,
    pub out_dir: PathBuf,
    pub target: Option<String>,
}

impl RustcArgs {
    pub fn enabled_features(&self) -> Vec<&str> {
        let mut result = Vec::new();
        for item in &self.cfg {
            if item.starts_with("feature=\"") {
                // feature names cannot contain quotes according to the documentation:
                // https://doc.rust-lang.org/cargo/reference/features.html#the-features-section
                result.push(item.split('"').nth(1).unwrap());
            }
        }
        result
    }
}

pub fn parse_args() -> Result<RustcArgs, pico_args::Error> {
    let raw_args: Vec<OsString> = std::env::args_os().skip(2).collect();
    let mut parser = pico_args::Arguments::from_vec(raw_args);

    Ok(RustcArgs {
        crate_name: parser.value_from_str("--crate-name")?,
        crate_types: parser.values_from_str("--crate-type")?,
        cfg: parser.values_from_str("--cfg")?,
        out_dir: parser.value_from_os_str::<&str, PathBuf, pico_args::Error>("--out-dir", |s| {
            Ok(PathBuf::from(s))
        })?,
        target: parser.opt_value_from_str("--target")?,
    })
}
