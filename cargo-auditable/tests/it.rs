//! Integration Tests for cargo auditable
use std::{
    collections::HashMap,
    ffi::OsStr,
    io::Write,
    path::PathBuf,
    process::{Command, Output, Stdio},
};

use auditable_serde::{DependencyKind, VersionInfo};
use cargo_metadata::{
    camino::{Utf8Path, Utf8PathBuf},
    Artifact,
};

// Path to cargo-auditable binary under test
const EXE: &str = env!("CARGO_BIN_EXE_cargo-auditable");

// Path to Cargo itself
const CARGO: &str = env!("CARGO");

/// Run cargo auditable with --manifest-path <cargo_toml_path arg> and extra args,
/// returning of map of workspace member names -> produced binaries (bin and cdylib)
/// Reads the AUDITABLE_TEST_TARGET environment variable to determine the target to compile for
fn run_cargo_auditable<P>(
    cargo_toml_path: P,
    args: &[&str],
    env: &[(&str, &OsStr)],
) -> HashMap<String, Vec<Utf8PathBuf>>
where
    P: AsRef<OsStr>,
{
    // run `cargo clean` before performing the build,
    // otherwise already built binaries will be used
    // and we won't actually test the *current* version of `cargo auditable`
    let status = Command::new(CARGO)
        .arg("clean")
        .arg("--manifest-path")
        .arg(&cargo_toml_path)
        .status()
        .unwrap();
    assert!(status.success(), "Failed to invoke `cargo clean`!");

    let mut command = Command::new(EXE);
    command
        .arg("auditable")
        .arg("build")
        .arg("--release")
        .arg("--manifest-path")
        .arg(&cargo_toml_path)
        // We'll parse these to get binary paths
        .arg("--message-format=json")
        .args(args);

    if let Ok(target) = std::env::var("AUDITABLE_TEST_TARGET") {
        if args.iter().all(|arg| !arg.starts_with("--target")) {
            command.arg(format!("--target={target}"));
        }
    }

    for (name, value) in env {
        command.env(name, value);
    }

    let output = command
        // We don't need to read stderr, so inherit for easier test debugging
        .stderr(Stdio::inherit())
        .stdout(Stdio::piped())
        .output()
        .unwrap();

    ensure_build_succeeded(&output);

    let mut bins = HashMap::new();
    std::str::from_utf8(&output.stdout)
        .unwrap()
        .lines()
        .flat_map(|line: &str| {
            let mut binaries = vec![];
            if let Ok(artifact) = serde_json::from_str::<Artifact>(line) {
                // workspace member name is first word in package ID
                let member = artifact
                    .package_id
                    .to_string()
                    .split(' ')
                    .next()
                    .unwrap()
                    .to_string();
                // bin targets are straightforward - use executable
                if let Some(executable) = artifact.executable {
                    binaries.push((member, executable));
                // cdylibs less so
                } else if artifact
                    .target
                    .kind
                    .iter()
                    .any(|kind| kind.as_str() == "cdylib")
                {
                    // Detect files with .so (Linux), .dylib (Mac) and .dll (Windows) extensions
                    artifact
                        .filenames
                        .into_iter()
                        .filter(|f| {
                            f.extension() == Some("dylib")
                                || f.extension() == Some("so")
                                || f.extension() == Some("dll")
                        })
                        .for_each(|f| {
                            binaries.push((member.clone(), f));
                        });
                }
            }
            binaries
        })
        .for_each(|(package, binary)| {
            bins.entry(pkgid_to_bin_name(&package))
                .or_insert(Vec::new())
                .push(binary);
        });
    bins
}

fn pkgid_to_bin_name(pkgid: &str) -> String {
    // the input is string in the format such as
    // "path+file:///home/shnatsel/Code/cargo-auditable/cargo-auditable/tests/fixtures/lib_and_bin_crate#0.1.0"
    // (for full docs see `cargo pkgid`)
    // and we need just the crate name, e.g. "lib_and_bin_crate".
    // Weirdly it doesn't use OS path separator, it always uses '/'
    pkgid
        .rsplit_once('/')
        .unwrap()
        .1
        .split_once('#')
        .unwrap()
        .0
        .to_owned()
}

fn ensure_build_succeeded(output: &Output) {
    if !output.status.success() {
        let stderr = std::io::stderr();
        let mut handle = stderr.lock();
        handle.write_all(&output.stdout).unwrap();
        handle.write_all(&output.stderr).unwrap();
        handle.flush().unwrap();
        panic!("Build with `cargo auditable` failed");
    }
}

