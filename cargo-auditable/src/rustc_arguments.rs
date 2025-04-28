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
#[derive(Debug)]
pub struct RustcArgs {
    pub crate_name: Option<String>,
    pub crate_types: Vec<String>,
    pub cfg: Vec<String>,
    pub emit: Vec<String>,
    pub out_dir: Option<PathBuf>,
    pub target: Option<String>,
    pub print: Vec<String>,
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

impl RustcArgs {
    // Split into its own function for unit testing
    fn from_vec(raw_args: Vec<OsString>) -> Result<RustcArgs, pico_args::Error> {
        let mut parser = pico_args::Arguments::from_vec(raw_args);

        // --emit requires slightly more complex parsing
        let raw_emit_args: Vec<String> = parser.values_from_str("--emit")?;
        let mut emit: Vec<String> = Vec::new();
        for raw_arg in raw_emit_args {
            for item in raw_arg.split(',') {
                emit.push(item.to_owned());
            }
        }

        Ok(RustcArgs {
            crate_name: parser.opt_value_from_str("--crate-name")?,
            crate_types: parser.values_from_str("--crate-type")?,
            cfg: parser.values_from_str("--cfg")?,
            emit,
            out_dir: parser
                .opt_value_from_os_str::<&str, PathBuf, pico_args::Error>("--out-dir", |s| {
                    Ok(PathBuf::from(s))
                })?,
            target: parser.opt_value_from_str("--target")?,
            print: parser.values_from_str("--print")?,
        })
    }
}

pub fn parse_args() -> Result<RustcArgs, pico_args::Error> {
    let raw_args: Vec<OsString> = std::env::args_os().skip(2).collect();
    RustcArgs::from_vec(raw_args)
}

pub fn should_embed_audit_data(args: &RustcArgs) -> bool {
    // Only inject audit data into crate types 'bin' and 'cdylib',
    // it doesn't make sense for static libs and weird other types.
    if !(args.crate_types.contains(&"bin".to_owned())
        || args.crate_types.contains(&"cdylib".to_owned()))
    {
        return false;
    }

    // when --emit is specified explicitly, only inject audit data for --emit=link
    // because it doesn't make sense for all other types such as llvm-ir, asm, etc.
    if !args.emit.is_empty() && !args.emit.contains(&"link".to_owned()) {
        return false;
    }

    // --print disables compilation UNLESS --emit is also specified
    if !args.print.is_empty() && args.emit.is_empty() {
        return false;
    }

    true
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rustc_vv() {
        let raw_rustc_args = vec!["-vV"];
        let raw_rustc_args: Vec<OsString> = raw_rustc_args.into_iter().map(|s| s.into()).collect();
        let args = RustcArgs::from_vec(raw_rustc_args).unwrap();
        assert!(!should_embed_audit_data(&args));
    }

    #[test]
    fn rustc_version_verbose() {
        let raw_rustc_args = vec!["--version", "--verbose"];
        let raw_rustc_args: Vec<OsString> = raw_rustc_args.into_iter().map(|s| s.into()).collect();
        let args = RustcArgs::from_vec(raw_rustc_args).unwrap();
        assert!(!should_embed_audit_data(&args));
    }

    #[test]
    fn cargo_c_compatibility() {
        let raw_rustc_args = vec!["--crate-name", "rustls", "--edition=2021", "src/lib.rs", "--error-format=json", "--json=diagnostic-rendered-ansi,artifacts,future-incompat", "--crate-type", "staticlib", "--crate-type", "cdylib", "--emit=dep-info,link", "-C", "embed-bitcode=no", "-C", "debuginfo=2", "-C", "link-arg=-Wl,-soname,librustls.so.0.14.0", "-Cmetadata=rustls-ffi", "--cfg", "cargo_c", "--print", "native-static-libs", "--cfg", "feature=\"aws-lc-rs\"", "--cfg", "feature=\"capi\"", "--cfg", "feature=\"default\"", "--check-cfg", "cfg(docsrs)", "--check-cfg", "cfg(feature, values(\"aws-lc-rs\", \"capi\", \"cert_compression\", \"default\", \"no_log_capture\", \"read_buf\", \"ring\"))", "-C", "metadata=b6a43041f637feb8", "--out-dir", "/home/user/Code/rustls-ffi/target/x86_64-unknown-linux-gnu/debug/deps", "--target", "x86_64-unknown-linux-gnu", "-C", "linker=clang", "-C", "incremental=/home/user/Code/rustls-ffi/target/x86_64-unknown-linux-gnu/debug/incremental", "-L", "dependency=/home/user/Code/rustls-ffi/target/x86_64-unknown-linux-gnu/debug/deps", "-L", "dependency=/home/user/Code/rustls-ffi/target/debug/deps", "--extern", "libc=/home/user/Code/rustls-ffi/target/x86_64-unknown-linux-gnu/debug/deps/liblibc-4fc7c9f82dda33ee.rlib", "--extern", "log=/home/user/Code/rustls-ffi/target/x86_64-unknown-linux-gnu/debug/deps/liblog-6f7c8f4d1d5ec422.rlib", "--extern", "rustls=/home/user/Code/rustls-ffi/target/x86_64-unknown-linux-gnu/debug/deps/librustls-a93cda0ba0380929.rlib", "--extern", "pki_types=/home/user/Code/rustls-ffi/target/x86_64-unknown-linux-gnu/debug/deps/librustls_pki_types-27749859644f0979.rlib", "--extern", "rustls_platform_verifier=/home/user/Code/rustls-ffi/target/x86_64-unknown-linux-gnu/debug/deps/librustls_platform_verifier-bceca5cf09f3d7ba.rlib", "--extern", "webpki=/home/user/Code/rustls-ffi/target/x86_64-unknown-linux-gnu/debug/deps/libwebpki-bc4a16dd84e0b062.rlib", "-C", "link-arg=-fuse-ld=/home/user/mold-2.32.0-x86_64-linux/bin/mold", "-L", "native=/home/user/Code/rustls-ffi/target/x86_64-unknown-linux-gnu/debug/build/aws-lc-sys-d52f8990d9ede41d/out"];
        let raw_rustc_args: Vec<OsString> = raw_rustc_args.into_iter().map(|s| s.into()).collect();
        let args = RustcArgs::from_vec(raw_rustc_args).unwrap();
        assert!(should_embed_audit_data(&args));
    }

    #[test]
    fn embed_licensing_compatibility() {
        // https://github.com/rust-secure-code/cargo-auditable/issues/198
        let raw_rustc_args = vec![
            "-",
            "--crate-name ___",
            "--print=file-names",
            "--crate-type bin",
            "--crate-type rlib",
            "--crate-type dylib",
            "--crate-type cdylib",
            "--crate-type staticlib",
            "--crate-type proc-macro",
            "--print=sysroot",
            "--print=split-debuginfo",
            "--print=crate-name",
            "--print=cfg",
        ];
        let raw_rustc_args: Vec<OsString> = raw_rustc_args.into_iter().map(|s| s.into()).collect();
        let args = RustcArgs::from_vec(raw_rustc_args).unwrap();
        assert!(!should_embed_audit_data(&args));
    }

    #[test]
    fn multiple_emit_values() {
        let raw_rustc_args = vec![
            "--emit=dep-info,link",
            "--emit",
            "llvm-bc",
            // end of interesting args, start of boilerplate
            "--crate-name",
            "foobar",
            "--out-dir",
            "/foo/bar",
        ];
        let raw_rustc_args: Vec<OsString> = raw_rustc_args.into_iter().map(|s| s.into()).collect();
        let mut args = RustcArgs::from_vec(raw_rustc_args).unwrap();

        let expected = vec!["dep-info", "link", "llvm-bc"];
        let mut expected: Vec<String> = expected.into_iter().map(|s| s.into()).collect();

        args.emit.sort();
        expected.sort();

        assert_eq!(args.emit, expected)
    }
}
