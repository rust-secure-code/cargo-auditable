#![forbid(unsafe_code)]

//! Parses and serializes the JSON dependency tree embedded in executables by the
//! [`cargo auditable`](https://github.com/rust-secure-code/cargo-auditable).
//!
//! This crate defines the data structures that a serialized to/from JSON
//! and implements the serialization/deserialization routines via `serde`.
//! It also provides optional conversions from [`cargo metadata`](https://docs.rs/cargo_metadata/)
//! and to [`Cargo.lock`](https://docs.rs/cargo-lock) formats.
//! 
//! The [`VersionInfo`] struct is where all the magic happens, see the docs on it for more info.
//! 
//! ## Basic usage
//!
//! The following snippet demonstrates full extraction pipeline, including
//! platform-specific executable handling via
//! [`auditable-extract`](http://docs.rs/auditable-serde/) and decompression
//! using the safe-Rust [`miniz_oxide`](http://docs.rs/miniz_oxide/):
//!
//! ```rust,ignore
//! use std::io::{Read, BufReader};
//! use std::{error::Error, fs::File, str::FromStr};
//!
//! fn main() -> Result<(), Box<dyn Error>> {
//!     // Read the input
//!     let f = File::open("target/release/hello-world")?;
//!     let mut f = BufReader::new(f);
//!     let mut input_binary = Vec::new();
//!     f.read_to_end(&mut input_binary)?;
//!     // Extract the compressed audit data
//!     let compressed_audit_data = auditable_extract::raw_auditable_data(&input_binary)?;
//!     // Decompress it with your Zlib implementation of choice. We recommend miniz_oxide
//!     use miniz_oxide::inflate::decompress_to_vec_zlib;
//!     let decompressed_data = decompress_to_vec_zlib(&compressed_audit_data)
//!         .map_err(|_| "Failed to decompress audit data")?;
//!     let decompressed_data = String::from_utf8(decompressed_data)?;
//!     println!("{}", decompressed_data);
//!     // Parse the audit data to Rust data structures
//!     let dependency_tree = auditable_serde::VersionInfo::from_str(&decompressed_data);
//!     Ok(())
//! }
//! ```

mod validation;

use validation::RawVersionInfo;

use serde::{Deserialize, Serialize};

use std::str::FromStr;
#[cfg(feature = "toml")]
use cargo_lock;
#[cfg(feature = "toml")]
use std::convert::TryInto;
#[cfg(any(feature = "from_metadata",feature = "toml"))]
use std::convert::TryFrom;
#[cfg(feature = "from_metadata")]

#[cfg(feature = "from_metadata")]
use std::{error::Error, cmp::Ordering::*, cmp::min, fmt::Display, collections::HashMap};

/// Dependency tree embedded in the binary.
///
/// Implements `Serialize` and `Deserialize` traits from `serde`, so you can use
/// [all the usual methods from serde-json](https://docs.rs/serde_json/1.0.57/serde_json/#functions)
/// to read and write it.
///
/// `from_str()` that parses JSON is also implemented for your convenience:
/// ```rust
/// use auditable_serde::VersionInfo;
/// use std::str::FromStr;
/// let json_str = r#"{"packages":[{
///     "name":"adler",
///     "version":"0.2.3",
///     "source":"registry"
/// }]}"#;
/// let info = VersionInfo::from_str(json_str).unwrap();
/// assert_eq!(&info.packages[0].name, "adler");
/// ```
///
/// ## Optional features
///
/// If the `from_metadata` feature is enabled, a conversion from 
/// [`cargo_metadata::Metadata`](https://docs.rs/cargo_metadata/0.11.1/cargo_metadata/struct.Metadata.html)
/// is possible via the `TryFrom` trait. This is the preferred way to construct this structure.
/// An example demonstrating that can be found
/// [here](https://github.com/rust-secure-code/cargo-auditable/blob/master/auditable-serde/examples/from-metadata.rs).
///
/// If the `toml` feature is enabled, a conversion into the [`cargo_lock::Lockfile`](https://docs.rs/cargo-lock/)
/// struct is possible via the `TryFrom` trait. This can be useful if you need to interoperate with tooling
/// that consumes the `Cargo.lock` file format. An example demonstrating it can be found
/// [here](https://github.com/rust-secure-code/cargo-auditable/blob/master/auditable-serde/examples/json-to-toml.rs).
#[derive(Serialize, Deserialize, Debug, PartialEq, Eq, PartialOrd, Ord, Clone)]
#[serde(try_from = "RawVersionInfo")]
pub struct VersionInfo {
    pub packages: Vec<Package>,
}

