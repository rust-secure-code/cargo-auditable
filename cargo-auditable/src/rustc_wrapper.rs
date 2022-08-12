use std::{env, ffi::OsString, process::Command};

use crate::{collect_audit_data, object_file, rustc_arguments, target_info};

use std::io::BufRead;

pub fn main() {
    let mut command = rustc_command();

    // Binaries and C dynamic libraries are not built as non-primary packages,
    // so this should not cause issues with Cargo caches.
    if env::var_os("CARGO_PRIMARY_PACKAGE").is_some() {
        let args = rustc_arguments::parse_args().unwrap();

        // Only inject audit data into crate types 'bin' and 'cdylib'
        if args.crate_types.contains(&"bin".to_owned())
            || args.crate_types.contains(&"cdylib".to_owned())
        {
            // Get the audit data to embed
            let target_triple = args.target.clone().unwrap_or_else(rustc_host_target_triple);
            let contents: Vec<u8> =
                collect_audit_data::compressed_dependency_list(&args, &target_triple);
            // write the audit info to an object file
            let target_info = target_info::rustc_target_info(&target_triple);
            let binfile = object_file::create_metadata_file(
                &target_info,
                &target_triple,
                &contents,
                "AUDITABLE_VERSION_INFO",
            );
            // Place the audit data in the output dir.
            // We can place it anywhere really, the only concern is clutter and name collisions,
            // and the target dir is locked so we're probably good
            let filename = format!("{}_audit_data.o", args.crate_name);
            let path = args.out_dir.join(filename);
            std::fs::write(&path, binfile).expect("Unable to write output file");

            // Modify the rustc command to link the object file with audit data
            let mut linker_command = OsString::from("-Clink-arg=");
            linker_command.push(&path);
            command.arg(linker_command);
            // Prevent the symbol from being removed as unused by the linker
            if target_triple.contains("-apple-") {
                command.arg("-Clink-arg=-Wl,-u,_AUDITABLE_VERSION_INFO");
            } else {
                command.arg("-Clink-arg=-Wl,--undefined=AUDITABLE_VERSION_INFO");
            }
        }
    }

    // Invoke rustc
    let results = command
        .status()
        .expect("Failed to invoke rustc! Make sure it's in your $PATH");
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
