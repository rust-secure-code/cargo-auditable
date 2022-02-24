#![forbid(unsafe_code)]

mod object_file;
mod target_info;
mod collect_audit_data;

use cargo_subcommand::Subcommand;

fn main() {
    // TODO: refactor cargo-subcommand to use os_args and OsStr types. Paths can be non-UTF-8 on most platforms.
    // TODO: fix https://github.com/dvc94ch/cargo-subcommand/issues/9, it's a release blocker
    let cmd = Subcommand::new(std::env::args(), "auditable", |_, _| Ok(false)).unwrap();
    println!("{:#?}", cmd);

    // Get the audit data to embed
    let contents: Vec<u8> = collect_audit_data::compressed_dependency_list(&cmd);

    // TODO: run the code from `auditable-inject` to write the metadata to an object file
    let target_triple = cmd.target().unwrap_or(cmd.host_triple());
    let target_info = target_info::rustc_target_info(&target_triple);
    let binfile = object_file::create_metadata_file(
        &target_info,
        &target_triple,
        &contents,
        "AUDITABLE_VERSION_INFO", // TODO: make a constant and version it?
    );
    // TODO: proper path
    std::fs::write("audit_data.o", binfile).expect("Unable to write output file");

    // TODO: set the RUSTFLAGS environment variable and call Cargo with all the Cargo args
}