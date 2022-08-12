//! Integration Tests for cargo auditable
use std::{
    collections::HashMap,
    ffi::OsStr,
    fs::File,
    io::{Read, Write},
    path::PathBuf,
    process::{Command, Output, Stdio},
    str::FromStr,
};

use auditable_serde::{DependencyKind, VersionInfo};
use cargo_metadata::{
    camino::{Utf8Path, Utf8PathBuf},
    Artifact,
};
use miniz_oxide::inflate::decompress_to_vec_zlib;

// Path to cargo-auditable binary under test
const EXE: &str = env!("CARGO_BIN_EXE_cargo-auditable");

/// Run cargo auditable with --manifest-path <cargo_toml_path arg> and extra args,
/// returning of map of workspace member names -> produced binaries (bin and cdylib)
/// Reads the AUDITABLE_TEST_TARGET environment variable to determine the target to compile for
fn run_cargo_auditable<P>(cargo_toml_path: P, args: &[&str]) -> HashMap<String, Vec<Utf8PathBuf>>
where
    P: AsRef<OsStr>,
{
    let mut command = Command::new(EXE);
    command
        .arg("auditable")
        .arg("build")
        .arg("--manifest-path")
        .arg(cargo_toml_path)
        // We'll parse these to get binary paths
        .arg("--message-format=json")
        .args(args);

    if let Ok(target) = std::env::var("AUDITABLE_TEST_TARGET") {
        command.arg(format!("--target={target}"));
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
            bins.entry(package).or_insert(Vec::new()).push(binary);
        });
    bins
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
    // TODO merge with rust-audit-info and move into auditable extract?
    let mut f = File::open(binary).unwrap();
    let mut data = Vec::new();
    f.read_to_end(&mut data).unwrap();
    let compressed_audit_data = auditable_extract::raw_auditable_data(&data).unwrap();
    let decompressed_data =
        decompress_to_vec_zlib(compressed_audit_data).expect("Failed to decompress audit info");
    let decompressed_data = String::from_utf8(decompressed_data).unwrap();
    auditable_serde::VersionInfo::from_str(&decompressed_data).unwrap()
}

#[test]
fn test_cargo_auditable_workspaces() {
    // Path to workspace fixture Cargo.toml. See that file for overview of workspace members and their dependencies.
    let workspace_cargo_toml =
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/workspace/Cargo.toml");
    // Run in workspace root with default features
    let bins = run_cargo_auditable(&workspace_cargo_toml, &[]);
    eprintln!("Test fixture binary map: {:?}", bins);
    // No binaries for library_crate
    assert!(bins.get("library_crate").is_none());

    // binary_and_cdylib_crate
    let binary_and_cdylib_crate_bins = bins.get("binary_and_cdylib_crate").unwrap();
    match std::env::var("AUDITABLE_TEST_TARGET") {
        // musl targets do not produce cdylibs by default: https://github.com/rust-lang/cargo/issues/8607
        // So when targeting musl, we only check that the binary has been built, not the cdylib.
        Ok(target) if target.contains("musl") => assert!(binary_and_cdylib_crate_bins.len() >= 1),
        // everything else should build both the binary and cdylib
        _ => assert_eq!(binary_and_cdylib_crate_bins.len(), 2),
    }
    for binary in binary_and_cdylib_crate_bins {
        let dep_info = get_dependency_info(binary);
        eprintln!("{} dependency info: {:?}", binary, dep_info);
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
    let dep_info = get_dependency_info(&crate_with_features_bin);
    eprintln!(
        "{} dependency info: {:?}",
        crate_with_features_bin, dep_info
    );
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
    );
    // crate_with_features should now have three dependencies, library_crate binary_and_cdylib_crate and crate_with_features,
    let crate_with_features_bin = &bins.get("crate_with_features").unwrap()[0];
    let dep_info = get_dependency_info(crate_with_features_bin);
    eprintln!(
        "{} dependency info: {:?}",
        crate_with_features_bin, dep_info
    );
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
    let bins = run_cargo_auditable(&workspace_cargo_toml, &["--no-default-features"]);
    // crate_with_features should now only depend on itself
    let crate_with_features_bin = &bins.get("crate_with_features").unwrap()[0];
    let dep_info = get_dependency_info(crate_with_features_bin);
    eprintln!(
        "{} dependency info: {:?}",
        crate_with_features_bin, dep_info
    );
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
    let bins = run_cargo_auditable(&workspace_cargo_toml, &[]);
    eprintln!("Self-hosting binary map: {:?}", bins);

    // verify that the dependency info is present at all
    let bin = &bins.get("rust-audit-info").unwrap()[0];
    let dep_info = get_dependency_info(bin);
    eprintln!("{} dependency info: {:?}", bin, dep_info);
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
    let bins = run_cargo_auditable(&workspace_cargo_toml, &["--release"]);
    eprintln!("LTO binary map: {:?}", bins);

    // lto_binary_crate should only depend on itself
    let lto_binary_crate_bin = &bins.get("lto_binary_crate").unwrap()[0];
    let dep_info = get_dependency_info(lto_binary_crate_bin);
    eprintln!("{} dependency info: {:?}", lto_binary_crate_bin, dep_info);
    assert!(dep_info.packages.len() == 1);
    assert!(dep_info
        .packages
        .iter()
        .any(|p| p.name == "lto_binary_crate"));
}

