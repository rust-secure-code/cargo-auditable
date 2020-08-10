use serde::{Deserialize, Serialize, Serializer, ser::SerializeSeq};
use serde_json;
use std::{convert::{TryFrom, TryInto}, str::FromStr};
use std::{error::Error, cmp::Ordering::*, fmt::Display, collections::HashMap};
#[cfg(feature = "toml")]
use cargo_lock;
#[cfg(feature = "from_metadata")]
use cargo_metadata;

#[derive(Serialize, Deserialize, Debug, PartialEq, Eq, PartialOrd, Ord, Clone)]
//TODO: add #[serde(deny_unknown_fields)] once the format is finalized
//TODO: sort to enable reproducible builds
pub struct RawVersionInfo {
    packages: Vec<Package>,
}

#[derive(Serialize, Deserialize, Debug, PartialEq, Eq, PartialOrd, Ord, Clone)]
pub struct Package {
    name: String,
    version: String, //TODO: parse to a struct
    source: String,
    #[serde(default)]
    #[serde(skip_serializing_if = "is_default")]
    kind: DependencyKinds,
    #[serde(default)]
    #[serde(skip_serializing_if = "is_default")]
    dependencies: Vec<usize>,
}

#[derive(Serialize, Deserialize, Debug, PartialEq, Eq, PartialOrd, Ord, Copy, Clone)]
pub struct DependencyKinds {
    build: bool,
    runtime: bool,
}

impl Default for DependencyKinds {
    fn default() -> Self {
        DependencyKinds {
            build: false,
            runtime: true,
        }
    }
}

impl From<&PrivateDepKinds> for DependencyKinds {
    fn from(priv_kinds: &PrivateDepKinds) -> Self {
        let mut result = DependencyKinds { build: false, runtime: false };
        result.runtime = priv_kinds.runtime;
        result.build = priv_kinds.build;
        if priv_kinds.unknown {
            result.runtime = true; // Fallback for rustc 1.40 and earlier; also a solid default
        }
        result
    }
}

fn is_default<T: Default + PartialEq> (value: &T) -> bool {
    let default_value = T::default();
    value == &default_value
}

// fn sort_and_serialize_vec<S, T>(data: &Vec<T>, serializer: S) -> Result<S::Ok, S::Error> where S: Serializer, T: Serialize + Ord + Clone {
//     let mut seq = serializer.serialize_seq(Some(data.len()))?;
//     let mut data = data.clone();
//     // we do not care about reordering equal elements since they should be indistinguishable
//     data.sort();
//     for e in data {
//         seq.serialize_element(&e)?;
//     }
//     seq.end()
// }

impl FromStr for RawVersionInfo {
    type Err = serde_json::Error;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        serde_json::from_str(s)
    }
}

#[derive(Debug, Copy, Clone, Eq, PartialEq)]
struct PrivateDepKinds {
    dev: bool,
    build: bool,
    runtime: bool,
    unknown: bool,
}

// impl Default for PrivateDepKinds {
//     fn default() -> Self {
//         PrivateDepKinds { dev: false, build: false, runtime: false, unknown: false}
//     }
// }

impl From<&[cargo_metadata::DepKindInfo]> for PrivateDepKinds {
    fn from(deps: &[cargo_metadata::DepKindInfo]) -> Self {
        let mut result = PrivateDepKinds { dev: false, build: false, runtime: false, unknown: false };
        if deps.len() == 0 {
            // for compatibility with Rust earlier than 1.41
            result.unknown = true;
            result
        } else {
            for dep in deps {
                match dep.kind {
                    cargo_metadata::DependencyKind::Normal => result.runtime = true,
                    cargo_metadata::DependencyKind::Build => result.build = true,
                    cargo_metadata::DependencyKind::Development => result.dev = true,
                    _ => result.unknown = true,
                }
            }
            result
        }
    }
}