/// A single package in the dependency tree
#[derive(Serialize, Deserialize, Debug, PartialEq, Eq, PartialOrd, Ord, Clone)]
pub struct Package {
    /// Crate name specified in the `name` field in Cargo.toml file. Examples: "libc", "rand"
    pub name: String,
    /// The package's version in the [semantic version](https://semver.org) format. 
    pub version: semver::Version,
    /// Currently "git", "local", "crates.io" or "registry". Designed to be extensible with other revision control systems, etc.
    pub source: Source,
    /// "build" or "runtime". May be omitted if set to "runtime".
    /// If it's both a build and a runtime dependency, "runtime" is recorded.
    #[serde(default)]
    #[serde(skip_serializing_if = "is_default")]
    pub kind: DependencyKind,
    /// Packages are stored in an ordered array both in the `VersionInfo` struct and in JSON.
    /// Here we refer to each package by its index in the array.
    /// May be omitted if the list is empty.
    #[serde(default)]
    #[serde(skip_serializing_if = "is_default")]
    pub dependencies: Vec<usize>,
    /// Whether this is the root package in the dependency tree.
    /// There should only be one root package.
    /// May be omitted if set to `false`.
    #[serde(default)]
    #[serde(skip_serializing_if = "is_default")]
    pub root: bool,
}

/// Serializes to "git", "local", "crates.io" or "registry". Designed to be extensible with other revision control systems, etc.
#[non_exhaustive]
#[derive(Serialize, Deserialize, Debug, PartialEq, Eq, PartialOrd, Ord, Clone)]
#[serde(from = "&str")]
#[serde(into = "String")]
pub enum Source {
    CratesIo,
    Git,
    Local,
    Registry,
    Other(String)
}

impl From<&str> for Source {
    fn from(s: &str) -> Self {
        match s {
            "crates.io" => Self::CratesIo,
            "git" => Self::Git,
            "local" => Self::Local,
            "registry" => Self::Registry,
            other_str => Self::Other(other_str.to_string())
        }
    }
}

impl From<Source> for String {
    fn from(s: Source) -> String {
        match s {
            Source::CratesIo => "crates.io".to_owned(),
            Source::Git =>  "git".to_owned(),
            Source::Local =>  "local".to_owned(),
            Source::Registry =>  "registry".to_owned(),
            Source::Other(string) => string
        }
    }
}

#[cfg(feature = "from_metadata")]
impl From<&cargo_metadata::Source> for Source {
    fn from(meta_source: &cargo_metadata::Source) -> Self {
        match meta_source.repr.as_str() {
            "registry+https://github.com/rust-lang/crates.io-index" => Source::CratesIo,
            source => Source::from(source.split('+').next()
                .expect("Encoding of source strings in `cargo metadata` has changed!"))
        }
    }
}

/// The fields are ordered from weakest to strongest so that casting to integer would make sense
#[derive(Serialize, Deserialize, Debug, PartialEq, Eq, PartialOrd, Ord, Copy, Clone)]
pub enum DependencyKind {
    #[serde(rename = "build")]
    Build,
    #[serde(rename = "runtime")]
    Runtime,
}

impl Default for DependencyKind {
    fn default() -> Self {
        DependencyKind::Runtime
    }
}

/// The fields are ordered from weakest to strongest so that casting to integer would make sense
#[cfg(feature = "from_metadata")]
#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Copy, Clone)]
enum PrivateDepKind {
    Development,
    Build,
    Runtime,
}

#[cfg(feature = "from_metadata")]
impl From<PrivateDepKind> for DependencyKind {
    fn from(priv_kind: PrivateDepKind) -> Self {
        match priv_kind {
            PrivateDepKind::Development => panic!("Cannot convert development dependency to serializable format"),
            PrivateDepKind::Build => DependencyKind::Build,
            PrivateDepKind::Runtime => DependencyKind::Runtime,
        }
    }
}

fn is_default<T: Default + PartialEq> (value: &T) -> bool {
    let default_value = T::default();
    value == &default_value
}

impl FromStr for VersionInfo {
    type Err = serde_json::Error;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        serde_json::from_str(s)
    }
}

#[cfg(feature = "from_metadata")]
impl From<&cargo_metadata::DependencyKind> for PrivateDepKind {
    fn from(kind: &cargo_metadata::DependencyKind) -> Self {
        match kind {
            cargo_metadata::DependencyKind::Normal => PrivateDepKind::Runtime,
            cargo_metadata::DependencyKind::Development => PrivateDepKind::Development,
            cargo_metadata::DependencyKind::Build => PrivateDepKind::Build,
            _ => panic!("Unknown dependency kind")
        }
    }
}

/// Error returned by the conversion from 
/// [`cargo_metadata::Metadata`](https://docs.rs/cargo_metadata/0.11.1/cargo_metadata/struct.Metadata.html)
#[cfg(feature = "from_metadata")]
#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub enum InsufficientMetadata {
    NoDeps,
    VirtualWorkspace,
}