fn get_dependency_info(binary: &Utf8Path) -> VersionInfo {
    auditable_info::audit_info_from_file(binary.as_std_path(), Default::default()).unwrap()
}

#[test]
fn test_cargo_auditable_workspaces() {
    // Path to workspace fixture Cargo.toml. See that file for overview of workspace members and their dependencies.
    let workspace_cargo_toml =
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/workspace/Cargo.toml");
    // Run in workspace root with default features
    let bins = run_cargo_auditable(&workspace_cargo_toml, &[], &[]);
    eprintln!("Test fixture binary map: {bins:?}");
    // No binaries for library_crate
    assert!(bins.get("library_crate").is_none());

    // binary_and_cdylib_crate
    let binary_and_cdylib_crate_bins = bins.get("binary_and_cdylib_crate").unwrap();
    match std::env::var("AUDITABLE_TEST_TARGET") {
        // musl targets do not produce cdylibs by default: https://github.com/rust-lang/cargo/issues/8607
        // So when targeting musl, we only check that the binary has been built, not the cdylib.
        Ok(target) if target.contains("musl") => assert!(!binary_and_cdylib_crate_bins.is_empty()),
        // everything else should build both the binary and cdylib
        _ => assert_eq!(binary_and_cdylib_crate_bins.len(), 2),
    }
    for binary in binary_and_cdylib_crate_bins {
        let dep_info = get_dependency_info(binary);
        eprintln!("{binary} dependency info: {dep_info:?}");
        // binary_and_cdylib_crate should have two dependencies, library_crate and itself
        assert!(dep_info.packages.len() == 2);
        assert!(dep_info.packages.iter().any(|p| p.name == "library_crate"));
        assert!(dep_info
            .packages
            .iter()
            .any(|p| p.name == "binary_and_cdylib_crate"));
    }

    // crate_with_features should create a binary with two dependencies, library_crate and itself
    let crate_with_features_bin = &bins.get("crate_with_features").unwrap()[0];
    let dep_info = get_dependency_info(crate_with_features_bin);
    eprintln!("{crate_with_features_bin} dependency info: {dep_info:?}");
    assert!(dep_info.packages.len() == 2);
    assert!(dep_info.packages.iter().any(|p| p.name == "library_crate"));
    assert!(dep_info
        .packages
        .iter()
        .any(|p| p.name == "crate_with_features"));

    // Run enabling binary_and_cdylib_crate feature
    let bins = run_cargo_auditable(
        &workspace_cargo_toml,
        &["--features", "binary_and_cdylib_crate"],
        &[],
    );
    // crate_with_features should now have three dependencies, library_crate binary_and_cdylib_crate and crate_with_features,
    let crate_with_features_bin = &bins.get("crate_with_features").unwrap()[0];
    let dep_info = get_dependency_info(crate_with_features_bin);
    eprintln!("{crate_with_features_bin} dependency info: {dep_info:?}");
    assert!(dep_info.packages.len() == 3);
    assert!(dep_info.packages.iter().any(|p| p.name == "library_crate"));
    assert!(dep_info
        .packages
        .iter()
        .any(|p| p.name == "crate_with_features"));
    assert!(dep_info
        .packages
        .iter()
        .any(|p| p.name == "binary_and_cdylib_crate"));

    // Run without default features
    let bins = run_cargo_auditable(&workspace_cargo_toml, &["--no-default-features"], &[]);
    // crate_with_features should now only depend on itself
    let crate_with_features_bin = &bins.get("crate_with_features").unwrap()[0];
    let dep_info = get_dependency_info(crate_with_features_bin);
    eprintln!("{crate_with_features_bin} dependency info: {dep_info:?}");
    assert!(dep_info.packages.len() == 1);
    assert!(dep_info
        .packages
        .iter()
        .any(|p| p.name == "crate_with_features"));
}

/// This exercises a small real-world project
#[test]
fn test_self_hosting() {
    // Path to workspace fixture Cargo.toml. See that file for overview of workspace members and their dependencies.
    let workspace_cargo_toml =
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../rust-audit-info/Cargo.toml");
    // Run in workspace root with default features
    let bins = run_cargo_auditable(workspace_cargo_toml, &[], &[]);
    eprintln!("Self-hosting binary map: {bins:?}");

    // verify that the dependency info is present at all
    let bin = &bins.get("rust-audit-info").unwrap()[0];
    let dep_info = get_dependency_info(bin);
    eprintln!("{bin} dependency info: {dep_info:?}");
    assert!(dep_info.packages.len() > 1);
    assert!(dep_info
        .packages
        .iter()
        .any(|p| p.name == "rust-audit-info"));
}