#[cfg(feature = "from_metadata")]
impl From<&cargo_metadata::Metadata> for RawVersionInfo {
    fn from(metadata: &cargo_metadata::Metadata) -> Self {
        // Build a map of unique ID of each dependency to the dependency data
        let mut id_to_package: HashMap<&str, &cargo_metadata::Package> = HashMap::new();
        for p in metadata.packages.iter() {
            id_to_package.insert(&p.id.repr, p);
        }

        // Walk the dependency tree and resolve dependency kinds for each package
        let mut id_to_dep_kinds: HashMap<&str, PrivateDepKinds> = HashMap::new();
        // TODO: check that Resolve field is populated instead of unwrap(); this is the case for `--no-deps`
        for node in metadata.resolve.as_ref().unwrap().nodes.iter() {
            for dep in node.deps.iter() {
                id_to_dep_kinds.insert(&dep.pkg.repr, PrivateDepKinds::from(dep.dep_kinds.as_slice()));
            }
        }

        let metadata_package_dep_kinds = |p: &cargo_metadata::Package| {
            let package_id = p.id.repr.as_str();
            let package_dep_kinds = id_to_dep_kinds.get(package_id).unwrap_or(
                // Nothing depends on the toplevel package, so there's no build dep kind for it.
                // We default to all known dependency kinds being enabled for it.
                &PrivateDepKinds { dev: true, build: true, runtime: true, unknown: false }
            );
            package_dep_kinds
        };

        // Remove dev-only dependencies from the package list and collect them to Vec
        let mut packages: Vec<&cargo_metadata::Package> = id_to_package.values().filter(|p| {
            metadata_package_dep_kinds(p) != &PrivateDepKinds {
                dev: true, build: false, runtime: false, unknown: false
            }
        }).map(|x| *x).collect();

        // This function is the simplest place to introduce sorting, since
        // it contains enough data to distinguish between equal-looking packages
        // and provide a stable sorting that might not be possible
        // using the data from RawVersionInfo struct alone.

        // We use sort_unstable here because there is no point in
        // not reordering equal elements, since they're supplied by
        // in arbitratry order by cargo-metadata anyway
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
        let mut id_to_index = HashMap::new();
        for (index, package) in packages.iter().enumerate() {
            // This can be further optimized via mem::take() to avoid cloning, but eh
            id_to_index.insert(package.id.repr.clone(), index);
        };
        let packages: Vec<Package> = packages.into_iter().map(|p| {
            Package {
                name: p.name.to_owned(),
                version: p.version.to_string(), // TODO: use a struct
                source: source_to_source_string(&p.source),
                kind: metadata_package_dep_kinds(&p).into(),
                dependencies: Vec::new()
            }
        }).collect();
        // TODO: encode dependencies
        RawVersionInfo {packages}
    }
}

#[cfg(feature = "from_metadata")]
fn source_to_source_string(s: &Option<cargo_metadata::Source>) -> String {
    if let Some(source) = s {
        source.repr.as_str().split('+').next().unwrap_or("").to_owned()
    } else {
        "local".to_owned()
    }
}

// #[cfg(feature = "from_metadata")]
// fn strongest_dependency_kind(deps: &[cargo_metadata::DepKindInfo]) -> DependencyKind {
//     if deps.len() == 0 {
//         // for compatibility with Rust earlier than 1.41
//         DependencyKind::Runtime
//     } else {
//         let mut strongest_kind = DependencyKind::Development;
//         for dep in deps {
//             let kind = DependencyKind::try_from(&dep.kind).unwrap_or(DependencyKind::Runtime);
//             if kind as u8 > strongest_kind as u8 {
//                 strongest_kind = kind;
//             }
//         }
//         strongest_kind
//     }
// }

// #[cfg(feature = "toml")]
// impl RawVersionInfo {
//     pub fn from_toml(toml: &str) -> Result<Self, cargo_lock::error::Error> {
//         Ok(Self::from(&cargo_lock::lockfile::Lockfile::from_str(toml)?))
//     }
// }

// #[cfg(feature = "toml")]
// impl From<&cargo_lock::dependency::Dependency> for Dependency {
//     fn from(source: &cargo_lock::dependency::Dependency) -> Self {
//         Self {
//             name: source.name.as_str().to_owned(),
//             version: source.version.to_string(),
//         }
//     }
// }

