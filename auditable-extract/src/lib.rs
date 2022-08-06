#![forbid(unsafe_code)]

//! Extracts the dependency tree information embedded in executables by
//! [`cargo auditable`](https://github.com/rust-secure-code/cargo-auditable).
//!
//! This crate parses platform-specific binary formats ([ELF](https://en.wikipedia.org/wiki/Executable_and_Linkable_Format),
//! [PE](https://en.wikipedia.org/wiki/Portable_Executable), 
//! [Mach-O](https://en.wikipedia.org/wiki/Mach-O)) and extracts the audit data.
//! 
//! Unlike other binary parsing crates, it is specifically designed to be resilient to malicious input.
//! It 100% safe Rust (including all dependencies) and performs no heap allocations.
//! 
//! ## Usage
//!
//! The following snippet demonstrates full extraction pipeline, including decompression
//! using the safe-Rust [`miniz_oxide`](http://docs.rs/miniz_oxide/) and optional JSON parsing
//! via [`auditable-serde`](http://docs.rs/auditable-serde/):
//!
//! ```rust,ignore
//! use std::io::{Read, BufReader};
//! use std::{error::Error, fs::File, str::FromStr};
//!
//! fn main() -> Result<(), Box<dyn Error>> {
//!     // Read the input
//!     let f = File::open("target/release/hello-world")?;
//!     let mut f = BufReader::new(f);
//!     let mut input_binary = Vec::new();
//!     f.read_to_end(&mut input_binary)?;
//!     // Extract the compressed audit data
//!     let compressed_audit_data = auditable_extract::raw_auditable_data(&input_binary)?;
//!     // Decompress it with your Zlib implementation of choice. We recommend miniz_oxide
//!     use miniz_oxide::inflate::decompress_to_vec_zlib;
//!     let decompressed_data = decompress_to_vec_zlib(&compressed_audit_data)
//!         .map_err(|_| "Failed to decompress audit data")?;
//!     let decompressed_data = String::from_utf8(decompressed_data)?;
//!     println!("{}", decompressed_data);
//!     // Parse the audit data to Rust data structures
//!     let dependency_tree = auditable_serde::VersionInfo::from_str(&decompressed_data);
//!     Ok(())
//! }
//! ```


use binfarce::Format;

/// Extracts the Zlib-compressed dependency info from an executable.
///
/// This function does not allocate any memory on the heap and can be safely given untrusted input.
pub fn raw_auditable_data(data: &[u8]) -> Result<&[u8], Error> {
    match binfarce::detect_format(data) {
        Format::Elf32{byte_order} => {
            let section = binfarce::elf32::parse(data, byte_order)?
                .section_with_name(".dep-v0")?.ok_or(Error::NoAuditData)?;
            Ok(data.get(section.range()?).ok_or(Error::UnexpectedEof)?)
        },
        Format::Elf64{byte_order} => {
            let section = binfarce::elf64::parse(data, byte_order)?
                .section_with_name(".dep-v0")?.ok_or(Error::NoAuditData)?;
            Ok(data.get(section.range()?).ok_or(Error::UnexpectedEof)?)
        },
        Format::Macho => {
            let parsed = binfarce::macho::parse(data)?;
            let section = parsed.section_with_name("__DATA", ".dep-v0")?;
            let section = section.ok_or(Error::NoAuditData)?;
            Ok(data.get(section.range()?).ok_or(Error::UnexpectedEof)?)
        },
        Format::PE => {
            let parsed = binfarce::pe::parse(data)?;
            let section = parsed.section_with_name(".dep-v0")?.ok_or(Error::NoAuditData)?;
            Ok(data.get(section.range()?).ok_or(Error::UnexpectedEof)?)
        }
        _ => Err(Error::NotAnExecutable)
    }
}

#[derive(Debug, Copy, Clone)]
pub enum Error {
    NoAuditData,
    NotAnExecutable,
    UnexpectedEof,
    MalformedFile,
    SymbolsSectionIsMissing,
    SectionIsMissing,
    UnexpectedSectionType
}

impl std::error::Error for Error {}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let message = match self {
            Error::NoAuditData => "No audit data found in the executable",
            Error::NotAnExecutable => "Not an executable file",
            Error::UnexpectedEof => "Unexpected end of file",
            Error::MalformedFile => "Malformed executable file",
            Error::SymbolsSectionIsMissing => "Symbols section missing from executable",
            Error::SectionIsMissing => "Section is missing from executable",
            Error::UnexpectedSectionType => "Unexpected executable section type",
        };
        write!(f, "{}", message)
    }
}

impl From<binfarce::ParseError> for Error {
    fn from(e: binfarce::ParseError) -> Self {
        match e {
            binfarce::ParseError::MalformedInput => Error::MalformedFile,
            binfarce::ParseError::UnexpectedEof => Error::UnexpectedEof,
            binfarce::ParseError::SymbolsSectionIsMissing => Error::SymbolsSectionIsMissing,
            binfarce::ParseError::SectionIsMissing(_) => Error::SectionIsMissing,
            binfarce::ParseError::UnexpectedSectionType { .. } => Error::UnexpectedSectionType,
        }
    }
}