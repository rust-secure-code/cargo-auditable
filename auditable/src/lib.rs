#![feature(test)]
extern crate test;

static AUDITABLE_VERSION_INFO: &'static str = include_str!(concat!(env!("OUT_DIR"), "/Cargo.lock.annotated"));
//&'static str = "CARGO_AUDIT_INFO_START;v0;stuff-here\0";

/// Call this from main() or other reachable place in the code to annotate your executable
/// with information on which library versions were used for building it.
/// 
/// Calling this function should not incur any performance penalty at runtime.
#[inline(always)]
pub fn annotate_this_executable() {
    test::black_box(AUDITABLE_VERSION_INFO);
}