#[test]
fn test_bin_and_lib_in_one_crate() {
    // Path to workspace fixture Cargo.toml. See that file for overview of workspace members and their dependencies.
    let workspace_cargo_toml = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures/lib_and_bin_crate/Cargo.toml");

    let bins = run_cargo_auditable(&workspace_cargo_toml, &["--bin=some_binary"]);
    eprintln!("LTO binary map: {:?}", bins);

    // lib_and_bin_crate should only depend on itself
    let lib_and_bin_crate_bin = &bins.get("lib_and_bin_crate").unwrap()[0];
    let dep_info = get_dependency_info(lib_and_bin_crate_bin);
    eprintln!("{} dependency info: {:?}", lib_and_bin_crate_bin, dep_info);
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

    let bins = run_cargo_auditable(&workspace_cargo_toml, &[]);
    eprintln!("LTO binary map: {:?}", bins);

    // crate_with_build_script should only depend on itself
    let crate_with_build_script_bin = &bins.get("crate_with_build_script").unwrap()[0];
    let dep_info = get_dependency_info(crate_with_build_script_bin);
    eprintln!(
        "{} dependency info: {:?}",
        crate_with_build_script_bin, dep_info
    );
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
    let bins = run_cargo_auditable(&workspace_cargo_toml, &[]);
    eprintln!("Test fixture binary map: {:?}", bins);

    let test_target = std::env::var("AUDITABLE_TEST_TARGET");
    if test_target.is_err() || !test_target.unwrap().starts_with("m68k") {
        // 'with_platform_dep' should only depend on 'should_not_be_included' on m68k processors
        // and we're not building for those, so it should be omitted
        let bin = &bins.get("with_platform_dep").unwrap()[0];
        let dep_info = get_dependency_info(&bin);
        eprintln!("{} dependency info: {:?}", bin, dep_info);
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
    let bins = run_cargo_auditable(&workspace_cargo_toml, &[]);
    eprintln!("Test fixture binary map: {:?}", bins);

    // check that the build types are propagated correctly
    let toplevel_crate_bin = &bins.get("top_level_crate").unwrap()[0];
    let dep_info = get_dependency_info(toplevel_crate_bin);
    eprintln!("{} dependency info: {:?}", toplevel_crate_bin, dep_info);
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
    let bins = run_cargo_auditable(&workspace_cargo_toml, &[]);
    eprintln!("Test fixture binary map: {:?}", bins);

    // check that the build types are propagated correctly
    let toplevel_crate_bin = &bins.get("top_level_crate").unwrap()[0];
    let dep_info = get_dependency_info(toplevel_crate_bin);
    eprintln!("{} dependency info: {:?}", toplevel_crate_bin, dep_info);
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