use std::{process::Command, env};

use crate::{collect_audit_data, target_info, object_file};

use std::io::BufRead;

pub fn main() {
    // Get the audit data to embed
    let contents: Vec<u8> = collect_audit_data::compressed_dependency_list();

    // write the audit info to an object file
    // TODO: parse rustc arguments to detemine the target passed when cross-compiling
    let target_triple = rustc_default_target();
    let target_info = target_info::rustc_target_info(&target_triple);
    let binfile = object_file::create_metadata_file(
        &target_info,
        &target_triple,
        &contents,
        "AUDITABLE_VERSION_INFO", // TODO: make a constant and version it?
    );
    // TODO: proper path
    std::fs::write("audit_data.o", binfile).expect("Unable to write output file");

    // Invoke rustc
    let mut command = rustc_command();
    let results = command.status().expect("Failed to invoke rustc! Make sure it's in your $PATH");
    std::process::exit(results.code().unwrap());
}

/// Creates a rustc command line and populates arguments from arguments passed to us.
fn rustc_command() -> Command {
    let mut command = Command::new("rustc");
    // Pass along all the arguments that Cargo meant to pass to rustc
    // We skip the path to our binary as well as the first argument passed by Cargo which is always "rustc"
    command.args(env::args_os().skip(2));
    command.arg("-Clink-arg=audit_data.o");
    command.arg("-Clink-arg=-Wl,--require-defined=AUDITABLE_VERSION_INFO");
    command
}

fn rustc_default_target() -> String {
    Command::new("rustc")
        .arg("-vV")
        .output()
        .expect("Failed to invoke rustc! Is it in your $PATH?")
        .stdout
        .lines()
        .map(|l| l.unwrap())
        .find(|l| l.starts_with("host: "))
        .map(|l| l[6..].to_string())
        .expect("Failed to parse rustc output to determine the current platform. Please report this bug!")
}