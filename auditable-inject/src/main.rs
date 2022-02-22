// Shamelessly copied from rustc codebase:
// https://github.com/rust-lang/rust/blob/e100ec5bc7cd768ec17d75448b29c9ab4a39272b/compiler/rustc_codegen_ssa/src/back/metadata.rs#L233-L287

use object::write::{self, StandardSegment, Symbol, SymbolSection};
use object::{
    elf, Architecture, BinaryFormat, Endianness, FileFlags, Object, ObjectSection, SectionFlags,
    SectionKind, SymbolFlags, SymbolKind, SymbolScope,
};


pub fn create_compressed_metadata_file(
    sess: &Session,
    metadata: &EncodedMetadata,
    symbol_name: &str,
) -> Vec<u8> {
    let mut compressed = rustc_metadata::METADATA_HEADER.to_vec();
    FrameEncoder::new(&mut compressed).write_all(metadata.raw_data()).unwrap();
    let mut file = if let Some(file) = create_object_file(sess) {
        file
    } else {
        return compressed.to_vec();
    };
    let section = file.add_section(
        file.segment_name(StandardSegment::Data).to_vec(),
        b".rustc".to_vec(),
        SectionKind::ReadOnlyData,
    );
    match file.format() {
        BinaryFormat::Elf => {
            // Explicitly set no flags to avoid SHF_ALLOC default for data section.
            file.section_mut(section).flags = SectionFlags::Elf { sh_flags: 0 };
        }
        _ => {}
    };
    let offset = file.append_section_data(section, &compressed, 1);

    // For MachO and probably PE this is necessary to prevent the linker from throwing away the
    // .rustc section. For ELF this isn't necessary, but it also doesn't harm.
    file.add_symbol(Symbol {
        name: symbol_name.as_bytes().to_vec(),
        value: offset,
        size: compressed.len() as u64,
        kind: SymbolKind::Data,
        scope: SymbolScope::Dynamic,
        weak: false,
        section: SymbolSection::Section(section),
        flags: SymbolFlags::None,
    });

    file.write().unwrap()
}

fn create_object_file(sess: &Session) -> Option<write::Object<'static>> {
    let endianness = match sess.target.options.endian {
        Endian::Little => Endianness::Little,
        Endian::Big => Endianness::Big,
    };
    let architecture = match &sess.target.arch[..] {
        "arm" => Architecture::Arm,
        "aarch64" => Architecture::Aarch64,
        "x86" => Architecture::I386,
        "s390x" => Architecture::S390x,
        "mips" => Architecture::Mips,
        "mips64" => Architecture::Mips64,
        "x86_64" => {
            if sess.target.pointer_width == 32 {
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
    let binary_format = if sess.target.is_like_osx {
        BinaryFormat::MachO
    } else if sess.target.is_like_windows {
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
                | if sess.target.options.cpu.contains("r6") {
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
                | if sess.target.options.cpu.contains("r6") {
                    elf::EF_MIPS_ARCH_64R6 | elf::EF_MIPS_NAN2008
                } else {
                    elf::EF_MIPS_ARCH_64R2
                };
            file.flags = FileFlags::Elf { e_flags };
        }
        Architecture::Riscv64 if sess.target.options.features.contains("+d") => {
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

fn main() {
    println!("Hello, world!");
}
