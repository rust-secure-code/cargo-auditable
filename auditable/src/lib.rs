#![feature(test)]
extern crate test;

#[link_section = ".dep-list"]
static AUDITABLE_VERSION_INFO: [u8; include_bytes!(concat!(env!("OUT_DIR"), "/Cargo.lock.annotated")).len()] = *include_bytes!(concat!(env!("OUT_DIR"), "/Cargo.lock.annotated"));

/// Call this from main() or other reachable place in the code to annotate your executable
/// with information on which library versions were used for building it.
/// 
/// Calling this function should not incur any performance penalty at runtime.
#[inline(always)]
pub fn annotate_this_executable() {
    test::black_box(AUDITABLE_VERSION_INFO);
}

#[inline]
pub fn version_info() -> &'static [u8] {
    &AUDITABLE_VERSION_INFO
}
