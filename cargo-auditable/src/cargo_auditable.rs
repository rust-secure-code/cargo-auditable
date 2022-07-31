use std::{process::Command, env};

pub fn main() {
    // set the RUSTFLAGS environment variable to inject our object and call Cargo with all the Cargo args
    let mut command = cargo_command();
    // Set the environment variable to use this binary as a rustc wrapper, that's when we do the real work
    // TODO: technically argv[0] is a convention, a not certainty.
    // But it's probably not a code execution vulnerability since whoever sets this could set RUSTC_WRAPPER themselves?
    let path_to_this_binary = std::env::args_os().next().unwrap();
    command.env("RUSTC_WRAPPER", path_to_this_binary);
    let results = command.status().expect("Failed to invoke cargo! Make sure it's in your $PATH");
    std::process::exit(results.code().unwrap());
}

/// Creates a cargo command line and populates arguments from arguments passed to `cargo auditable`
/// Does not read or modify environment variables.
fn cargo_command() -> Command {
    let mut command = Command::new("cargo");
    // Pass along all our arguments; we don't currently have any args specific to `cargo auditable`
    // We skip argv[0] which is the path to this binary and the first argument which is 'auditable' passed by Cargo
    command.args(env::args_os().skip(2));
    command
}