// #[cfg(feature = "toml")]
// impl From<&cargo_lock::package::Package> for Package {
//     fn from(source: &cargo_lock::package::Package) -> Self {
//         Self {
//             name: source.name.as_str().to_owned(),
//             version: source.version.to_string(),
//             checksum: match &source.checksum {
//                 Some(value) => Some(value.to_string()),
//                 None => None,
//             },
//             dependencies: source.dependencies.iter().map(|d| d.into()).collect(),
//         }
//     }
// }

// #[cfg(feature = "toml")]
// impl From<&cargo_lock::lockfile::Lockfile> for RawVersionInfo {
//     fn from(source: &cargo_lock::lockfile::Lockfile) -> Self {
//         Self {
//             packages: source.packages.iter().map(|p| p.into()).collect(),
//         }
//     }
// }

// #[cfg(feature = "toml")]
// impl TryInto<cargo_lock::dependency::Dependency> for &Dependency {
//     type Error = cargo_lock::error::Error;
//     fn try_into(self) -> Result<cargo_lock::dependency::Dependency, Self::Error> {
//         Ok(cargo_lock::dependency::Dependency {
//             name: cargo_lock::package::name::Name::from_str(&self.name)?,
//             version: cargo_lock::package::Version::parse(&self.version)?,
//             source: None,
//         })
//     }
// }

// #[cfg(feature = "toml")]
// impl TryInto<cargo_lock::package::Package> for &Package {
//     type Error = cargo_lock::error::Error;
//     fn try_into(self) -> Result<cargo_lock::package::Package, Self::Error> {
//         Ok(cargo_lock::package::Package {
//             name: cargo_lock::package::name::Name::from_str(&self.name)?,
//             version: cargo_lock::package::Version::parse(&self.version)?,
//             checksum: match &self.checksum {
//                 Some(value) => Some(cargo_lock::package::checksum::Checksum::from_str(&value)?),
//                 None => None,
//             },
//             dependencies: {
//                 let result: Result<Vec<_>, _> =
//                     self.dependencies.iter().map(TryInto::try_into).collect();
//                 result?
//             },
//             replace: None,
//             source: None,
//         })
//     }
// }

// #[cfg(feature = "toml")]
// impl TryInto<cargo_lock::lockfile::Lockfile> for &RawVersionInfo {
//     type Error = cargo_lock::error::Error;
//     fn try_into(self) -> Result<cargo_lock::lockfile::Lockfile, Self::Error> {
//         Ok(cargo_lock::lockfile::Lockfile {
//             version: cargo_lock::lockfile::version::ResolveVersion::V2,
//             packages: {
//                 let result: Result<Vec<_>, _> =
//                     self.packages.iter().map(TryInto::try_into).collect();
//                 result?
//             },
//             root: None,
//             metadata: std::collections::BTreeMap::new(),
//             patch: cargo_lock::patch::Patch { unused: Vec::new() },
//         })
//     }
// }

// #[cfg(test)]
// mod tests {
//     use super::RawVersionInfo;
//     use std::{convert::TryInto, path::PathBuf};

//     #[cfg(feature = "toml")]
//     fn load_our_own_cargo_lock() -> String {
//         let crate_root_dir = PathBuf::from(std::env::var("CARGO_MANIFEST_DIR").unwrap());
//         let cargo_lock_location = crate_root_dir.join("Cargo.lock");
//         let cargo_lock_contents = std::fs::read_to_string(cargo_lock_location).unwrap();
//         cargo_lock_contents
//     }

//     #[test]
//     #[cfg(feature = "toml")]
//     fn lockfile_struct_conversion_roundtrip() {
//         let cargo_lock_contents = load_our_own_cargo_lock();
//         let version_info_struct = RawVersionInfo::from_toml(&cargo_lock_contents)
//             .expect("Failed to convert from TOML to JSON");
//         let lockfile_struct: cargo_lock::lockfile::Lockfile =
//             (&version_info_struct).try_into().unwrap();
//         let roundtripped_version_info_struct: RawVersionInfo = (&lockfile_struct).into();
//         assert_eq!(version_info_struct, roundtripped_version_info_struct);
//     }
// }
