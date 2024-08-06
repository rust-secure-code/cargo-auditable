use std::{collections::BTreeSet, path::Path};

use cargo_metadata::MetadataCommand;

use crate::collect_audit_data::execute_cargo_metadata;

pub fn list_features() -> Result<BTreeSet<String>, cargo_metadata::Error> {
    let mut metadata_command = MetadataCommand::new();

    // Cargo sets the path to itself in the `CARGO` environment variable:
    // https://doc.rust-lang.org/cargo/reference/environment-variables.html#environment-variables-cargo-sets-for-3rd-party-subcommands
    // This is also useful for using `cargo auditable` as a drop-in replacement for Cargo.
    if let Some(path) = std::env::var_os("CARGO") {
        metadata_command.cargo_path(path);
    }

    // We rely on the fact that Cargo sets the working directory for `rustc`
    // to the root directory of the crate it is compilng.
    // Therefore we do not need to change it in any way, nor explicitly specify the `Cargo.toml` location.

    let options = vec!["--no-deps".to_owned(), "--offline".to_owned()];
    metadata_command.other_options(options);

    let medatada = execute_cargo_metadata(&metadata_command)?;
    // TODO: if workspace default members are available, use them
    
    // `cargo metadata` only lets us know which package we ran in on
    // if you don't pass `--no-deps` and have it resolve the whole graph.
    // We do pass `--no-deps`, so we need to do something else.
    //
    // For a discussion of trade-offs involved, see
    // https://github.com/rust-secure-code/cargo-auditable/issues/124#issuecomment-2271216985
    //
    // We canonicalize the path to our current Cargo.toml
    // and the paths to all the Cargo.toml files in the metadata,
    // and select the package with the matching Cargo.toml path.
    let cargo_toml_path = Path::new("Cargo.toml").canonicalize()?;
    dbg!(&cargo_toml_path);
    let package = medatada.packages.iter().find(|pkg| {
        if let Ok(path) = dbg!(pkg.manifest_path.canonicalize()) {
            path == cargo_toml_path
        } else {
            false
        }
    });
    if let Some(package) = package {
        Ok(package.features.keys().cloned().collect())
    } else {
        // TODO: return error
        panic!("oh no");
    }
}
