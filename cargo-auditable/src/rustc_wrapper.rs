use std::{process::Command, env, ffi::OsString};

use crate::{collect_audit_data, target_info, object_file, rustc_arguments};

use std::io::BufRead;

pub fn main() {
    // This code is called only if RUSTC_WORKSPACE_WRAPPER is set,
    // so it is only called for crates in the current workspace,
    // or the sole local crate being built if there's no workspace.

    let mut command = rustc_command();

    let args = rustc_arguments::parse_args().unwrap();

    // Only inject arguments into crate types 'bin' and 'cdylib'
    // What if there are multiple types, you might ask? I have no idea!
    // TODO: check if crates that are both rlib and bin actually work
    if args.crate_types.contains(&"bin".to_owned()) || args.crate_types.contains(&"cdylib".to_owned()) {
        // Get the audit data to embed
        let contents: Vec<u8> = collect_audit_data::compressed_dependency_list(&args);
        // write the audit info to an object file
        let target_triple = args.target.unwrap_or(rustc_host_target_triple());
        let target_info = target_info::rustc_target_info(&target_triple);
        let binfile = object_file::create_metadata_file(
            &target_info,
            &target_triple,
            &contents,
            "AUDITABLE_VERSION_INFO", // TODO: make a constant and version it?
        );
        // Place the audit data in the output dir.
        // We can place it anywhere really, the only concern is clutter and name collisions,
        // and the target dir is locked so we're probably good
        let filename = format!("{}_audit_data.o", args.crate_name);
        let path = args.out_dir.clone().join(filename);
        std::fs::write(&path, binfile).expect("Unable to write output file");

        // Modify the rustc command to link the object file with audit data
        let mut linker_command = OsString::from("-Clink-arg=");
        linker_command.push(&path);
        command.arg(linker_command);
        // Prevent the symbol from being removed as unused by the linker. --require-defined is
        // stronger than --undefined, in that both attempt to include a symbol in the output
        // but the former fails if the symbol is undefined. lld doesn't support --require-defined
        // though, so attempt to use --undefined if that's being used.
        if command
            .get_args()
            .any(|arg| arg.to_string_lossy().ends_with("-fuse-ld=lld"))
        {
            command.arg("-Clink-arg=-Wl,--undefined=AUDITABLE_VERSION_INFO");
        } else {
            command.arg("-Clink-arg=-Wl,--require-defined=AUDITABLE_VERSION_INFO");
        }
    }

    // Invoke rustc
    let results = command.status().expect("Failed to invoke rustc! Make sure it's in your $PATH");
    std::process::exit(results.code().unwrap());
}

/// Creates a rustc command line and populates arguments from arguments passed to us.
fn rustc_command() -> Command {
    let mut command = Command::new("rustc");
    // Pass along all the arguments that Cargo meant to pass to rustc
    // We skip the path to our binary as well as the first argument passed by Cargo which is always "rustc"
    command.args(env::args_os().skip(2));
    command
}

/// Returns the default target triple for the rustc we're running
fn rustc_host_target_triple() -> String {
    // TODO: does this still work when rustup is configured to cross-compile by default, e.g. linux-gnu to linux-musl?
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