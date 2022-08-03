use std::io::BufRead;

pub type RustcTargetInfo = std::collections::HashMap<String, String>;

pub fn rustc_target_info(target_triple: &str) -> RustcTargetInfo {
    // this is hand-rolled because the relevant piece of Cargo is hideously complex for some reason
    parse_rustc_target_info(&std::process::Command::new("rustc")
        .arg("--print=cfg")
        .arg(format!("--target={}", target_triple)) //not being parsed by the shell, so not a vulnerability
        .output()
        .unwrap_or_else(|_| panic!("Failed to invoke rustc; make sure it's in $PATH and that '{}' is a valid target triple", target_triple))
        .stdout)
}

pub(crate) fn parse_rustc_target_info(rustc_output: &[u8]) -> RustcTargetInfo {
    // Decoupled from `rustc_target_info` to allow unit testing
    // `pub(crate)` so that unit tests in other modules could use it
    rustc_output
        .lines()
        .filter_map(|line| {
            let line = line.unwrap();
            // rustc outputs some free-standing values as well as key-value pairs
            // we're only interested in the pairs, which are separated by '=' and the value is quoted
            if line.contains('=') {
                let key = line.split('=').next().unwrap();
                let mut value: String = line.split('=').skip(1).collect();
                // strip first and last chars of the quoted value. Verify that they're quotes
                assert!(value.pop().unwrap() == '"');
                assert!(value.remove(0) == '"');
                Some((key.to_owned(), value))
            } else {
                None
            }
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_rustc_parser_linux() {
        let rustc_output = br#"debug_assertions
target_arch="x86_64"
target_endian="little"
target_env="gnu"
target_family="unix"
target_feature="fxsr"
target_feature="sse"
target_feature="sse2"
target_os="linux"
target_pointer_width="64"
target_vendor="unknown"
unix
"#;
        let result = parse_rustc_target_info(rustc_output);
        assert_eq!(result.get("target_arch").unwrap(), "x86_64");
        assert_eq!(result.get("target_endian").unwrap(), "little");
        assert_eq!(result.get("target_pointer_width").unwrap(), "64");
        assert_eq!(result.get("target_vendor").unwrap(), "unknown");
    }
}
