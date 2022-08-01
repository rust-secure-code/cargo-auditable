#![forbid(unsafe_code)]

mod cargo_auditable;
mod collect_audit_data;
mod object_file;
mod rustc_arguments;
mod rustc_wrapper;
mod target_info;

use std::process::exit;

/// Dispatches the call to either `cargo auditable` when invoked through cargo,
/// or to `rustc_wrapper` when Cargo internals invoke it
fn main() {
    let first_arg = std::env::args_os().nth(1);
    if let Some(arg) = first_arg {
        if arg == "auditable" {
            cargo_auditable::main()
        } else if arg == "rustc" {
            rustc_wrapper::main()
        } else {
            eprintln!("Unrecognized command: {arg:?}");
            exit(1);
        }
    } else {
        eprintln!("'cargo auditable' should be invoked through Cargo");
        exit(1);
    }
}
