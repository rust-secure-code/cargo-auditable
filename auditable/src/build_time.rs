#![cfg(feature = "build_time")]

use std::{env, path::{Path, PathBuf}, fs::File, io::Write};
use auditable_serde::RawVersionInfo;
use miniz_oxide::deflate::compress_to_vec_zlib;

/// Run this in your build.rs to collect dependency info and make it avaible to `inject_dependency_list!` macro
pub fn collect_dependency_list() {
    let cargo_lock_contents = load_cargo_lock();
    let version_info = RawVersionInfo::from_toml(&cargo_lock_contents).unwrap();
    let json = serde_json::to_string(&version_info).unwrap();
    let compressed_json = compress_to_vec_zlib(json.as_bytes(), choose_compression_level());
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
    // Required because there's no cross-platform way to use `include_bytes!`
    // on a file from the build dir other than this. I've tried lots of them.
    // See https://github.com/rust-lang/rust/issues/75075
    println!("cargo:rustc-env=RUST_AUDIT_DEPENDENCY_FILE_LOCATION={}", path.display());
}

fn choose_compression_level() -> u8 {
    let build_profile = env::var("PROFILE").unwrap();
    match build_profile.as_str() {
        "debug" => 1,
        "release" => 7, // not 9 because this also affects speed of incremental builds
        _ => panic!("Unknown build profile: {}", &build_profile)
    }
}
