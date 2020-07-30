#![feature(test)]
extern crate test;

#[link_section = ".rust-audit-dep-list"]
static AUDITABLE_VERSION_INFO: [u8; include_bytes!(concat!(env!("OUT_DIR"), "/Cargo.lock.annotated")).len()] = *include_bytes!(concat!(env!("OUT_DIR"), "/Cargo.lock.annotated"));

#[inline]
pub fn version_info() -> &'static [u8] {
    &AUDITABLE_VERSION_INFO
}
