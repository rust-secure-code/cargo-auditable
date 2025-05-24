use cargo_metadata::{Metadata, MetadataCommand};
use miniz_oxide::deflate::compress_to_vec_zlib;
use std::str::from_utf8;

use crate::{
    auditable_from_metadata::encode_audit_data, cargo_arguments::CargoArgs,
    rustc_arguments::RustcArgs,
};

/// Calls `cargo metadata` to obtain the dependency tree, serializes it to JSON and compresses it
pub fn compressed_dependency_list(rustc_args: &RustcArgs, target_triple: &str) -> Vec<u8> {
    let metadata = get_metadata(rustc_args, target_triple);
    let version_info = encode_audit_data(&metadata).unwrap();
    let json = serde_json::to_string(&version_info).unwrap();
    // compression level 7 makes this complete in a few milliseconds, so no need to drop to a lower level in debug mode
    let compressed_json = compress_to_vec_zlib(json.as_bytes(), 7);
    compressed_json
}

fn get_metadata(args: &RustcArgs, target_triple: &str) -> Metadata {
    // Cargo sets the path to itself in the `CARGO` environment variable:
    // https://doc.rust-lang.org/cargo/reference/environment-variables.html#environment-variables-cargo-sets-for-3rd-party-subcommands
    // This is also useful for using `cargo auditable` as a drop-in replacement for Cargo.
    let cargo_path = std::env::var_os("CARGO").unwrap_or("cargo".into());

    // Point cargo-metadata to the correct Cargo.toml in a workspace.
    // CARGO_MANIFEST_DIR env var will be set by Cargo when it calls our rustc wrapper
    let manifest_dir = std::env::var_os("CARGO_MANIFEST_DIR").unwrap();

    // Argument shared between `cargo metadata` and `cargo tree`
    let mut shared_args: Vec<String> = Vec::new();

    // Convert the features that are actually enabled for the crate into Cargo argument format
    let mut features = args.enabled_features();
    if let Some(index) = features.iter().position(|x| x == &"default") {
        features.remove(index);
    } else {
        shared_args.push("--no-default-features".into());
    }
    // no need for special handling of --all-features, features are resolved into an explicit list when passed to rustc
    if !features.is_empty() {
        shared_args.push("--features".into());
        shared_args.push(features.join(","));
    }

    // Pass arguments such as `--config`, `--offline` and `--locked`
    // from the original CLI invocation of `cargo auditable`
    let orig_args = CargoArgs::from_env()
        .expect("Env var 'CARGO_AUDITABLE_ORIG_ARGS' set by 'cargo-auditable' is unset!");
    if orig_args.offline {
        shared_args.push("--offline".to_owned());
    }
    if orig_args.frozen {
        shared_args.push("--frozen".to_owned());
    }
    if orig_args.locked {
        shared_args.push("--locked".to_owned());
    }
    for arg in orig_args.config {
        shared_args.push("--config".to_owned());
        shared_args.push(arg);
    }

    let mut metadata_args: Vec<String> = vec!["metadata".to_owned()];
    let tree_args = [
        "tree",
        "--edges=normal,build",
        "--prefix=none",
        "--format={p}",
    ];
    let mut tree_args: Vec<String> = tree_args.iter().map(|s| s.to_string()).collect();

    // Restrict the dependency resolution to just the platform the binary is being compiled for.
    // By default `cargo metadata` resolves the dependency tree for all platforms, so it has to be passed explicitly.
    metadata_args.extend_from_slice(&["--filter-platform".to_owned(), target_triple.to_owned()]);
    tree_args.extend_from_slice(&["--target".to_owned(), target_triple.to_owned()]);

    // Now that we've resolved the args, start assembling commands
    let mut metadata_command = std::process::Command::new(&cargo_path);
    let mut tree_command = std::process::Command::new(&cargo_path);

    metadata_command.args(metadata_args);
    tree_command.args(tree_args);

    for cmd in [&mut metadata_command, &mut tree_command] {
        // Clear RUSTC_WORKSPACE_WRAPPER in the child process to avoid recursion.
        // The alternative would be modifying the environment of our own process,
        // which is sketchy and discouraged on POSIX because it's not thread-safe:
        // https://doc.rust-lang.org/stable/std/env/fn.remove_var.html
        cmd.env_remove("RUSTC_WORKSPACE_WRAPPER");

        cmd.current_dir(&manifest_dir);
        cmd.args(&shared_args);
    }

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

/// Sadly `cargo metadata` does not expose enough information to accurately reconstruct
/// the dependency tree.
/// [Features are always resolved at workspace level, not per-crate.](https://github.com/rust-lang/cargo/issues/7754)
/// Features only enabled by dev-dependencies are also getting enabled in `cargo metadata` output,
/// but not in the real build.
///
/// The issue is discussed in more detail here: <https://github.com/rust-secure-code/cargo-auditable/issues/66>
///
/// We have three ways to approach this:
///
/// The [Cargo native SBOM RFC](https://github.com/rust-lang/rfcs/pull/3553) would solve it,
/// but it will be a long time (potentially years) until it's stabilized.
///
/// We could use [an independent reimplementation of the Cargo dependency resolution](https://docs.rs/guppy/)
/// to fill in the gaps in `cargo metadata`, but that involves a lot of complexity,
/// and risks getting out of sync with the actual Cargo algorithms, resulting in subtly incorrect SBOMs.
/// This will be especially bad in LTS Linux distributions where `cargo auditable` can be years out of date.
///
/// The third option is to parse `cargo tree` output, which isn't meant to be machine-readable.
/// The advantages of this is that if something goes wrong, at least it should be very noticeable,
/// and we don't take on a great deal of complexity or risk going out of sync with complex Cargo algorithms.
///
/// This implements the third option - it seems to be the least bad one available.
fn parse_cargo_tree_output() {
    todo!()
}
