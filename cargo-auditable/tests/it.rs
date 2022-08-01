//! Integration Tests for cargo auditable
use std::{
    collections::HashMap,
    ffi::OsStr,
    fs::File,
    io::{Read, Write},
    path::PathBuf,
    process::{Command, Stdio, Output},
    str::FromStr,
};

use auditable_serde::VersionInfo;
use cargo_metadata::{
    camino::{Utf8Path, Utf8PathBuf},
    Artifact,
};
use miniz_oxide::inflate::decompress_to_vec_zlib;

// Path to cargo-auditable binary under test
const EXE: &str = env!("CARGO_BIN_EXE_cargo-auditable");

/// Run cargo auditable with --manifest-path <cargo_toml_path arg> and extra args,
/// returning of map of workspace member names -> produced binaries (bin and cdylib)
fn run_cargo_auditable<P>(cargo_toml_path: P, args: &[&str]) -> HashMap<String, Vec<Utf8PathBuf>>
where
    P: AsRef<OsStr>,
{
    let output = Command::new(EXE)
        .arg("auditable")
        .arg("build")
        .arg("--manifest-path")
        .arg(cargo_toml_path)
        // We'll parse these to get binary paths
        .arg("--message-format=json")
        .args(args)
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
                    // Filter out rlibs, assume everything else is a cdylib as --bin artifacts
                    // have a separate executable artifact message, and we don't use other types e.g
                    // staticlib in the test fixtures
                    artifact
                        .filenames
                        .into_iter()
                        .filter(|f| f.extension() != Some("rlib"))
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
    if ! output.status.success() {
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

    // binary_and_cdylib_crate should have two dependencies, library_crate and itself
    let binary_and_cdylib_crate_bin = &bins.get("binary_and_cdylib_crate").unwrap()[0];
    let dep_info = get_dependency_info(binary_and_cdylib_crate_bin);
    eprintln!(
        "{} dependency info: {:?}",
        binary_and_cdylib_crate_bin, dep_info
    );
    assert!(dep_info.packages.len() == 2);
    assert!(dep_info.packages.iter().any(|p| p.name == "library_crate"));
    assert!(dep_info
        .packages
        .iter()
        .any(|p| p.name == "binary_and_cdylib_crate"));

    // crate_with_features should create a binary and cdylib each with two dependencies, library_crate and itself
    let crate_with_features_bins = &bins.get("crate_with_features").unwrap();
    assert!(crate_with_features_bins.len() == 2);
    let dep_info = get_dependency_info(&crate_with_features_bins[0]);
    eprintln!(
        "{} dependency info: {:?}",
        binary_and_cdylib_crate_bin, dep_info
    );
    assert!(dep_info.packages.len() == 2);
    assert!(dep_info.packages.iter().any(|p| p.name == "library_crate"));
    assert!(dep_info
        .packages
        .iter()
        .any(|p| p.name == "crate_with_features"));

    let dep_info = get_dependency_info(&crate_with_features_bins[1]);
    eprintln!(
        "{} dependency info: {:?}",
        binary_and_cdylib_crate_bin, dep_info
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
        binary_and_cdylib_crate_bin, dep_info
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
        binary_and_cdylib_crate_bin, dep_info
    );
    assert!(dep_info.packages.len() == 1);
    assert!(dep_info
        .packages
        .iter()
        .any(|p| p.name == "crate_with_features"));
}

/// This exercises a real-world project with complications such as proc macros
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
