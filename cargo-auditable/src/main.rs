#![forbid(unsafe_code)]

mod object_file;
mod target_info;
mod collect_audit_data;

use cargo_subcommand::Subcommand;
use std::{process::Command, ffi::OsString};

fn main() {
    // TODO: refactor cargo-subcommand to use os_args and OsStr types. Paths can be non-UTF-8 on most platforms.
    // (or maybe not in case it turns out that cargo-metadata breaks on those anyway)
    let cmd = Subcommand::new(std::env::args(), "auditable", |_, _| Ok(false)).unwrap();

    // Get the audit data to embed
    let contents: Vec<u8> = collect_audit_data::compressed_dependency_list(&cmd);

    // write the audit info to an object file
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

    // set the RUSTFLAGS environment variable to inject our object and call Cargo with all the Cargo args
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
    // Work around https://github.com/rust-lang/cargo/issues/4423 by explicitly passing the host platform if not already specified.
    // Otherwise proc macros will fail to build. Sadly this changes the output directory, which is one hell of a footgun!
    // TODO: either prevent the change to the output dir, or make it so different and obvious that it's not confusing anymore.
    if cargo_auditable_args.target().is_none() {
        command.arg(format!("--target={}", cargo_auditable_args.host_triple()));
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