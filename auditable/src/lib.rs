#![forbid(unsafe_code)]

//! Know the exact crate versions used to build your Rust executable.
//! Audit binaries for known bugs or security vulnerabilities in production,
//! at scale, with zero bookkeeping.
//!
//! This works by embedding data about the dependency tree in JSON format
//! into a dedicated linker section of the compiled executable.
//!
//! ## Usage
//!
//! Add the following to your `Cargo.toml`:
//!
//! ```toml
//! build = "build.rs"
//!
//! [dependencies]
//! auditable = "0.1"
//!
//! [build-dependencies]
//! auditable-build = "0.1"
//! ```
//!
//! Create a `build.rs` file next to `Cargo.toml` with the following contents:
//! ```rust,ignore
//! fn main() {
//!     auditable_build::collect_dependency_list();
//! }
//! ```
//!
//! Add the following to the beginning your `main.rs` (or any other file):
//!
//! ```rust,ignore
//! static COMPRESSED_DEPENDENCY_LIST: &[u8] = auditable::inject_dependency_list!();
//! ```
//!
//! Put the following in some reachable location in the code, e.g. in `fn main()`:
//! ```rust,ignore
//!     // Actually use the data to work around a bug in rustc:
//!     // https://github.com/rust-lang/rust/issues/47384
//!     // On nightly you can use `test::black_box` instead of `println!`
//!     println!("{}", COMPRESSED_DEPENDENCY_LIST[0]);
//! ```
//!
//! ## Recovering the info
//!
//! The data can be extracted later using the [`auditable-extract`](https://docs.rs/auditable-extract/) crate
//! or via a command-line tool.
//!
//! See the [README](https://github.com/Shnatsel/rust-audit#rust-audit) for instruction
//! on recovering the info and other frequently asked questions.

/// Embeds the dependency tree into a dedicated linker section in the compiled executable.
///
/// Requires a build script with a call to `auditable_build::collect_dependency_list()` to work.
#[macro_export]
macro_rules! inject_dependency_list {
    () => ({
        #[used]
        #[cfg_attr(target_os = "linux", link_section = ".dep-v0")]
        #[cfg_attr(target_os = "windows", link_section = ".dep-v0")]
        #[cfg_attr(target_os = "macos", link_section = "__TEXT,.dep-v0")]
        // All other platforms are not explicitly supported yet and thus don't get any auditable info
        // It's better to compile on unsupported platforms without audit info than to break compilation
        static AUDITABLE_VERSION_INFO: [u8; include_bytes!(env!("RUST_AUDIT_DEPENDENCY_FILE_LOCATION"))
        .len()] = *include_bytes!(env!("RUST_AUDIT_DEPENDENCY_FILE_LOCATION"));
        &AUDITABLE_VERSION_INFO
    });
}
