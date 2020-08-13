#![forbid(unsafe_code)]

/// Put this in your `main.rs` or `lib.rs` to inject dependency info into a dedicated linker section of your binary.
/// In order to work around a bug in rustc you also have to pass an identifier into this macro and then use it,
/// for example:
/// ```rust,ignore
///static COMPRESSED_DEPENDENCY_LIST: &[u8] = auditable::inject_dependency_list!();
///
///fn main() {
///    println!("{}", COMPRESSED_DEPENDENCY_LIST[0]);
///}
///```
#[macro_export]
macro_rules! inject_dependency_list {
    () => ({
        #[used]
        #[cfg_attr(target_os = "linux", link_section = ".rust-deps-v0")]
        #[cfg_attr(target_os = "windows", link_section = ".rust-deps-v0")]
        #[cfg_attr(target_os = "macos", link_section = "__TEXT.rust-deps-v0")]
        // All other platforms are not explicitly supported yet and thus don't get any auditable info
        // It's better to compile on unsupported platforms without audit info than to break compilation
        static AUDITABLE_VERSION_INFO: [u8; include_bytes!(env!("RUST_AUDIT_DEPENDENCY_FILE_LOCATION"))
        .len()] = *include_bytes!(env!("RUST_AUDIT_DEPENDENCY_FILE_LOCATION"));
        &AUDITABLE_VERSION_INFO
    });
}