#[test]
fn test_lto() {
    // Path to workspace fixture Cargo.toml. See that file for overview of workspace members and their dependencies.
    let workspace_cargo_toml = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures/lto_binary_crate/Cargo.toml");
    // Run in workspace root with default features
    let bins = run_cargo_auditable(workspace_cargo_toml, &[], &[]);
    eprintln!("LTO binary map: {bins:?}");

    // lto_binary_crate should only depend on itself
    let lto_binary_crate_bin = &bins.get("lto_binary_crate").unwrap()[0];
    let dep_info = get_dependency_info(lto_binary_crate_bin);
    eprintln!("{lto_binary_crate_bin} dependency info: {dep_info:?}");
    assert!(dep_info.packages.len() == 1);
    assert!(dep_info
        .packages
        .iter()
        .any(|p| p.name == "lto_binary_crate"));
}

#[test]
fn test_lto_stripped() {
    // Path to workspace fixture Cargo.toml. See that file for overview of workspace members and their dependencies.
    let workspace_cargo_toml = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures/lto_stripped_binary/Cargo.toml");
    // Run in workspace root with default features
    let bins = run_cargo_auditable(workspace_cargo_toml, &[], &[]);
    eprintln!("Stripped binary map: {bins:?}");

    // lto_stripped_binary should only depend on itself
    let lto_stripped_binary_bin = &bins.get("lto_stripped_binary").unwrap()[0];
    let dep_info = get_dependency_info(lto_stripped_binary_bin);
    eprintln!("{lto_stripped_binary_bin} dependency info: {dep_info:?}");
    assert!(dep_info.packages.len() == 1);
    assert!(dep_info
        .packages
        .iter()
        .any(|p| p.name == "lto_stripped_binary"));
}

#[test]
fn test_bin_and_lib_in_one_crate() {
    // Path to workspace fixture Cargo.toml. See that file for overview of workspace members and their dependencies.
    let workspace_cargo_toml = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures/lib_and_bin_crate/Cargo.toml");

    let bins = run_cargo_auditable(workspace_cargo_toml, &["--bin=some_binary"], &[]);
    eprintln!("Test fixture binary map: {bins:?}");

    // lib_and_bin_crate should only depend on itself
    let lib_and_bin_crate_bin = &bins.get("lib_and_bin_crate").unwrap()[0];
    let dep_info = get_dependency_info(lib_and_bin_crate_bin);
    eprintln!("{lib_and_bin_crate_bin} dependency info: {dep_info:?}");
    assert!(dep_info.packages.len() == 1);
    assert!(dep_info
        .packages
        .iter()
        .any(|p| p.name == "lib_and_bin_crate"));
}

/// A previous approach had trouble with build scripts and proc macros.
/// Verify that those still work.
#[test]
fn test_build_script() {
    // Path to workspace fixture Cargo.toml. See that file for overview of workspace members and their dependencies.
    let workspace_cargo_toml = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures/crate_with_build_script/Cargo.toml");

    let bins = run_cargo_auditable(workspace_cargo_toml, &[], &[]);
    eprintln!("Test fixture binary map: {bins:?}");

    // crate_with_build_script should only depend on itself
    let crate_with_build_script_bin = &bins.get("crate_with_build_script").unwrap()[0];
    let dep_info = get_dependency_info(crate_with_build_script_bin);
    eprintln!("{crate_with_build_script_bin} dependency info: {dep_info:?}");
    assert!(dep_info.packages.len() == 1);
    assert!(dep_info
        .packages
        .iter()
        .any(|p| p.name == "crate_with_build_script"));
}

#[test]
fn test_platform_specific_deps() {
    // Path to workspace fixture Cargo.toml. See that file for overview of workspace members and their dependencies.
    let workspace_cargo_toml = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures/platform_specific_deps/Cargo.toml");
    // Run in workspace root with default features
    let bins = run_cargo_auditable(workspace_cargo_toml, &[], &[]);
    eprintln!("Test fixture binary map: {bins:?}");

    let test_target = std::env::var("AUDITABLE_TEST_TARGET");
    if test_target.is_err() || !test_target.unwrap().starts_with("m68k") {
        // 'with_platform_dep' should only depend on 'should_not_be_included' on m68k processors
        // and we're not building for those, so it should be omitted
        let bin = &bins.get("with_platform_dep").unwrap()[0];
        let dep_info = get_dependency_info(bin);
        eprintln!("{bin} dependency info: {dep_info:?}");
        assert!(dep_info.packages.len() == 1);
        assert!(!dep_info
            .packages
            .iter()
            .any(|p| p.name == "should_not_be_included"));
    }
}

