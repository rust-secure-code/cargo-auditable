use std::collections::HashMap;

use auditable_serde::{Package, Source, VersionInfo};
use cargo_metadata::{
    semver::{self, Version},
    DependencyKind,
};
use serde::{Deserialize, Serialize};

/// Cargo SBOM precursor format.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SbomPrecursor {
    /// Schema version
    pub version: u32,
    /// Index into the crates array for the root crate
    pub root: usize,
    /// Array of all crates
    pub crates: Vec<Crate>,
    /// Information about rustc used to perform the compilation
    pub rustc: RustcInfo,
}

impl From<SbomPrecursor> for VersionInfo {
    fn from(sbom: SbomPrecursor) -> Self {
        // cargo sbom data format has more nodes than the auditable info format - if a crate is both a build
        // and runtime dependency it will appear twice in the `crates` array.
        // The `VersionInfo` format lists each package only once, with a single `kind` field
        // (Runtime having precedence over other kinds).

        // Firstly, we deduplicate the (name, version) pairs and create a mapping from the
        // original indices in the cargo sbom array to the new index in the auditable info package array.
        let (_, mut packages, indices) = sbom.crates.iter().enumerate().fold(
            (HashMap::new(), Vec::new(), Vec::new()),
            |(mut id_to_index_map, mut packages, mut indices), (index, crate_)| {
                match id_to_index_map.entry(crate_.id.clone()) {
                    std::collections::hash_map::Entry::Occupied(entry) => {
                        // Just store the new index in the indices array
                        indices.push(*entry.get());
                    }
                    std::collections::hash_map::Entry::Vacant(entry) => {
                        let (name, version, source) = parse_fully_qualified_package_id(&crate_.id);
                        // If the entry does not exist, we create it
                        packages.push(Package {
                            name,
                            version,
                            source,
                            // Assume build, if we determine this is a runtime dependency we'll update later
                            kind: auditable_serde::DependencyKind::Build,
                            // We will fill this in later
                            dependencies: Vec::new(),
                            root: index == sbom.root,
                        });
                        entry.insert(packages.len() - 1);
                        indices.push(packages.len() - 1);
                    }
                }
                (id_to_index_map, packages, indices)
            },
        );

        // Traverse the graph as given by the sbom to fill in the dependencies with the new indices.
        //
        // Keep track of whether the dependency is a runtime dependency.
        // If we ever encounter a non-runtime dependency, all deps in the remaining subtree
        // are not runtime dependencies, i.e a runtime dep of a build dep is not recognized as a runtime dep.
        let mut stack = Vec::new();
        stack.push((sbom.root, true));
        while let Some((old_index, is_runtime)) = stack.pop() {
            let crate_ = &sbom.crates[old_index];
            for dep in &crate_.dependencies {
                stack.push((dep.index, dep.kind == DependencyKind::Normal && is_runtime));
            }

            let package = &mut packages[indices[old_index]];
            if is_runtime {
                package.kind = auditable_serde::DependencyKind::Runtime
            };

            for dep in &crate_.dependencies {
                let new_dep_index = indices[dep.index];
                if package.dependencies.contains(&new_dep_index) {
                    continue; // Already added this dependency
                } else if new_dep_index == indices[old_index] {
                    // If the dependency is the same as the package itself, skip it
                    continue;
                } else {
                    package.dependencies.push(new_dep_index);
                }
            }
        }

        VersionInfo {
            packages,
            format: 8,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Crate {
    /// Package ID specification
    pub id: String,
    /// List of target kinds
    pub kind: Vec<String>,
    /// Enabled feature flags
    pub features: Vec<String>,
    /// Dependencies for this crate
    pub dependencies: Vec<Dependency>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Dependency {
    /// Index into the crates array
    pub index: usize,
    /// Dependency kind: "normal", "build", or "dev"
    pub kind: DependencyKind,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RustcInfo {
    /// Compiler version
    pub version: String,
    /// Compiler wrapper
    pub wrapper: Option<String>,
    /// Compiler workspace wrapper
    pub workspace_wrapper: Option<String>,
    /// Commit hash for rustc
    pub commit_hash: String,
    /// Host target triple
    pub host: String,
    /// Verbose version string: `rustc -vV`
    pub verbose_version: String,
}

const CRATES_IO_INDEX: &str = "https://github.com/rust-lang/crates.io-index";

/// Parses a fully qualified package ID spec string into a tuple of (name, version, source).
/// The package ID spec format is defined at https://doc.rust-lang.org/cargo/reference/pkgid-spec.html#package-id-specifications-1
///
/// The fully qualified form of a package ID spec is mentioned in the Cargo documentation,
/// figuring it out is left as an exercise to the reader.
///
/// Adapting the grammar in the cargo doc, the format appears to be :
/// ```norust
/// fully_qualified_spec :=  kind "+" proto "://" hostname-and-path [ "?" query] "#" [ name "@" ] semver
/// query := ( "branch" | "tag" | "rev" ) "=" ref
/// semver := digits "." digits "." digits [ "-" prerelease ] [ "+" build ]
/// kind := "registry" | "git" | "path"
/// proto := "http" | "git" | "file" | ...
/// ```
/// where:
/// - the `[ name "@" ]` segment is elided when the crate name equals the URL's last path
///   segment (i.e. for `path` deps where the directory name matches, and `git` deps where
///   the repo name matches)
/// - the query string is only present for git dependencies (which we can ignore since we don't
///   record git information)
fn parse_fully_qualified_package_id(id: &str) -> (String, Version, Source) {
    let (kind, rest) = id.split_once('+').expect("Package ID to have a kind");
    let (url, rest) = rest
        .split_once('#')
        .expect("Package ID to have version information");
    let source = match (kind, url) {
        ("registry", CRATES_IO_INDEX) => Source::CratesIo,
        ("registry", _) => Source::Registry,
        ("git", _) => Source::Git,
        ("path", _) => Source::Local,
        _ => Source::Other(kind.to_string()),
    };

    // `rest` is usually `name@version`, but cargo elides `name@` when the crate name
    // equals the URL's last path segment. This applies to `path` deps and to git deps
    // pointing at a repo whose name matches the crate (e.g. top-level `rayon`); sub-crates
    // in the same repo still carry the name explicitly.
    //
    //   path+file:///abs/path/sample-package#0.1.0
    //   git+https://github.com/rayon-rs/rayon?branch=foo#1.11.0
    //   git+https://github.com/rayon-rs/rayon?branch=foo#rayon-core@1.13.0
    if let Some((name, version)) = rest.split_once('@') {
        (
            name.to_string(),
            semver::Version::parse(version).expect("Version to be valid SemVer"),
            source,
        )
    } else {
        // Recover the elided name from the URL's last path segment.
        // Strip the optional `?query` first; accept `\` for Windows local paths.
        let path = url.split_once('?').map(|(p, _)| p).unwrap_or(url);
        let name = path
            .rsplit(['/', '\\'])
            .next()
            .filter(|segment| !segment.is_empty())
            .expect("Package ID URL to end with a package name");
        (
            name.to_string(),
            semver::Version::parse(rest).expect("Version to be valid SemVer"),
            source,
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn assert_id(id: &str, expected_name: &str, expected_version: &str, expected_source: Source) {
        let (name, version, source) = parse_fully_qualified_package_id(id);
        assert_eq!(name, expected_name, "name mismatch for {id}");
        assert_eq!(
            version.to_string(),
            expected_version,
            "version mismatch for {id}"
        );
        assert_eq!(source, expected_source, "source mismatch for {id}");
    }

    #[test]
    fn registry_with_name() {
        assert_id(
            "registry+https://github.com/rust-lang/crates.io-index#zerocopy@0.8.16",
            "zerocopy",
            "0.8.16",
            Source::CratesIo,
        );
    }

    #[test]
    fn path_with_elided_name() {
        // Directory name matches crate name, so cargo elides `name@`.
        assert_id(
            "path+file:///tmp/sample-package#0.1.0",
            "sample-package",
            "0.1.0",
            Source::Local,
        );
    }

    #[test]
    fn path_with_explicit_name() {
        // Directory name differs from crate name, so cargo emits `name@`.
        assert_id(
            "path+file:///tmp/some-dir#different-name@0.1.0",
            "different-name",
            "0.1.0",
            Source::Local,
        );
    }

    #[test]
    fn git_with_explicit_name() {
        // Sub-crate inside a git repo: name is present.
        assert_id(
            "git+https://github.com/rayon-rs/rayon?branch=main#rayon-core@1.13.0",
            "rayon-core",
            "1.13.0",
            Source::Git,
        );
    }

    #[test]
    fn git_with_elided_name() {
        // Crate name matches the repo's last path segment, so cargo elides
        // `name@`. Regression test: this used to panic.
        assert_id(
            "git+https://github.com/rayon-rs/rayon?branch=main#1.11.0",
            "rayon",
            "1.11.0",
            Source::Git,
        );
    }

    #[test]
    fn git_with_elided_name_no_query() {
        assert_id(
            "git+https://github.com/rayon-rs/rayon#1.11.0",
            "rayon",
            "1.11.0",
            Source::Git,
        );
    }
}
