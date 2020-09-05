#![forbid(unsafe_code)]

use binfarce;
use binfarce::Format;

pub fn raw_auditable_data<'a>(data: &'a [u8]) -> Option<&'a [u8]> {
    match binfarce::detect_format(data) {
        Format::Elf32{byte_order} => {
            let section = binfarce::elf32::parse(data, byte_order).ok()?
                .section_with_name(".rust-deps-v0").ok()??;
                data.get(section.range().ok()?)
        },
        Format::Elf64{byte_order} => {
            let section = binfarce::elf64::parse(data, byte_order).ok()?
                .section_with_name(".rust-deps-v0").ok()??;
                data.get(section.range().ok()?)
        },
        Format::Macho => {
            let parsed = binfarce::macho::parse(data).ok()?;
            let section = parsed.section_with_name("__TEXT", "rust-deps-v0")?;
            data.get(section.range().ok()?)
        },
        Format::PE => {
            let parsed = binfarce::pe::parse(data).ok()?;
            let section = parsed.section_with_name("rdep-v0").ok()??;
            data.get(section.range().ok()?)
        }
        _ => None
    }
}