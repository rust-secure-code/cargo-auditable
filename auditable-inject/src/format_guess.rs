//! Format guess based on the target triple, e.g. `x86_64-unknown-linux-gnu`
//! This is hand-rolled and should probably be thrown out when uplifting this into Cargo;
//! Presumably Cargo already can parse the target triple well enough.

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