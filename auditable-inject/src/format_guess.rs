use std::io::BufRead;

use object::{Architecture, BinaryFormat, Endianness};

pub struct FormatDescription {
    pub format: BinaryFormat,
    pub architecture: Architecture,
    pub endian: Endianness,
}

pub fn guess_format(target_triple: &str) -> FormatDescription {
    FormatDescription {
        format: guess_binary_format(target_triple),
        architecture: guess_architecture(target_triple),
        endian: guess_endianness(target_triple),
    }
}

fn guess_endianness(target_triple: &str) -> Endianness {
    Endianness::Little // TODO
}

fn guess_binary_format(target_triple: &str) -> BinaryFormat {
    if target_triple.contains("apple") { BinaryFormat::MachO }
    else if target_triple.contains("windows") { BinaryFormat::Pe }
    else { BinaryFormat::Elf }
    // TODO: handle asm.js and wasm somehow
}

fn guess_architecture(target_triple: &str) -> Architecture {
    // Referenced from:
    // https://github.com/rust-lang/rust/blob/3b186511f62b0ce20e72ede0e8e13f8787155f02/compiler/rustc_codegen_ssa/src/back/metadata.rs#L102-L122
    if target_triple.starts_with("arm") { Architecture::Arm }
    else if target_triple.starts_with("aarch64") { Architecture::Aarch64 }
    else if target_triple.starts_with("s390x") { Architecture::S390x }
    else if target_triple.starts_with("mips64") { Architecture::Mips64 }
    else if target_triple.starts_with("mips") { Architecture::Mips }
    else if target_triple.starts_with("x86_64") { Architecture::X86_64 }
        // TODO - x32 ABI ignored for now
        // if sess.target.pointer_width == 32 {
        //     Architecture::X86_64_X32
        // }
    else if target_triple.starts_with("x86") { Architecture::I386 }
    else if target_triple.starts_with("powerpc64") { Architecture::PowerPc64 }
    else if target_triple.starts_with("powerpc") { Architecture::PowerPc }
    else if target_triple.starts_with("riscv32") { Architecture::Riscv32 }
    else if target_triple.starts_with("riscv64") { Architecture::Riscv64 }
    else if target_triple.starts_with("sparc64") { Architecture::Sparc64 }
    else { panic!("Unsupported architecture"); }
}

type RustcTargetInfo = std::collections::HashMap<String, String>;

fn rustc_target_info(target_triple: &str) -> RustcTargetInfo {
    // this is hand-rolled because the relevant piece of Cargo is hideously complex for some reason
    parse_rustc_target_info(&std::process::Command::new("rustc")
        .arg("--print=cfg")
        .arg(format!("target={}", target_triple)) //not being parsed by the shell, so not a vulnerability
        .output()
        .expect(&format!("Failed to invoke rustc; make sure it's in $PATH and that '{}' is a valid target triple", target_triple))
        .stdout)
}

fn parse_rustc_target_info(rustc_output: &[u8]) -> RustcTargetInfo {
    // this is split into its own function for unit testing
    rustc_output
    .lines()
    .filter_map(|line| {
        let line = line.unwrap();
        // rustc outputs some free-standing values as well as key-value pairs
        // we're only interested in the pairs, which are separated by '=' and the value is quoted
        if line.contains("=") {
            let key = line.split("=").nth(0).unwrap();
            let mut value: String = line.split("=").skip(1).collect();
            // strip first and last chars of the quoted value. Verify that they're quotes
            assert!(value.pop().unwrap() == '"');
            assert!(value.remove(0) == '"');
            Some((key.to_owned(), value))
        } else {
            None
        }
    }).collect()
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