#[test]
fn test_build_then_runtime_dep() {
    // Path to workspace fixture Cargo.toml. See that file for overview of workspace members and their dependencies.
    let workspace_cargo_toml = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures/build_then_runtime_dep/Cargo.toml");
    // Run in workspace root with default features
    let bins = run_cargo_auditable(workspace_cargo_toml, &[], &[]);
    eprintln!("Test fixture binary map: {bins:?}");

    // check that the build types are propagated correctly
    let toplevel_crate_bin = &bins.get("top_level_crate").unwrap()[0];
    let dep_info = get_dependency_info(toplevel_crate_bin);
    eprintln!("{toplevel_crate_bin} dependency info: {dep_info:?}");
    assert!(dep_info.packages.len() == 3);
    assert!(dep_info
        .packages
        .iter()
        .any(|p| p.name == "build_dep" && p.kind == DependencyKind::Build));
    assert!(dep_info
        .packages
        .iter()
        .any(|p| p.name == "runtime_dep_of_build_dep" && p.kind == DependencyKind::Build));
}

#[test]
fn test_runtime_then_build_dep() {
    // Path to workspace fixture Cargo.toml. See that file for overview of workspace members and their dependencies.
    let workspace_cargo_toml = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures/runtime_then_build_dep/Cargo.toml");
    // Run in workspace root with default features
    let bins = run_cargo_auditable(workspace_cargo_toml, &[], &[]);
    eprintln!("Test fixture binary map: {bins:?}");

    // check that the build types are propagated correctly
    let toplevel_crate_bin = &bins.get("top_level_crate").unwrap()[0];
    let dep_info = get_dependency_info(toplevel_crate_bin);
    eprintln!("{toplevel_crate_bin} dependency info: {dep_info:?}");
    assert!(dep_info.packages.len() == 3);
    assert!(dep_info
        .packages
        .iter()
        .any(|p| p.name == "runtime_dep" && p.kind == DependencyKind::Runtime));
    assert!(dep_info
        .packages
        .iter()
        .any(|p| p.name == "build_dep_of_runtime_dep" && p.kind == DependencyKind::Build));
}

#[test]
fn test_custom_rustc_path() {
    // Path to workspace fixture Cargo.toml. See that file for overview of workspace members and their dependencies.
    let workspace_cargo_toml = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures/custom_rustc_path/Cargo.toml");
    // locate rustc
    let rustc_path = which::which("rustc").unwrap();
    // Run in workspace root with a custom path to rustc
    let bins = run_cargo_auditable(workspace_cargo_toml, &[], &[("RUSTC", rustc_path.as_ref())]);
    eprintln!("Test fixture binary map: {bins:?}");

    // check that the build types are propagated correctly
    let toplevel_crate_bin = &bins.get("top_level_crate").unwrap()[0];
    let dep_info = get_dependency_info(toplevel_crate_bin);
    eprintln!("{toplevel_crate_bin} dependency info: {dep_info:?}");
    assert!(dep_info.packages.len() == 3);
    assert!(dep_info
        .packages
        .iter()
        .any(|p| p.name == "runtime_dep" && p.kind == DependencyKind::Runtime));
    assert!(dep_info
        .packages
        .iter()
        .any(|p| p.name == "build_dep_of_runtime_dep" && p.kind == DependencyKind::Build));
}

#[test]
fn test_workspace_member_version_info() {
    // Test that `/path/to/cargo-auditable rustc -vV works when compiling a workspace member
    //
    // Never happens with Cargo - it does call `rustc -vV`,
    // but either bypasses the wrapper or doesn't set CARGO_PRIMARY_PACKAGE=true.
    // However it does happen with `sccache`:
    // https://github.com/rust-secure-code/cargo-auditable/issues/87
    let mut command = Command::new(EXE);
    command.env("CARGO_PRIMARY_PACKAGE", "true");
    command.args(["rustc", "-vV"]);

    let status = command.status().unwrap();
    assert!(status.success());
}

#[test]
fn test_wasm() {
    // Path to workspace fixture Cargo.toml. See that file for overview of workspace members and their dependencies.
    let workspace_cargo_toml =
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/wasm_crate/Cargo.toml");
    // Run in workspace root with default features
    run_cargo_auditable(
        workspace_cargo_toml,
        &["--target=wasm32-unknown-unknown"],
        &[],
    );

    // check that the build types are propagated correctly
    let dep_info = get_dependency_info(
        "tests/fixtures/wasm_crate/target/wasm32-unknown-unknown/release/wasm_crate.wasm".into(),
    );
    eprintln!("wasm_crate.wasm dependency info: {dep_info:?}");
    assert_eq!(dep_info.packages.len(), 16);
}
