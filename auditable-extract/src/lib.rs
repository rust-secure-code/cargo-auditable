#![forbid(unsafe_code)]

use binfarce;
use binfarce::Format;

pub fn raw_auditable_data<'a>(data: &'a [u8]) -> Option<&'a [u8]> {
    match binfarce::detect_format(data) {
        Format::Elf32{byte_order} => {
            let section = binfarce::elf32::parse(data, byte_order).ok()?
                .section_with_name(".rust-audit-dep-list")?;
                data.get(section.range().ok()?)
        },
        Format::Elf64{byte_order} => {
            let section = binfarce::elf64::parse(data, byte_order).ok()?
                .section_with_name(".rust-audit-dep-list")?;
                data.get(section.range().ok()?)
        },
        _ => todo!(),
    }
}