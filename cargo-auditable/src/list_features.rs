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
    //if medatada.workspace_default_members.len() == 1 {
        // TODO  
    //} else {
    // We're running Cargo version 1.70 or earlier,
    // or we made some faulty assumption somewhere
    // and it's actually possible for several
    // default workspace members to appear here.
    // debug_assert!(medatada.workspace_default_members.is_empty());
    
    // `cargo metadata` only lets us know which package we ran in on
    // if you don't pass `--no-deps` and have it resolve the whole graph.
    // We do pass `--no-deps`, so we need to do something else.
    //
    // We canonicalize the path to our current Cargo.toml
    // and the paths to all the Cargo.toml files in the metadata,
    // and select the package with the matching Cargo.toml path.
    let cargo_toml_path = Path::new("Cargo.toml").canonicalize()?;
    for pkg in &medatada.packages {

    }

    //}
    //assert!(medatada.workspace_default_members);
    //let features: BTreeSet<String> = 

    todo!()
}
