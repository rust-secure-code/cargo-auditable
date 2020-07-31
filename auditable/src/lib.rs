#[used]
#[link_section = ".rust-audit-dep-list"]
static AUDITABLE_VERSION_INFO: [u8; include_bytes!(concat!(
    env!("OUT_DIR"),
    "/Cargo.lock.annotated"
))
.len()] = *include_bytes!(concat!(env!("OUT_DIR"), "/Cargo.lock.annotated"));

/// Returns the version info embedded into the executable at compile time.
/// You should call `str::from_utf8()` on it to make it usable.
#[inline]
pub fn version_info() -> &'static [u8] {
    &AUDITABLE_VERSION_INFO
}
