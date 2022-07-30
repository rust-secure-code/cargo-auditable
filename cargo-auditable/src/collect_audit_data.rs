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
    let metadata_command = MetadataCommand::new();
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