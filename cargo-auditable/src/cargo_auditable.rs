use std::{env, process::Command};

pub fn main() {
    // set the RUSTFLAGS environment variable to inject our object and call Cargo with all the Cargo args
    let mut command = cargo_command();
    // Set the environment variable to use this binary as a rustc wrapper, that's when we do the real work
    // It's important that we set RUSTC_WORKSPACE_WRAPPER and not RUSTC_WRAPPER because only the former invalidates cache.
    // If we use RUSTC_WRAPPER, running `cargo auditable` will not trigger a rebuild.
    // The WORKSPACE part is a bit of a misnomer: it will be run for a local crate even if there's just one, not a workspace.
    // Security note:
    // `std::env::current_exe()` is not supposed to be relied on for security - the binary may be moved, etc.
    // But should not a code execution vulnerability since whoever sets this could set RUSTC_WORKSPACE_WRAPPER themselves
    // This would matter if the binary was made setuid, but it isn't, so this should be fine.
    let path_to_this_binary = std::env::current_exe().unwrap();
    command.env("RUSTC_WORKSPACE_WRAPPER", path_to_this_binary);
    let results = command
        .status()
        .expect("Failed to invoke cargo! Make sure it's in your $PATH");
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
