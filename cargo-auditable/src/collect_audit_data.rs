use auditable_serde::VersionInfo;
use cargo_metadata::{Metadata, MetadataCommand};
use miniz_oxide::deflate::compress_to_vec_zlib;
use std::{convert::TryFrom, str::from_utf8};

use crate::rustc_arguments::RustcArgs;

/// Run this in your build.rs to collect dependency info and make it avaible to `inject_dependency_list!` macro
pub fn compressed_dependency_list(args: &RustcArgs) -> Vec<u8> {
    let version_info = VersionInfo::try_from(&get_metadata(args)).unwrap();
    let json = serde_json::to_string(&version_info).unwrap();
    let compressed_json = compress_to_vec_zlib(json.as_bytes(), choose_compression_level());
    compressed_json
}

fn choose_compression_level() -> u8 {
    // TODO: check if optimizations are enabled by parsing rustc arguments
    7
}

fn get_metadata(args: &RustcArgs) -> Metadata {
    let mut metadata_command = MetadataCommand::new();

    // Point cargo-metadata to the correct Cargo.toml in a workspace.
    // CARGO_MANIFEST_DIR env var will be set by Cargo when it calls our rustc wrapper
    let manifest_dir = std::env::var_os("CARGO_MANIFEST_DIR").unwrap();
    metadata_command.current_dir(manifest_dir);

    // Pass the features that are actually enabled for this crate to cargo-metadata
    let mut features = args.enabled_features();
    if let Some(index) = features.iter().position(|x| x == &"default") {
        features.remove(index);
    } else {
        metadata_command.features(cargo_metadata::CargoOpt::NoDefaultFeatures);
    }
    let owned_features: Vec<String> = features.iter().map(|s| s.to_string()).collect();
    metadata_command.features(cargo_metadata::CargoOpt::SomeFeatures(owned_features));

    // Get the underlying std::process::Command and re-implement MetadataCommandd::exec, to clear
    // RUSTC_WRAPPER in the child process to avoid recursion
    let mut metadata_command = metadata_command.cargo_command();
    metadata_command.env_remove("RUSTC_WORKSPACE_WRAPPER");
    let output = metadata_command.output().unwrap();
    if !output.status.success() {
        panic!(
            "cargo metadata failure: {}",
            String::from_utf8_lossy(&output.stderr)
        );
    }
    let stdout = from_utf8(&output.stdout)
        .expect("cargo metadata output not utf8")
        .lines()
        .find(|line| line.starts_with('{'))
        .expect("cargo metadata output not json");
    MetadataCommand::parse(stdout).expect("failed to parse cargo metadata output")
}
