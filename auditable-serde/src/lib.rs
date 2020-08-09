use serde::{Deserialize, Serialize, Serializer, ser::SerializeSeq};
use serde_json;
use std::{convert::{TryFrom, TryInto}, str::FromStr};
use std::{error::Error, cmp::Ordering::*, fmt::Display};
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
    kind: DependencyKind,
    #[serde(default)]
    #[serde(skip_serializing_if = "is_default")]
    dependencies: Vec<usize>,
}

// The fields are ordered from weakest to strongers so that casting to integer would make sense
#[derive(Serialize, Deserialize, Debug, PartialEq, Eq, PartialOrd, Ord, Copy, Clone)]
pub enum DependencyKind {
    Development, //TODO: do we want to include these?
    Build,
    Runtime,
}

impl Default for DependencyKind {
    fn default() -> Self {
        DependencyKind::Runtime
    }
}

#[derive(Debug)]
pub struct UnknownDependencyKind;
impl Error for UnknownDependencyKind {}
impl Display for UnknownDependencyKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Unknown dependency kind")
    }
}

#[cfg(feature = "from_metadata")]
impl TryFrom<&cargo_metadata::DependencyKind> for DependencyKind {
    type Error = UnknownDependencyKind;
    fn try_from(value: &cargo_metadata::DependencyKind) -> Result<Self, Self::Error> {
        match value {
            cargo_metadata::DependencyKind::Normal => Ok(DependencyKind::Runtime),
            cargo_metadata::DependencyKind::Development => Ok(DependencyKind::Development),
            cargo_metadata::DependencyKind::Build => Ok(DependencyKind::Build),
            // we assume build deps by default, useful for Rust 1.40 and earlier
            cargo_metadata::DependencyKind::Unknown => Err(UnknownDependencyKind),
        }
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

#[cfg(feature = "from_metadata")]
impl From<cargo_metadata::Metadata> for RawVersionInfo {
    fn from(mut metadata: cargo_metadata::Metadata) -> Self {
        // This is the simplest place to introduce sorting, since
        // it contains enough data to distinguish between equal-looking packages
        // and provide a stable sorting that might not be possible
        // with the data from RawVersionInfo struct alone.

        // We use sort_unstable here because there is no point in
        // not reordering equal elements, since they're supplied by
        // in arbitratry order by cargo-metadata anyway
        // and the order even varies between executions.
        metadata.packages.sort_unstable_by(|a, b| {
            // This is a workaround for Package not implementing Ord.
            // Deriving it in cargo_metadata would be more reliable.
            let names_order = a.name.cmp(&b.name);
            if names_order != Equal {return names_order;}
            let versions_order = a.name.cmp(&b.name);
            if versions_order != Equal {return versions_order;}
            // IDs are unique so comparing them should be sufficient
            a.id.repr.cmp(&b.id.repr)
        });
        let mut id_to_index = std::collections::HashMap::new();
        for (index, package) in metadata.packages.iter().enumerate() {
            // This can be further optimized via mem::take() to avoid cloning, but eh
            id_to_index.insert(package.id.repr.clone(), index);
        };
        let packages: Vec<Package> = metadata.packages.into_iter().map(|p| {
            Package {
                name: p.name,
                version: p.version.to_string(), // TODO: use a struct
                source: metadata_source_to_source_string(&p.source),
                kind: DependencyKind::default(), // will be overwritten later
                dependencies: Vec::new()
            }
        }).collect();
        // TODO: encode dependencies
        RawVersionInfo {packages}
    }
}

#[cfg(feature = "from_metadata")]
fn metadata_source_to_source_string(s: &Option<cargo_metadata::Source>) -> String {
    if let Some(source) = s {
        source.repr.as_str().split('+').next().unwrap_or("").to_owned()
    } else {
        "local".to_owned()
    }
}

#[cfg(feature = "from_metadata")]
fn strongest_dependency_kind(deps: &[cargo_metadata::DepKindInfo]) -> DependencyKind {
    if deps.len() == 0 {
        // for compatibility with Rust earlier than 1.41
        DependencyKind::Runtime
    } else {
        let mut strongest_kind = DependencyKind::Development;
        for dep in deps {
            let kind = DependencyKind::try_from(&dep.kind).unwrap_or(DependencyKind::Runtime);
            if kind as u8 > strongest_kind as u8 {
                strongest_kind = kind;
            }
        }
        strongest_kind
    }
}

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