#[cfg(feature = "from_metadata")]
impl Display for InsufficientMetadata {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            InsufficientMetadata::NoDeps => {
                write!(f, "Missing dependency information! Please call 'cargo metadata' without '--no-deps' flag.")
            }
            InsufficientMetadata::VirtualWorkspace => {
                write!(f, "Missing root crate! Please call this from a package directory, not workspace root.")
            }
        }
    }
}

#[cfg(feature = "from_metadata")]
impl Error for InsufficientMetadata {}

#[cfg(feature = "from_metadata")]
impl TryFrom<&cargo_metadata::Metadata> for VersionInfo {
    type Error = InsufficientMetadata;
    fn try_from(metadata: &cargo_metadata::Metadata) -> Result<Self, Self::Error> {
        let toplevel_crate_id = metadata.resolve.as_ref().ok_or(InsufficientMetadata::NoDeps)?
        .root.as_ref().ok_or(InsufficientMetadata::VirtualWorkspace)?.repr.as_str();

        // Walk the dependency tree and resolve dependency kinds for each package.
        // We need this because there may be several different paths to the same package
        // and we need to aggregate dependency types across all of them.
        // Moreover, `cargo metadata` doesn't propagate dependency information:
        // A runtime dependency of a build dependency of your package should be recorded
        // as *build* dependency, but Cargo flags it as a runtime dependency.
        // Hoo boy, here I go hand-rolling BFS again!
        let nodes = &metadata.resolve.as_ref().unwrap().nodes;
        let id_to_node: HashMap<&str, &cargo_metadata::Node> = nodes.iter().map(|n| (n.id.repr.as_str(), n)).collect();
        let mut id_to_dep_kind: HashMap<&str, PrivateDepKind> = HashMap::new();
        id_to_dep_kind.insert(toplevel_crate_id, PrivateDepKind::Runtime);
        let mut current_queue: Vec<&cargo_metadata::Node> = vec![id_to_node[toplevel_crate_id]];
        let mut next_step_queue: Vec<&cargo_metadata::Node> = Vec::new();
        while !current_queue.is_empty() {
            for parent in current_queue.drain(..) {
                let parent_dep_kind = id_to_dep_kind[parent.id.repr.as_str()];
                for child in &parent.deps {
                    let child_id = child.pkg.repr.as_str();
                    let dep_kind = strongest_dep_kind(child.dep_kinds.as_slice());
                    let dep_kind = min(dep_kind, parent_dep_kind);
                    let dep_kind_on_previous_visit = id_to_dep_kind.get(child_id);
                    if dep_kind_on_previous_visit == None || &dep_kind > dep_kind_on_previous_visit.unwrap() {
                        // if we haven't visited this node in dependency graph yet
                        // or if we've visited it with a weaker dependency type,
                        // records its new dependency type and add it to the queue to visit its dependencies
                        id_to_dep_kind.insert(child_id, dep_kind);
                        next_step_queue.push(id_to_node[child_id]);
                    }
                }
            }
            std::mem::swap(&mut next_step_queue, &mut current_queue);
        }

        let metadata_package_dep_kind = |p: &cargo_metadata::Package| {
            let package_id = p.id.repr.as_str();
            id_to_dep_kind.get(package_id)
        };

        // Remove dev-only dependencies from the package list and collect them to Vec
        let mut packages: Vec<&cargo_metadata::Package> = metadata.packages.iter().filter(|p| {
            let dep_kind = metadata_package_dep_kind(p);
            // Dependencies that are present in the workspace but not used by the current root crate
            // will not be in the map we've built by traversing the root crate's dependencies.
            // In this case they will not be in the map at all. We skip them, along with dev-dependencies.
            dep_kind.is_some() && dep_kind.unwrap() != &PrivateDepKind::Development
        }).collect();

        // This function is the simplest place to introduce sorting, since
        // it contains enough data to distinguish between equal-looking packages
        // and provide a stable sorting that might not be possible
        // using the data from VersionInfo struct alone.
        //
        // We use sort_unstable here because there is no point in
        // not reordering equal elements, since they're supplied by
        // in arbitrary order by cargo-metadata anyway
        // and the order even varies between executions.
        packages.sort_unstable_by(|a, b| {
            // This is a workaround for Package not implementing Ord.
            // Deriving it in cargo_metadata might be more reliable?
            let names_order = a.name.cmp(&b.name);
            if names_order != Equal {return names_order;}
            let versions_order = a.name.cmp(&b.name);
            if versions_order != Equal {return versions_order;}
            // IDs are unique so comparing them should be sufficient
            a.id.repr.cmp(&b.id.repr)
        });

        // Build a mapping from package ID to the index of that package in the Vec
        // because serializable representation doesn't store IDs
        let mut id_to_index = HashMap::new();
        for (index, package) in packages.iter().enumerate() {
            id_to_index.insert(package.id.repr.as_str(), index);
        };
        
        // Convert packages from cargo-metadata representation to our representation
        let mut packages: Vec<Package> = packages.into_iter().map(|p| {
            Package {
                name: p.name.to_owned(),
                version: p.version.clone(),
                source: p.source.as_ref().map_or(Source::Local, |s| Source::from(s)),
                kind: (*metadata_package_dep_kind(p).unwrap()).into(),
                dependencies: Vec::new(),
                root: p.id.repr == toplevel_crate_id,
            }
        }).collect();

        // Fill in dependency info from resolved dependency graph
        for node in metadata.resolve.as_ref().unwrap().nodes.iter() {
            let package_id = node.id.repr.as_str();
            if id_to_index.contains_key(package_id) { // dev-dependencies are not included
                let package : &mut Package = &mut packages[id_to_index[package_id]];
                // Dependencies
                for dep in node.dependencies.iter() {
                    // omit package if it is a development-only dependency
                    let dep_id = dep.repr.as_str();
                    if id_to_dep_kind[dep_id] != PrivateDepKind::Development {
                        package.dependencies.push(id_to_index[dep_id]);
                    }
                }
                // .sort_unstable() is fine because they're all integers
                package.dependencies.sort_unstable();
            }
        }
        Ok(VersionInfo {packages})
    }
}

