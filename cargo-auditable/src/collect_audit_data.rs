use std::convert::TryFrom;
use auditable_serde::VersionInfo;
use miniz_oxide::deflate::compress_to_vec_zlib;
use cargo_metadata::{Metadata, MetadataCommand};

/// Run this in your build.rs to collect dependency info and make it avaible to `inject_dependency_list!` macro
pub fn compressed_dependency_list() -> Vec<u8> {
    let version_info = VersionInfo::try_from(&get_metadata()).unwrap();
    let json = serde_json::to_string(&version_info).unwrap();
    let compressed_json = compress_to_vec_zlib(json.as_bytes(), choose_compression_level());
    compressed_json
}

fn choose_compression_level() -> u8 {
    // TODO: check if optimizations are enabled by parsing rustc arguments
    7
}

fn get_metadata() -> Metadata {
    let mut metadata_command = MetadataCommand::new();
    // this env var will be set by Cargo
    let manifest_dir = std::env::var_os("CARGO_MANIFEST_DIR").unwrap();
    metadata_command.current_dir(manifest_dir);
    // remove RUSTC_WRAPPER so that we don't recurse back into our own rustc wrapper infinitely
    // Unfortunately, the cargo_metadata crate we use here doesn't allow setting env vars or using a custom command,
    // so we have to clear the env var from our very own process, which the metadata command will then inherit.
    // This is mindly horrifying because it's a global effect on our current process and also isn't thread-safe
    std::env::remove_var("RUSTC_WRAPPER");
    // TODO: parse rustc arguments to pass on features
    // let mut features = enabled_features();
    // if let Some(index) = features.iter().position(|x| x.as_str() == "default") {
    //     features.remove(index);
    // } else {
    //     metadata_command.features(cargo_metadata::CargoOpt::NoDefaultFeatures);
    // }
    // metadata_command.features(cargo_metadata::CargoOpt::SomeFeatures(features));
    metadata_command.exec().unwrap()
}