use auditable_serde::VersionInfo;
use cargo_metadata::{Metadata, MetadataCommand};
use miniz_oxide::deflate::compress_to_vec_zlib;
use std::{convert::TryFrom, str::from_utf8, collections::BTreeSet};

use crate::{cargo_arguments::CargoArgs, rustc_arguments::RustcArgs};

/// Calls `cargo metadata` to obtain the dependency tree, serializes it to JSON and compresses it
pub fn compressed_dependency_list(rustc_args: &RustcArgs, target_triple: &str) -> Vec<u8> {
    let vars: Vec<_> = std::env::vars().collect();
    dbg!(vars);
    let metadata = get_metadata(rustc_args, target_triple, true);
    let version_info = VersionInfo::try_from(&metadata).unwrap();
    let json = serde_json::to_string(&version_info).unwrap();
    // compression level 7 makes this complete in a few milliseconds, so no need to drop to a lower level in debug mode
    let compressed_json = compress_to_vec_zlib(json.as_bytes(), 7);
    compressed_json
}

fn get_metadata(args: &RustcArgs, target_triple: &str, set_features: bool) -> Metadata {
    let mut metadata_command = MetadataCommand::new();

    // Cargo sets the path to itself in the `CARGO` environment variable:
    // https://doc.rust-lang.org/cargo/reference/environment-variables.html#environment-variables-cargo-sets-for-3rd-party-subcommands
    // This is also useful for using `cargo auditable` as a drop-in replacement for Cargo.
    if let Some(path) = std::env::var_os("CARGO") {
        metadata_command.cargo_path(path);
    }

    // Point cargo-metadata to the correct Cargo.toml in a workspace.
    // CARGO_MANIFEST_DIR env var will be set by Cargo when it calls our rustc wrapper
    let manifest_dir = std::env::var_os("CARGO_MANIFEST_DIR").unwrap();
    metadata_command.current_dir(manifest_dir);

    if set_features {
        // Work around a bug in Cargo: sometimes it passes `--cfg 'feature="foo"'` to rustc
        // when the feature foo actually doesn't exist, and trying to set it will cause an error:
        // https://github.com/rust-lang/cargo/issues/12336
        //
        // So first we query the metadata to see which features are present in `Cargo.toml`,
        // and then we restrict the list of features passed to rustc only to the list from `Cargo.toml`.

        // Query all existing features
        let metadata = get_metadata(args, target_triple, false);
        let current_package = current_package(&metadata);
        let all_existing_features: BTreeSet<String> = current_package.features.keys().cloned().collect();

        // Query the features currently set (may contain some invalid ones)
        let mut enabled_features = args.enabled_features();
        if let Some(index) = enabled_features.iter().position(|x| x == &"default") {
            enabled_features.remove(index);
        } else {
            metadata_command.features(cargo_metadata::CargoOpt::NoDefaultFeatures);
        }
        let mut enabled_features: Vec<String> = enabled_features.iter().map(|s| s.to_string()).collect();

        // Do the filtering
        dbg!(&all_existing_features);
        enabled_features.retain(|s| all_existing_features.contains(s));
        // Pass the final list of features to `cargo metadata`
        metadata_command.features(cargo_metadata::CargoOpt::SomeFeatures(enabled_features));
    }

    // Restrict the dependency resolution to just the platform the binary is being compiled for.
    // By default `cargo metadata` resolves the dependency tree for all platforms.
    let mut other_args = vec!["--filter-platform".to_owned(), target_triple.to_owned()];

    // Pass arguments such as `--config`, `--offline` and `--locked`
    // from the original CLI invocation of `cargo auditable`
    let orig_args = CargoArgs::from_env()
        .expect("Env var 'CARGO_AUDITABLE_ORIG_ARGS' set by 'cargo-auditable' is unset!");
    if orig_args.offline {
        other_args.push("--offline".to_owned());
    }
    if orig_args.frozen {
        other_args.push("--frozen".to_owned());
    }
    if orig_args.locked {
        other_args.push("--locked".to_owned());
    }
    for arg in orig_args.config {
        other_args.push("--config".to_owned());
        other_args.push(arg);
    }

    // This can only be done once, multiple calls will replace previously set options.
    metadata_command.other_options(other_args);

    // Get the underlying std::process::Command and re-implement MetadataCommand::exec,
    // to clear RUSTC_WORKSPACE_WRAPPER in the child process to avoid recursion.
    // The alternative would be modifying the environment of our own process,
    // which is sketchy and discouraged on POSIX because it's not thread-safe:
    // https://doc.rust-lang.org/stable/std/env/fn.remove_var.html
    let mut metadata_command = metadata_command.cargo_command();
    dbg!(&metadata_command);
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

fn current_package(metadata: &Metadata) -> &cargo_metadata::Package {
    let root_id = metadata.resolve.as_ref().unwrap().root.as_ref().unwrap();
    dbg!(&root_id);
    metadata.packages.iter().find(|p| &p.id == root_id).unwrap()
}