#[cfg(feature = "from_metadata")]
fn strongest_dep_kind(deps: &[cargo_metadata::DepKindInfo]) -> PrivateDepKind {
    deps.iter().map(|d| PrivateDepKind::from(&d.kind)).max()
    .unwrap_or(PrivateDepKind::Runtime) // for compatibility with Rust earlier than 1.41
}

#[cfg(feature = "toml")]
impl TryFrom <&Package> for cargo_lock::Dependency {
    type Error = cargo_lock::Error;
    fn try_from(input: &Package) -> Result<Self, Self::Error> {
        Ok(cargo_lock::Dependency {
            name: cargo_lock::package::Name::from_str(&input.name)?,
            // to_string() is used to work around incompatible semver crate versions
            version: cargo_lock::package::Version::parse(&input.version.to_string())?,
            source: Option::None,
        })
    }
}

#[cfg(feature = "toml")]
impl TryFrom<&VersionInfo> for cargo_lock::Lockfile {
    type Error = cargo_lock::Error;
    fn try_from(input: &VersionInfo) -> Result<Self, Self::Error> {
        let mut root_package: Option<cargo_lock::Package> = None;
        let mut packages: Vec<cargo_lock::Package> = Vec::new();
        for pkg in input.packages.iter() {
            let lock_pkg = cargo_lock::package::Package {
                name: cargo_lock::package::Name::from_str(&pkg.name)?,
                // to_string() is used to work around incompatible semver crate versions
                version: cargo_lock::package::Version::parse(&pkg.version.to_string())?,
                checksum: Option::None,
                dependencies: {
                    let result: Result<Vec<_>, _> = pkg.dependencies.iter().map(|i| {
                        input.packages.get(*i).ok_or(cargo_lock::Error::Parse(
                            format!("There is no dependency with index {} in the input JSON", i))
                        )?.try_into()
                    }).collect();
                    result?
                },
                replace: None,
                source: match &pkg.source {
                    Source::CratesIo => Some(cargo_lock::package::SourceId::from_url("registry+https://github.com/rust-lang/crates.io-index")?),
                    _ => None // we don't store enough info about other sources to reconstruct the URL
                }
            };
            if pkg.root {
                if root_package.is_some() {
                    return Err(cargo_lock::Error::Parse("More than one root package specified in JSON!".to_string()));
                }
                root_package = Some(lock_pkg.clone());
            }
            packages.push(lock_pkg);
        }
        Ok(cargo_lock::Lockfile {
            version: cargo_lock::ResolveVersion::V2,
            packages: packages,
            root: root_package,
            metadata: std::collections::BTreeMap::new(),
            patch: cargo_lock::Patch { unused: Vec::new() },
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::{convert::TryInto, path::PathBuf};

    #[cfg(feature = "from_metadata")]
    fn load_own_metadata() -> cargo_metadata::Metadata {
        let mut cmd = cargo_metadata::MetadataCommand::new();
        let cargo_toml_path = PathBuf::from(std::env::var("CARGO_MANIFEST_DIR").unwrap()).join("Cargo.toml");
        cmd.manifest_path(cargo_toml_path);
        cmd.exec().unwrap()
    }

    #[test]
    #[cfg(feature = "toml")]
    #[cfg(feature = "from_metadata")]
    fn to_toml() {
        let metadata = load_own_metadata();
        let version_info_struct: VersionInfo = (&metadata).try_into().unwrap();
        let _lockfile_struct: cargo_lock::Lockfile = (&version_info_struct).try_into().unwrap();
    }
}
