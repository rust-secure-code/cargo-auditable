use std::{env, path::{Path, PathBuf}, fs::File, io::Write};
use auditable_serde::RawVersionInfo;

// FIXME: breaks on cross-compilation from windows to unix or vice versa
// because all Cargo `cfg`s are for the target platform, not host.
// Other things I've tried: https://github.com/Shnatsel/rust-audit/issues/15

/// Put this in your `main.rs` or `lib.rs` to inject dependency info into a dedicated linker section of your binary
#[cfg(not(target_family = "windows"))]
#[macro_export]
macro_rules! inject_dependency_list {
    () => {
        #[used]
        #[link_section = ".rust-audit-dep-list"]
        static AUDITABLE_VERSION_INFO: [u8; include_bytes!(concat!(
            env!("OUT_DIR"), "/",
            "dependency-list.json.gz"
        ))
        .len()] = *include_bytes!(concat!(env!("OUT_DIR"), "/dependency-list.json.gz"));
    };
}

/// Put this in your `main.rs` or `lib.rs` to inject dependency info into a dedicated linker section of your binary
#[cfg(target_family = "windows")]
#[macro_export]
macro_rules! inject_dependency_list {
    () => {
        #[used]
        #[link_section = ".rust-audit-dep-list"]
        static AUDITABLE_VERSION_INFO: [u8; include_bytes!(concat!(
            env!("OUT_DIR"), "\\",
            "dependency-list.json.gz"
        ))
        .len()] = *include_bytes!(concat!(env!("OUT_DIR"), "/dependency-list.json.gz"));
    };
}

/// Run this in your build.rs to collect dependency info and make it avaible to `inject_dependency_list!` macro
pub fn collect_dependency_list() {
    let cargo_lock_contents = load_cargo_lock();
    let version_info = RawVersionInfo::from_toml(&cargo_lock_contents).unwrap();
    let json = serde_json::to_string(&version_info).unwrap();
    let compressed_json = miniz_oxide::deflate::compress_to_vec(json.as_bytes(), 7);
    write_dependency_info(&compressed_json);
}

fn load_cargo_lock() -> String {
    let crate_root_dir = PathBuf::from(std::env::var("CARGO_MANIFEST_DIR").unwrap());
    let cargo_lock_location = crate_root_dir.join("Cargo.lock");
    let cargo_lock_contents = std::fs::read_to_string(cargo_lock_location).unwrap();
    cargo_lock_contents
}

fn write_dependency_info(data: &[u8]) {
    let out_dir = env::var("OUT_DIR").unwrap();
    let dest_dir = Path::new(&out_dir);
    let f = File::create(dest_dir.join("dependency-list.json.gz")).unwrap();
    let mut writer = std::io::BufWriter::new(f);
    writer.write_all(data).unwrap();
}