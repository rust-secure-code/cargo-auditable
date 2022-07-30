use std::convert::TryFrom;
use auditable_serde::VersionInfo;
use cargo_subcommand::{Subcommand, Profile};
use miniz_oxide::deflate::compress_to_vec_zlib;
use cargo_metadata::{Metadata, MetadataCommand};

/// Run this in your build.rs to collect dependency info and make it avaible to `inject_dependency_list!` macro
pub fn compressed_dependency_list(cmd: &Subcommand) -> Vec<u8> {
    let version_info = VersionInfo::try_from(&get_metadata(cmd)).unwrap();
    let json = serde_json::to_string(&version_info).unwrap();
    let compressed_json = compress_to_vec_zlib(json.as_bytes(), choose_compression_level(cmd.profile()));
    compressed_json
}

fn choose_compression_level(profile: &Profile) -> u8 {
    match profile {
        Profile::Dev => 1,
        Profile::Release => 7, // not 9 because this also affects speed of incremental builds,
        Profile::Custom(_) => 7, // We treat custom profile as release;
        // there might be some clever way to check if optimizations are enabled, but it's out of scope for now
    }
}

fn get_metadata(cmd: &Subcommand) -> Metadata {
    let metadata_command = MetadataCommand::new();
    // TODO: feature handling
    // let mut features = enabled_features();
    // if let Some(index) = features.iter().position(|x| x.as_str() == "default") {
    //     features.remove(index);
    // } else {
    //     metadata_command.features(cargo_metadata::CargoOpt::NoDefaultFeatures);
    // }
    // metadata_command.features(cargo_metadata::CargoOpt::SomeFeatures(features));
    metadata_command.exec().unwrap()
}