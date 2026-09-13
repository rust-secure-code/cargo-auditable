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

/// Mach-O object files are expected to carry an `LC_BUILD_VERSION` load command
/// describing the platform they were built for. Without it Apple's `ld` has nothing
/// to read the platform from, so it guesses and reports the guess on stderr:
///
/// ```text
/// ld: no platform load command found in '..._audit_data.o', assuming: macOS
/// ```
///
/// Since Rust 1.97 the compiler surfaces linker output through the `linker_messages`
/// lint, which makes that message visible on every `cargo auditable build` on macOS.
///
/// rustc emits the load command for the same reason, in the file this module is
/// adapted from: see `macho_object_build_version_for_target` in
/// `compiler/rustc_codegen_ssa/src/back/metadata.rs`.
///
/// `minos` and `sdk` are deliberately left at zero. This object carries only the
/// dependency list and no code, so it constrains nothing at runtime, and declaring a
/// minimum OS version it does not actually require risks conflicting with the
/// deployment target of the binary it is linked into. rustc omits the SDK version for
/// the same reason.
fn macho_build_version(info: &RustcTargetInfo) -> Option<write::MachOBuildVersion> {
    let target_os = info.get("target_os").map(String::as_str);
    let target_abi = info.get("target_abi").map(String::as_str);
    let platform = match (target_os, target_abi) {
        // macOS is the one Apple OS with no ABI variants — every `*-apple-darwin`
        // target reports an empty `target_abi` — so it is unambiguous even when the
        // key is missing entirely, which is what a pre-1.75-ish rustc gives us.
        (Some("macos"), _) => object::macho::PLATFORM_MACOS,
        // For every other Apple OS the ABI is precisely what separates device from
        // simulator from Mac Catalyst, so an ABSENT `target_abi` is not "device" —
        // it is "unknown", and must fall through to emitting nothing.
        (Some("ios"), Some("macabi")) => object::macho::PLATFORM_MACCATALYST,
        (Some("ios"), Some("sim")) => object::macho::PLATFORM_IOSSIMULATOR,
        (Some("ios"), Some(_)) => object::macho::PLATFORM_IOS,
        (Some("tvos"), Some("sim")) => object::macho::PLATFORM_TVOSSIMULATOR,
        (Some("tvos"), Some(_)) => object::macho::PLATFORM_TVOS,
        (Some("watchos"), Some("sim")) => object::macho::PLATFORM_WATCHOSSIMULATOR,
        (Some("watchos"), Some(_)) => object::macho::PLATFORM_WATCHOS,
        (Some("visionos"), Some("sim")) => object::macho::PLATFORM_XROSSIMULATOR,
        (Some("visionos"), Some(_)) => object::macho::PLATFORM_XROS,
        // An Apple target we cannot identify: either an OS added after this was
        // written, or a device-family target built by a compiler too old to
        // report `target_abi`. Emit nothing rather than assert a platform we
        // cannot verify — the warning is the current behaviour and is
        // recoverable, whereas a WRONG platform is a hard link failure
        // (`ld: ... has platform iOS, which is different from target platform
        // macCatalyst`).
        _ => return None,
    };
    let mut build_version = write::MachOBuildVersion::default();
    build_version.platform = platform;
    Some(build_version)
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
    if binary_format == BinaryFormat::MachO {
        if let Some(build_version) = macho_build_version(info) {
            file.set_macho_build_version(build_version);
        }
    }
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

    // Add the COFF `@feat.00` symbol used to communicate linker feature flags.
    //
    // When linking with /SAFESEH on x86, lld requires that all linker inputs be marked as safe
    // exception handling compatible. Our metadata objects masquerade as regular COFF objects and
    // are treated as linker inputs, so they need the flag too.
    //
    // This implementation mirrors the rustc's metadata object generation:
    // <https://github.com/rust-lang/rust/blob/b90dc1e597db0bbc0cab0eccb39747b1a9d7e607/compiler/rustc_codegen_ssa/src/back/metadata.rs#L224-L252>
    //
    // See also:
    //
    // - <https://github.com/rust-lang/rust/issues/96498>
    // - <https://learn.microsoft.com/en-us/windows/win32/debug/pe-format>
    if binary_format == BinaryFormat::Coff {
        // Disable mangling so the "@feat.00" symbol name is written verbatim.
        // CoffI386 mangling adds a `_` prefix which would break this special symbol.
        let original_mangling = file.mangling();
        file.set_mangling(write::Mangling::None);

        let mut feature: u64 = 0;
        if architecture == Architecture::I386 {
            feature |= 1; // IMAGE_FILE_SAFE_EXCEPTION_HANDLER
        }
        file.add_symbol(Symbol {
            name: b"@feat.00".to_vec(),
            value: feature,
            size: 0,
            kind: SymbolKind::Data,
            scope: SymbolScope::Compilation,
            weak: false,
            section: SymbolSection::Absolute,
            flags: SymbolFlags::None,
        });

        file.set_mangling(original_mangling);
    }

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
    use std::collections::HashMap;

    use super::*;
    use crate::target_info::parse_rustc_target_info;

    fn apple_target_info(target_os: &str, target_abi: Option<&str>) -> RustcTargetInfo {
        let mut info = HashMap::from([
            ("target_vendor".to_owned(), "apple".to_owned()),
            ("target_os".to_owned(), target_os.to_owned()),
        ]);
        if let Some(abi) = target_abi {
            info.insert("target_abi".to_owned(), abi.to_owned());
        }
        info
    }

    #[test]
    fn test_macho_platform_detection() {
        use object::macho;

        // Device targets report an EMPTY `target_abi`, which is what rustc
        // actually emits (`aarch64-apple-ios` -> `target_abi=""`). A MISSING key
        // is a different case entirely and is covered by its own test below.
        let cases = [
            (("macos", Some("")), macho::PLATFORM_MACOS),
            (("ios", Some("")), macho::PLATFORM_IOS),
            (("ios", Some("sim")), macho::PLATFORM_IOSSIMULATOR),
            (("ios", Some("macabi")), macho::PLATFORM_MACCATALYST),
            (("tvos", Some("")), macho::PLATFORM_TVOS),
            (("tvos", Some("sim")), macho::PLATFORM_TVOSSIMULATOR),
            (("watchos", Some("")), macho::PLATFORM_WATCHOS),
            (("watchos", Some("sim")), macho::PLATFORM_WATCHOSSIMULATOR),
            (("visionos", Some("")), macho::PLATFORM_XROS),
            (("visionos", Some("sim")), macho::PLATFORM_XROSSIMULATOR),
        ];
        for ((target_os, target_abi), expected) in cases {
            let info = apple_target_info(target_os, target_abi);
            assert_eq!(
                macho_build_version(&info).expect("known platform").platform,
                expected,
                "target_os={target_os} target_abi={target_abi:?}"
            );
        }
    }

    /// The minimum OS version and SDK version are deliberately left unset: this
    /// object carries no code, so declaring a minimum it does not require could
    /// conflict with the deployment target of the binary it is linked into.
    #[test]
    fn test_macho_build_version_leaves_minos_and_sdk_unset() {
        let version =
            macho_build_version(&apple_target_info("macos", None)).expect("known platform");
        assert_eq!(version.minos, 0);
        assert_eq!(version.sdk, 0);
    }

    /// An Apple target we do not recognise gets no load command at all, rather
    /// than a platform we cannot verify.
    #[test]
    fn test_macho_build_version_absent_for_unknown_platform() {
        assert!(macho_build_version(&apple_target_info("futureos", None)).is_none());
    }

    /// Regression: a compiler too old to report `target_abi` must NOT be treated
    /// as "device". rustc 1.74 omits the key entirely for every Apple target —
    /// verified against a real 1.74.0 toolchain — so `x86_64-apple-ios-macabi`
    /// and `aarch64-apple-ios-sim` arrive indistinguishable from device iOS.
    /// Guessing `PLATFORM_IOS` there is not a cosmetic error: the linker rejects
    /// the mismatch outright (`has platform iOS, which is different from target
    /// platform macCatalyst`), turning today's harmless warning into a build
    /// failure. Emit nothing instead.
    #[test]
    fn test_macho_build_version_absent_when_target_abi_is_unavailable() {
        for os in ["ios", "tvos", "watchos", "visionos"] {
            assert!(
                macho_build_version(&apple_target_info(os, None)).is_none(),
                "{os} without target_abi must not be assumed to be a device target"
            );
        }
    }

    /// ...but macOS is still identifiable without `target_abi`, because it is the
    /// one Apple OS with no ABI variants (every `*-apple-darwin` target reports an
    /// empty `target_abi`). Requiring the key here would silently drop the load
    /// command on the most common platform whenever an older compiler is wrapped.
    #[test]
    fn test_macho_build_version_present_for_macos_without_target_abi() {
        assert_eq!(
            macho_build_version(&apple_target_info("macos", None))
                .expect("macOS is unambiguous without target_abi")
                .platform,
            object::macho::PLATFORM_MACOS
        );
    }

    /// Device targets report an EMPTY `target_abi`, not a missing one — that is
    /// what distinguishes them from the old-compiler case above.
    #[test]
    fn test_macho_build_version_empty_target_abi_is_a_device_target() {
        assert_eq!(
            macho_build_version(&apple_target_info("ios", Some("")))
                .expect("empty target_abi is a device target")
                .platform,
            object::macho::PLATFORM_IOS
        );
    }

    #[test]
    fn test_riscv_abi_detection() {
        // real-world target with double floats
        let info = HashMap::from([("target_os".to_owned(), "linux".to_owned())]);
        let features = riscv_features("riscv64gc-unknown-linux-gnu", &info);
        assert!(features.contains('c'));
        assert!(features.contains('d'));
        assert!(features.contains('f'));
        // real-world target without floats
        let info = HashMap::from([("target_os".to_owned(), "none".to_owned())]);
        let features = riscv_features("riscv32imac-unknown-none-elf", &info);
        assert!(features.contains('c'));
        assert!(!features.contains('d'));
        assert!(!features.contains('f'));
        // real-world target without floats or compression
        let info = HashMap::from([("target_os".to_owned(), "none".to_owned())]);
        let features = riscv_features("riscv32i-unknown-none-elf", &info);
        assert!(!features.contains('c'));
        assert!(!features.contains('d'));
        assert!(!features.contains('f'));
        // made-up target without compression and with single floats
        let info = HashMap::from([("target_os".to_owned(), "none".to_owned())]);
        let features = riscv_features("riscv32if-unknown-none-elf", &info);
        assert!(!features.contains('c'));
        assert!(!features.contains('d'));
        assert!(features.contains('f'));
        // real-world Android riscv target
        let info = HashMap::from([("target_os".to_owned(), "android".to_owned())]);
        let features = riscv_features("riscv64-linux-android", &info);
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
    fn test_create_object_file_windows_msvc_i686() {
        let rustc_output = br#"debug_assertions
target_arch="x86"
target_endian="little"
target_env="msvc"
target_family="windows"
target_feature="fxsr"
target_feature="sse"
target_feature="sse2"
target_os="windows"
target_pointer_width="32"
target_vendor="pc"
windows
"#;
        let target_triple = "i686-pc-windows-msvc";
        let target_info = parse_rustc_target_info(rustc_output);
        let result = create_object_file(&target_info, target_triple).unwrap();
        assert_eq!(result.format(), BinaryFormat::Coff);
        assert_eq!(result.architecture(), Architecture::I386);
    }

    /// Verify that i686 COFF metadata objects contain an absolute `@feat.00` symbol with
    /// `IMAGE_FILE_SAFE_EXCEPTION_HANDLER` (bit 0) set.
    ///
    /// See <https://github.com/rust-lang/rust/issues/96498>
    #[test]
    fn test_create_metadata_file_windows_msvc_i686_has_feat00() {
        let rustc_output = br#"debug_assertions
target_arch="x86"
target_endian="little"
target_env="msvc"
target_family="windows"
target_feature="fxsr"
target_feature="sse"
target_feature="sse2"
target_os="windows"
target_pointer_width="32"
target_vendor="pc"
windows
"#;
        let target_triple = "i686-pc-windows-msvc";
        let target_info = parse_rustc_target_info(rustc_output);
        let contents = b"test audit data";
        let result = create_metadata_file(
            &target_info,
            target_triple,
            contents,
            "AUDITABLE_VERSION_INFO",
        )
        .expect("should produce an object file for i686-pc-windows-msvc");

        // Parse the COFF symbol table and verify `@feat.00` has value bit0=1 and absolute section.
        let symtab_ptr = u32::from_le_bytes(result[8..12].try_into().unwrap()) as usize;
        let sym_count = u32::from_le_bytes(result[12..16].try_into().unwrap()) as usize;
        let symbol_size = 18;

        let feat = (0..sym_count).find_map(|i| {
            let start = symtab_ptr + i * symbol_size;
            let end = start + symbol_size;
            let entry = result.get(start..end)?;
            if &entry[0..8] != b"@feat.00" {
                return None;
            }
            let value = u32::from_le_bytes(entry[8..12].try_into().unwrap());
            let section_number = i16::from_le_bytes(entry[12..14].try_into().unwrap());
            Some((value, section_number))
        });

        let (value, section_number) = feat.expect("COFF object for i686 must contain @feat.00");
        assert_eq!(
            value & 1,
            1,
            "@feat.00 must set IMAGE_FILE_SAFE_EXCEPTION_HANDLER on i686"
        );
        assert_eq!(
            section_number, -1,
            "@feat.00 must be an absolute COFF symbol (section number -1)"
        );
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
