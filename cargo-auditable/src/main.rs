#![forbid(unsafe_code)]

mod object_file;
mod target_info;
mod collect_audit_data;

use cargo_subcommand::Subcommand;
use std::{process::Command, ffi::OsString};

fn main() {
    // TODO: refactor cargo-subcommand to use os_args and OsStr types. Paths can be non-UTF-8 on most platforms.
    // TODO: fix https://github.com/dvc94ch/cargo-subcommand/issues/9, it's a release blocker
    let cmd = Subcommand::new(std::env::args(), "auditable", |_, _| Ok(false)).unwrap();
    //println!("{:#?}", cmd);

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
    let mut command = cargo_command(&cmd);
    command.env("RUSTFLAGS", rustflags_with_audit_object());
    let results = command.status().expect("Failed to invoke cargo! Make sure it's in your $PATH");
    std::process::exit(results.code().unwrap());
}

/// Creates a cargo command line and populates arguments from arguments passed to `cargo auditable`
/// Does not read or modify environment variables.
fn cargo_command(cargo_auditable_args: &Subcommand) -> Command {
    let mut command = Command::new("cargo");
    // subcommand such as "build", "run", "check", "test"
    command.arg(cargo_auditable_args.cmd()); // TODO: prohibit recursion. Someone could put "auditable" here, and the build will fail.
    // Pass along all our arguments; we don't currently have any args specific to `cargo auditable`
    for arg in cargo_auditable_args.args() {
        command.arg(arg);
    }
    command
}

const OUR_RUSTFLAGS: &str = " -Clink-arg=audit_data.o -Clink-arg=-Wl,--require-defined=AUDITABLE_VERSION_INFO ";

fn rustflags_with_audit_object() -> OsString {
    match std::env::var_os("RUSTFLAGS") {
        None => OUR_RUSTFLAGS.into(),
        Some(mut val) => {
            val.push(OUR_RUSTFLAGS);
            val
        },
    }
}