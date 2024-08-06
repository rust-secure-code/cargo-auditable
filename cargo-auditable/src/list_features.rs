use std::collections::BTreeSet;

use cargo_metadata::MetadataCommand;

use crate::collect_audit_data::execute_cargo_metadata;

pub fn list_features(crate_name: &str) -> Result<BTreeSet<String>, cargo_metadata::Error> {
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
    dbg!(crate_name);
    let package = medatada.packages.iter().find(|pkg| {
        pkg.targets.iter().find(|target| target.name == crate_name).is_some()
    });
    if let Some(package) = package {
        Ok(package.features.keys().cloned().collect())
    } else {
        // TODO: return error
        panic!("oh no");
    }
}
