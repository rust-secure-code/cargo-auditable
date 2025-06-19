use std::collections::HashMap;

use auditable_serde::{Package, Source, VersionInfo};
use cargo_metadata::DependencyKind;
use cargo_util_schemas::core::{PackageIdSpec, SourceKind};
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
        // (Runtime having precence over other kinds).

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
                        // If the entry does not exist, we create it
                        packages.push(Package {
                            name: crate_.id.name().to_string(),
                            version: crate_.id.version().expect("Package to have version"),
                            source: match crate_.id.kind() {
                                Some(SourceKind::Path) => Source::Local,
                                Some(SourceKind::Git(_)) => Source::Git,
                                Some(_) => Source::Registry,
                                None => Source::CratesIo,
                            },
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

        VersionInfo { packages }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Crate {
    /// Package ID specification
    pub id: PackageIdSpec,
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
