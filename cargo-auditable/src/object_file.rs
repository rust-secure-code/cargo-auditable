//! Shamelessly copied from rustc codebase:
//! https://github.com/rust-lang/rust/blob/3b186511f62b0ce20e72ede0e8e13f8787155f02/compiler/rustc_codegen_ssa/src/back/metadata.rs#L260-L298
//! and butchered ever so slightly

use object::write::{self, StandardSegment, Symbol, SymbolSection};
use object::{
    elf, Architecture, BinaryFormat, Endianness, FileFlags, SectionFlags, SectionKind, SymbolFlags,
    SymbolKind, SymbolScope,
};

use crate::target_info::RustcTargetInfo;

pub fn create_metadata_file(
    // formerly `create_compressed_metadata_file` in the rustc codebase
    target_info: &RustcTargetInfo,
    target_triple: &str,
    contents: &[u8],
    symbol_name: &str,
) -> Vec<u8> {
    let mut file =
        create_object_file(target_info, target_triple).expect("Unsupported architecture");
    let section = file.add_section(
        file.segment_name(StandardSegment::Data).to_vec(),
        b".dep-v0".to_vec(),
        SectionKind::ReadOnlyData,
    );
    match file.format() {
        BinaryFormat::Elf => {
            // Explicitly set no flags to avoid SHF_ALLOC default for data section.
            file.section_mut(section).flags = SectionFlags::Elf { sh_flags: 0 };
        }
        _ => {}
    };
    let offset = file.append_section_data(section, contents, 1);

    // For MachO and probably PE this is necessary to prevent the linker from throwing away the
    // .rustc section. For ELF this isn't necessary, but it also doesn't harm.
    file.add_symbol(Symbol {
        name: symbol_name.as_bytes().to_vec(),
        value: offset,
        size: contents.len() as u64,
        kind: SymbolKind::Data,
        scope: SymbolScope::Dynamic,
        weak: false,
        section: SymbolSection::Section(section),
        flags: SymbolFlags::None,
    });

    file.write().unwrap()
}

fn create_object_file(
    info: &RustcTargetInfo,
    target_triple: &str,
) -> Option<write::Object<'static>> {
    // This conversion evolves over time, and has some subtle logic for MIPS and RISC-V later on, that also evolves.
    // If/when uplifiting this into Cargo, we will need to extract this code from rustc and put it in the `object` crate
    // so that it could be shared between rustc and Cargo.
    let endianness = match info["target_endian"].as_str() {
        "little" => Endianness::Little,
        "big" => Endianness::Big,
        _ => unreachable!(),
    };
    let architecture = match info["target_arch"].as_str() {
        "arm" => Architecture::Arm,
        "aarch64" => Architecture::Aarch64,
        "x86" => Architecture::I386,
        "s390x" => Architecture::S390x,
        "mips" => Architecture::Mips,
        "mips64" => Architecture::Mips64,
        "x86_64" => {
            if info["target_pointer_width"].as_str() == "32" {
                Architecture::X86_64_X32
            } else {
                Architecture::X86_64
            }
        }
        "powerpc" => Architecture::PowerPc,
        "powerpc64" => Architecture::PowerPc64,
        "riscv32" => Architecture::Riscv32,
        "riscv64" => Architecture::Riscv64,
        "sparc64" => Architecture::Sparc64,
        // Unsupported architecture.
        _ => return None,
    };
    let binary_format = if target_triple.contains("-apple-") {
        BinaryFormat::MachO
    } else if target_triple.contains("-windows-") {
        BinaryFormat::Coff
    } else {
        BinaryFormat::Elf
    };

    let mut file = write::Object::new(binary_format, architecture, endianness);
    match architecture {
        Architecture::Mips => {
            // copied from `mipsel-linux-gnu-gcc foo.c -c` and
            // inspecting the resulting `e_flags` field.
            let e_flags = elf::EF_MIPS_CPIC
                | elf::EF_MIPS_PIC
                | if target_triple.contains("r6") {
                    elf::EF_MIPS_ARCH_32R6 | elf::EF_MIPS_NAN2008
                } else {
                    elf::EF_MIPS_ARCH_32R2
                };
            file.flags = FileFlags::Elf { e_flags };
        }
        Architecture::Mips64 => {
            // copied from `mips64el-linux-gnuabi64-gcc foo.c -c`
            let e_flags = elf::EF_MIPS_CPIC
                | elf::EF_MIPS_PIC
                | if target_triple.contains("r6") {
                    elf::EF_MIPS_ARCH_64R6 | elf::EF_MIPS_NAN2008
                } else {
                    elf::EF_MIPS_ARCH_64R2
                };
            file.flags = FileFlags::Elf { e_flags };
        }
        Architecture::Riscv64 if has_riscv_double_precision_float_abi(target_triple) => {
            // copied from `riscv64-linux-gnu-gcc foo.c -c`, note though
            // that the `+d` target feature represents whether the double
            // float abi is enabled.
            let e_flags = elf::EF_RISCV_RVC | elf::EF_RISCV_FLOAT_ABI_DOUBLE;
            file.flags = FileFlags::Elf { e_flags };
        }
        _ => {}
    };
    Some(file)
}

