use std::{
    env,
    ffi::{OsStr, OsString},
    process::Command,
};

use crate::{
    binary_file, collect_audit_data, list_features::list_features, platform_detection::{is_apple, is_msvc, is_wasm}, rustc_arguments, target_info
};

use std::io::BufRead;

pub fn main(rustc_path: &OsStr) {
    let mut command = rustc_command(rustc_path);

    // Binaries and C dynamic libraries are not built as non-primary packages,
    // so this should not cause issues with Cargo caches.
    if env::var_os("CARGO_PRIMARY_PACKAGE").is_some() {
        let arg_parsing_result = rustc_arguments::parse_args();
        if let Ok(args) = rustc_arguments::parse_args() {
            // Only inject audit data into crate types 'bin' and 'cdylib',
            // and only if --print is not specified (which disables compilation)
            if args.print.is_empty()
                && (args.crate_types.contains(&"bin".to_owned())
                    || args.crate_types.contains(&"cdylib".to_owned()))
            {
                // Get the audit data to embed
                let target_triple = args
                    .target
                    .clone()
                    .unwrap_or_else(|| rustc_host_target_triple(rustc_path));
                // work around Cargo passing nonexistent features to rustc when the "dep:" syntax is used:
                // https://github.com/rust-secure-code/cargo-auditable/issues/124
                let existing_features = match list_features(&args.crate_name) {
                    Ok(features) => Some(features),
                    Err(e) => {
                        eprintln!("ERROR: failed to list Cargo features for package: {e}\n\
                        This is a bug in cargo-auditable. Please report it!\
                        The build will continue, but compilation might fail in rare cases.");
                        None
                    },
                };
                let contents: Vec<u8> =
                    collect_audit_data::compressed_dependency_list(&args, &target_triple, &existing_features);
                // write the audit info to an object file
                let target_info = target_info::rustc_target_info(rustc_path, &target_triple);
                let binfile = binary_file::create_binary_file(
                    &target_info,
                    &target_triple,
                    &contents,
                    "AUDITABLE_VERSION_INFO",
                );
                if let Some(file) = binfile {
                    // Place the audit data in the output dir.
                    // We can place it anywhere really, the only concern is clutter and name collisions,
                    // and the target dir is locked so we're probably good
                    let filename = format!("{}_audit_data.o", args.crate_name);
                    let path = args.out_dir.join(filename);
                    std::fs::write(&path, file).expect("Unable to write output file");

                    // Modify the rustc command to link the object file with audit data
                    let mut linker_command = OsString::from("-Clink-arg=");
                    linker_command.push(&path);
                    command.arg(linker_command);
                    // Prevent the symbol from being removed as unused by the linker
                    if is_apple(&target_info) {
                        command.arg("-Clink-arg=-Wl,-u,_AUDITABLE_VERSION_INFO");
                    } else if is_msvc(&target_info) {
                        command.arg("-Clink-arg=/INCLUDE:AUDITABLE_VERSION_INFO");
                    } else if is_wasm(&target_info) {
                        // We don't emit the symbol name in WASM, so nothing to do
                    } else {
                        command.arg("-Clink-arg=-Wl,--undefined=AUDITABLE_VERSION_INFO");
                    }
                } else {
                    // create_binary_file() returned None, indicating an unsupported architecture
                    eprintln!("WARNING: target '{target_triple}' is not supported by 'cargo auditable'!\n\
                    The build will continue, but no audit data will be injected into the binary.");
                }
            }
        } else {
            // Failed to parse rustc arguments.

            // This may be due to a `rustc -vV` call, or similar non-compilation command.
            // This never happens with Cargo - it does call `rustc -vV`,
            // but either bypasses the wrapper or doesn't set CARGO_PRIMARY_PACKAGE=true.
            // However it does happen with `sccache`:
            // https://github.com/rust-secure-code/cargo-auditable/issues/87
            // This is probably a bug in `sccache`, but it's easier to fix here.

            // There are many non-compilation flags (and they can be compound),
            // so parsing them properly adds a lot of complexity.
            // So we just check if `--crate-name` is passed and if not,
            // assume that it's a non-compilation command.
            if env::args_os()
                .skip(2)
                .any(|arg| arg == OsStr::new("--crate-name"))
            {
                // this was a compilation command, bail
                arg_parsing_result.unwrap();
            }
            // for commands like `rustc --version` we just pass on the arguments without changes
        }
    }

    // Invoke rustc
    let results = command.status().unwrap_or_else(|err| {
        let mut command_with_args: Vec<&OsStr> = vec![command.get_program()];
        command_with_args.extend(command.get_args());
        eprintln!(
            "Failed to invoke rustc! Make sure it's in your $PATH\n\
                The error was: {}\n\
                The attempted call was: {:?}",
            err, command_with_args,
        );
        std::process::exit(1);
    });
    std::process::exit(results.code().unwrap());
}

/// Creates a rustc command line and populates arguments from arguments passed to us.
fn rustc_command(rustc_path: &OsStr) -> Command {
    let mut command = Command::new(rustc_path);
    // Pass along all the arguments that Cargo meant to pass to rustc
    // We skip the path to our binary as well as the first argument passed by Cargo,
    // which is the path to rustc to use (or just "rustc")
    command.args(env::args_os().skip(2));
    command
}

/// Returns the default target triple for the rustc we're running
fn rustc_host_target_triple(rustc_path: &OsStr) -> String {
    Command::new(rustc_path)
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
