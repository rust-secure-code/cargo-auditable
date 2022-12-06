use crate::cargo_arguments::CargoArgs;
use std::{env, process::Command};

pub fn main() {
    // set the RUSTFLAGS environment variable to inject our object and call Cargo with all the Cargo args

    // Cargo sets the path to itself in the `CARGO` environment variable:
    // https://doc.rust-lang.org/cargo/reference/environment-variables.html#environment-variables-cargo-sets-for-3rd-party-subcommands
    // This is also useful for using `cargo auditable` as a drop-in replacement for Cargo.
    let cargo = env::var_os("CARGO").unwrap_or_else(|| "cargo".into());
    let mut command = Command::new(cargo);
    // Pass along all our arguments; we don't currently have any args specific to `cargo auditable`
    // We skip argv[0] which is the path to this binary and the first argument which is 'auditable' passed by Cargo
    command.args(env::args_os().skip(2));
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

    // Pass on the arguments we received so that they can be inspected later.
    // We're interested in flags like `--offline` and `--config` which have to be passed to `cargo metadata` later.
    // The shell has already split them for us and we don't want to mangle them, but we need to round-trip them
    // through a string. Since we already depend on `serde-json` and it does the job, use JSON.
    // This doesn't support non-UTF8 arguments, but `cargo_metadata` crate doesn't support them either,
    // so this is not an issue right now.
    // If it ever becomes one, we could use the `serde-bytes-repr` crate for a clean round-trip.
    let args = CargoArgs::from_args();
    let args_in_json = serde_json::to_string(&args).unwrap();
    command.env("CARGO_AUDITABLE_ORIG_ARGS", args_in_json);

    let results = command
        .status()
        .expect("Failed to invoke cargo! Make sure it's in your $PATH");
    std::process::exit(results.code().unwrap());
}