use std::{env, path::{Path, PathBuf}, fs::File, io::Write};
use auditable_serde::RawVersionInfo;

/// Put this in your `main.rs` or `lib.rs` to inject dependency info into a dedicated linker section of your binary.
/// In order to work around a bug in rustc you also have to pass an identifier into this macro and then use it,
/// for example:
/// ```rust
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
        #[link_section = ".rust-audit-dep-list"]
        static AUDITABLE_VERSION_INFO: [u8; include_bytes!(env!("RUST_AUDIT_DEPENDENCY_FILE_LOCATION"))
        .len()] = *include_bytes!(env!("RUST_AUDIT_DEPENDENCY_FILE_LOCATION"));
        &AUDITABLE_VERSION_INFO
    });
}

/// Run this in your build.rs to collect dependency info and make it avaible to `inject_dependency_list!` macro
pub fn collect_dependency_list() {
    let cargo_lock_contents = load_cargo_lock();
    let version_info = RawVersionInfo::from_toml(&cargo_lock_contents).unwrap();
    let json = serde_json::to_string(&version_info).unwrap();
    let compressed_json = miniz_oxide::deflate::compress_to_vec_zlib(json.as_bytes(), 7);
    let output_file_path = output_file_path();
    write_dependency_info(&compressed_json, &output_file_path);
    export_dependency_file_path(&output_file_path);
}

fn load_cargo_lock() -> String {
    let crate_root_dir = PathBuf::from(std::env::var("CARGO_MANIFEST_DIR").unwrap());
    let cargo_lock_location = crate_root_dir.join("Cargo.lock");
    let cargo_lock_contents = std::fs::read_to_string(cargo_lock_location).unwrap();
    cargo_lock_contents
}

fn output_file_path() -> std::path::PathBuf {
    let out_dir = env::var("OUT_DIR").unwrap();
    let dest_dir = Path::new(&out_dir);
    dest_dir.join("dependency-list.json.zlib")
}

fn write_dependency_info(data: &[u8], path: &Path) {
    let f = File::create(path).unwrap();
    let mut writer = std::io::BufWriter::new(f);
    writer.write_all(data).unwrap();
}

fn export_dependency_file_path(path: &Path) {
    println!("cargo:rustc-env=RUST_AUDIT_DEPENDENCY_FILE_LOCATION={}", path.display());
}