//! Shamelessly copied from rustc codebase:
//! https://github.com/rust-lang/rust/blob/dcca6a375bd4eddb3deea7038ebf29d02af53b48/compiler/rustc_codegen_ssa/src/back/metadata.rs#L97-L206
//! and butchered ever so slightly

use object::write::{self, StandardSegment, Symbol, SymbolSection};
use object::{
    elf, Architecture, BinaryFormat, Endianness, FileFlags, SectionFlags, SectionKind, SymbolFlags,
    SymbolKind, SymbolScope,
};

use crate::platform_detection::{is_32bit, is_apple, is_windows};
use crate::target_info::RustcTargetInfo;

/// Returns None if the architecture is not supported
pub fn create_metadata_file(
    // formerly `create_compressed_metadata_file` in the rustc codebase
    target_info: &RustcTargetInfo,
    target_triple: &str,
    contents: &[u8],
    symbol_name: &str,
) -> Option<Vec<u8>> {
    let mut file = create_object_file(target_info, target_triple)?;
    let section = file.add_section(
        file.segment_name(StandardSegment::Data).to_vec(),
        b".dep-v0".to_vec(),
        SectionKind::ReadOnlyData,
    );
    if let BinaryFormat::Elf = file.format() {
        // Explicitly set no flags to avoid SHF_ALLOC default for data section.
        file.section_mut(section).flags = SectionFlags::Elf { sh_flags: 0 };
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

    Some(file.write().unwrap())
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
        "aarch64" => {
            if is_32bit(info) {
                Architecture::Aarch64_Ilp32
            } else {
                Architecture::Aarch64
            }
        }
        "x86" => Architecture::I386,
        "s390x" => Architecture::S390x,
        "mips" => Architecture::Mips,
        "mips64" => Architecture::Mips64,
        "x86_64" => {
            if is_32bit(info) {
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
        "loongarch64" => Architecture::LoongArch64,
        // Unsupported architecture.
        _ => return None,
    };
    let binary_format = if is_apple(info) {
        BinaryFormat::MachO
    } else if is_windows(info) {
        BinaryFormat::Coff
    } else {
        BinaryFormat::Elf
    };

    let mut file = write::Object::new(binary_format, architecture, endianness);
    let e_flags = match architecture {
        Architecture::Mips => {
            // the original code matches on info we don't have to support pre-1999 MIPS variants:
            // https://github.com/rust-lang/rust/blob/dcca6a375bd4eddb3deea7038ebf29d02af53b48/compiler/rustc_codegen_ssa/src/back/metadata.rs#L144C3-L153
            // We can't support them, so this part was was modified significantly.
            let arch = if target_triple.contains("r6") {
                elf::EF_MIPS_ARCH_32R6
            } else {
                elf::EF_MIPS_ARCH_32R2
            };
            // end of modified part

            // The only ABI LLVM supports for 32-bit MIPS CPUs is o32.
            let mut e_flags = elf::EF_MIPS_CPIC | elf::EF_MIPS_ABI_O32 | arch;
            // commented out: insufficient info to support this outside rustc
            // if sess.target.options.relocation_model != RelocModel::Static {
            //     e_flags |= elf::EF_MIPS_PIC;
            // }
            if target_triple.contains("r6") {
                e_flags |= elf::EF_MIPS_NAN2008;
            }
            e_flags
        }
        Architecture::Mips64 => {
            // copied from `mips64el-linux-gnuabi64-gcc foo.c -c`
            #[allow(clippy::let_and_return)] // for staying as close to upstream as possible
            let e_flags = elf::EF_MIPS_CPIC
                | elf::EF_MIPS_PIC
                | if target_triple.contains("r6") {
                    elf::EF_MIPS_ARCH_64R6 | elf::EF_MIPS_NAN2008
                } else {
                    elf::EF_MIPS_ARCH_64R2
                };
            e_flags
        }
        Architecture::Riscv32 | Architecture::Riscv64 => {
            // Source: https://github.com/riscv-non-isa/riscv-elf-psabi-doc/blob/079772828bd10933d34121117a222b4cc0ee2200/riscv-elf.adoc
            let mut e_flags: u32 = 0x0;
            let features = riscv_features(target_triple, info);
            // Check if compressed is enabled
            if features.contains('c') {
                e_flags |= elf::EF_RISCV_RVC;
            }

            // Select the appropriate floating-point ABI
            if features.contains('d') {
                e_flags |= elf::EF_RISCV_FLOAT_ABI_DOUBLE;
            } else if features.contains('f') {
                e_flags |= elf::EF_RISCV_FLOAT_ABI_SINGLE;
            } else {
                e_flags |= elf::EF_RISCV_FLOAT_ABI_SOFT;
            }
            e_flags
        }
        Architecture::LoongArch64 => {
            // Source: https://github.com/loongson/la-abi-specs/blob/release/laelf.adoc#e_flags-identifies-abi-type-and-version
            let mut e_flags: u32 = elf::EF_LARCH_OBJABI_V1;
            let features = loongarch_features(target_triple);

            // Select the appropriate floating-point ABI
            if features.contains('d') {
                e_flags |= elf::EF_LARCH_ABI_DOUBLE_FLOAT;
            } else if features.contains('f') {
                e_flags |= elf::EF_LARCH_ABI_SINGLE_FLOAT;
            } else {
                e_flags |= elf::EF_LARCH_ABI_SOFT_FLOAT;
            }
            e_flags
        }
        _ => 0,
    };
    // adapted from LLVM's `MCELFObjectTargetWriter::getOSABI`
    let os_abi = match info["target_os"].as_str() {
        "hermit" => elf::ELFOSABI_STANDALONE,
        "freebsd" => elf::ELFOSABI_FREEBSD,
        "solaris" => elf::ELFOSABI_SOLARIS,
        _ => elf::ELFOSABI_NONE,
    };
    let abi_version = 0;
    file.flags = FileFlags::Elf {
        os_abi,
        abi_version,
        e_flags,
    };
    Some(file)
}

// This function was not present in the original rustc code, which simply used
// `sess.target.options.features`
// We do not have access to compiler internals, so we have to reimplement this function.
// And `rustc --print=cfg` doesn't expose some of the features we care about,
// specifically the 'd' and 'f' features.
// Hence this function, which is not as robust as I would like.
fn riscv_features(target_triple: &str, info: &RustcTargetInfo) -> String {
    let arch = target_triple.split('-').next().unwrap();
    assert_eq!(&arch[..5], "riscv");
    let mut extensions = arch[7..].to_owned();
    if extensions.contains('g') {
        extensions.push_str("imadf");
    }
    // Most but not all riscv targets declare target features.
    // A notable exception is `riscv64-linux-android`.
    // We assume that all Linux-capable targets are -gc.
    match info["target_os"].as_str() {
        "linux" | "android" => extensions.push_str("imadfc"),
        _ => (),
    }
    extensions
}

// This function was not present in the original rustc code, which simply used
// `sess.target.options.features`
// We do not have access to compiler internals, so we have to reimplement this function.
fn loongarch_features(target_triple: &str) -> String {
    match target_triple {
        "loongarch64-unknown-none-softfloat" => "".to_string(),
        _ => "f,d".to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::target_info::parse_rustc_target_info;

    #[test]
    fn test_riscv_abi_detection() {
        // real-world target with double floats
        let features = riscv_features("riscv64gc-unknown-linux-gnu");
        assert!(features.contains('c'));
        assert!(features.contains('d'));
        assert!(features.contains('f'));
        // real-world target without floats
        let features = riscv_features("riscv32imac-unknown-none-elf");
        assert!(features.contains('c'));
        assert!(!features.contains('d'));
        assert!(!features.contains('f'));
        // real-world target without floats or compression
        let features = riscv_features("riscv32i-unknown-none-elf");
        assert!(!features.contains('c'));
        assert!(!features.contains('d'));
        assert!(!features.contains('f'));
        // made-up target without compression and with single floats
        let features = riscv_features("riscv32if-unknown-none-elf");
        assert!(!features.contains('c'));
        assert!(!features.contains('d'));
        assert!(features.contains('f'));
        // real-world Android riscv target
        let features = riscv_features("riscv64-linux-android");
        assert!(features.contains('c'));
        assert!(features.contains('d'));
        assert!(features.contains('f'));
    }

    #[test]
    fn test_loongarch_abi_detection() {
        // real-world target with double floats
        let features = loongarch_features("loongarch64-unknown-linux-gnu");
        assert!(features.contains('d'));
        assert!(features.contains('f'));
        // real-world target with double floats
        let features = loongarch_features("loongarch64-unknown-linux-musl");
        assert!(features.contains('d'));
        assert!(features.contains('f'));
        // real-world target with double floats
        let features = loongarch_features("loongarch64-unknown-none");
        assert!(features.contains('d'));
        assert!(features.contains('f'));
        // real-world target with soft floats
        let features = loongarch_features("loongarch64-unknown-none-softfloat");
        assert!(!features.contains('d'));
        assert!(!features.contains('f'));
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