// This function was not present in the original rustc code, which simply used
// `sess.target.options.features.contains("+d")`
// We do not have access to compiler internals, so we have to reimplement the check
// for double-precision floating-point ABI.
fn has_riscv_double_precision_float_abi(target_triple: &str) -> bool {
    let arch = target_triple.split('-').next().unwrap();
    assert_eq!(&arch[..5], "riscv");
    let extensions = &arch[7..];
    extensions.contains('g') || extensions.contains('d')
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::target_info::parse_rustc_target_info;

    #[test]
    fn test_riscv_abi_detection() {
        // real-world target with double floats
        assert!(has_riscv_double_precision_float_abi(
            "riscv64gc-unknown-linux-gnu"
        ));
        // real-world target without double floats
        assert!(!has_riscv_double_precision_float_abi(
            "riscv32imac-unknown-none-elf"
        ));
        // made-up target with double floats but without atomics
        assert!(has_riscv_double_precision_float_abi(
            "riscv64imd-unknown-none-elf"
        ));
    }

    #[test]
    fn test_create_object_file_linux() {
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
        let target_triple = "x86_64-unknown-linux-gnu";
        let target_info = parse_rustc_target_info(rustc_output);
        let result = create_object_file(&target_info, target_triple).unwrap();
        assert_eq!(result.format(), BinaryFormat::Elf);
        assert_eq!(result.architecture(), Architecture::X86_64);
    }

    #[test]
    fn test_create_object_file_windows_msvc() {
        let rustc_output = br#"debug_assertions
target_arch="x86_64"
target_endian="little"
target_env="msvc"
target_family="windows"
target_feature="fxsr"
target_feature="sse"
target_feature="sse2"
target_os="windows"
target_pointer_width="64"
target_vendor="pc"
windows
"#;
        let target_triple = "x86_64-pc-windows-msvc";
        let target_info = parse_rustc_target_info(rustc_output);
        let result = create_object_file(&target_info, target_triple).unwrap();
        assert_eq!(result.format(), BinaryFormat::Coff);
        assert_eq!(result.architecture(), Architecture::X86_64);
    }

    #[test]
    fn test_create_object_file_windows_gnu() {
        let rustc_output = br#"debug_assertions
target_arch="x86_64"
target_endian="little"
target_env="gnu"
target_family="windows"
target_feature="fxsr"
target_feature="sse"
target_feature="sse2"
target_os="windows"
target_pointer_width="64"
target_vendor="pc"
windows
"#;
        let target_triple = "x86_64-pc-windows-gnu";
        let target_info = crate::target_info::parse_rustc_target_info(rustc_output);
        let result = create_object_file(&target_info, target_triple).unwrap();
        assert_eq!(result.format(), BinaryFormat::Coff);
        assert_eq!(result.architecture(), Architecture::X86_64);
    }

    #[test]
    fn test_create_object_file_macos() {
        let rustc_output = br#"debug_assertions
target_arch="x86_64"
target_endian="little"
target_env=""
target_family="unix"
target_feature="fxsr"
target_feature="sse"
target_feature="sse2"
target_feature="sse3"
target_feature="ssse3"
target_os="macos"
target_pointer_width="64"
target_vendor="apple"
unix
"#;
        let target_triple = "x86_64-apple-darwin";
        let target_info = crate::target_info::parse_rustc_target_info(rustc_output);
        let result = create_object_file(&target_info, target_triple).unwrap();
        assert_eq!(result.format(), BinaryFormat::MachO);
        assert_eq!(result.architecture(), Architecture::X86_64);
    }

    #[test]
    fn test_create_object_file_linux_arm() {
        let rustc_output = br#"debug_assertions
target_arch="aarch64"
target_endian="little"
target_env="gnu"
target_family="unix"
target_os="linux"
target_pointer_width="64"
target_vendor="unknown"
unix
"#;
        let target_triple = "aarch64-unknown-linux-gnu";
        let target_info = parse_rustc_target_info(rustc_output);
        let result = create_object_file(&target_info, target_triple).unwrap();
        assert_eq!(result.format(), BinaryFormat::Elf);
        assert_eq!(result.architecture(), Architecture::Aarch64);
    }
}
