//! Converts from `cargo_metadata` crate structs to `auditable-serde` structs,
//! which map to our own serialialized representation.

use std::{cmp::min, cmp::Ordering::*, collections::HashMap, error::Error, fmt::Display};

use auditable_serde::{DependencyKind, Package, Source, VersionInfo};

fn source_from_meta(meta_source: &cargo_metadata::Source) -> Source {
    match meta_source.repr.as_str() {
        "registry+https://github.com/rust-lang/crates.io-index" => Source::CratesIo,
        source => Source::from(
            source
                .split('+')
                .next()
                .expect("Encoding of source strings in `cargo metadata` has changed!"),
        ),
    }
}

/// The values are ordered from weakest to strongest so that casting to integer would make sense
#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Copy, Clone)]
enum PrivateDepKind {
    Development,
    Build,
    Runtime,
}

impl From<PrivateDepKind> for DependencyKind {
    fn from(priv_kind: PrivateDepKind) -> Self {
        match priv_kind {
            PrivateDepKind::Development => {
                panic!("Cannot convert development dependency to serializable format")
            }
            PrivateDepKind::Build => DependencyKind::Build,
            PrivateDepKind::Runtime => DependencyKind::Runtime,
        }
    }
}

impl From<&cargo_metadata::DependencyKind> for PrivateDepKind {
    fn from(kind: &cargo_metadata::DependencyKind) -> Self {
        match kind {
            cargo_metadata::DependencyKind::Normal => PrivateDepKind::Runtime,
            cargo_metadata::DependencyKind::Development => PrivateDepKind::Development,
            cargo_metadata::DependencyKind::Build => PrivateDepKind::Build,
            _ => panic!("Unknown dependency kind"),
        }
    }
}

/// Error returned by the conversion from
/// [`cargo_metadata::Metadata`](https://docs.rs/cargo_metadata/0.11.1/cargo_metadata/struct.Metadata.html)
#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub enum InsufficientMetadata {
    NoDeps,
    VirtualWorkspace,
}

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

impl Error for InsufficientMetadata {}

pub fn encode_audit_data(
    metadata: &cargo_metadata::Metadata,
) -> Result<VersionInfo, InsufficientMetadata> {
    let toplevel_crate_id = metadata
        .resolve
        .as_ref()
        .ok_or(InsufficientMetadata::NoDeps)?
        .root
        .as_ref()
        .ok_or(InsufficientMetadata::VirtualWorkspace)?
        .repr
        .as_str();

    // Walk the dependency tree and resolve dependency kinds for each package.
    // We need this because there may be several different paths to the same package
    // and we need to aggregate dependency types across all of them.
    // Moreover, `cargo metadata` doesn't propagate dependency information:
    // A runtime dependency of a build dependency of your package should be recorded
    // as *build* dependency, but Cargo flags it as a runtime dependency.
    // Hoo boy, here I go hand-rolling BFS again!
    let nodes = &metadata.resolve.as_ref().unwrap().nodes;
    let id_to_node: HashMap<&str, &cargo_metadata::Node> =
        nodes.iter().map(|n| (n.id.repr.as_str(), n)).collect();
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
                if dep_kind_on_previous_visit.is_none()
                    || &dep_kind > dep_kind_on_previous_visit.unwrap()
                {
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
    let mut packages: Vec<&cargo_metadata::Package> = metadata
        .packages
        .iter()
        .filter(|p| {
            let dep_kind = metadata_package_dep_kind(p);
            // Dependencies that are present in the workspace but not used by the current root crate
            // will not be in the map we've built by traversing the root crate's dependencies.
            // In this case they will not be in the map at all. We skip them, along with dev-dependencies.
            dep_kind.is_some() && dep_kind.unwrap() != &PrivateDepKind::Development
        })
        .collect();

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
        if names_order != Equal {
            return names_order;
        }
        let versions_order = a.name.cmp(&b.name);
        if versions_order != Equal {
            return versions_order;
        }
        // IDs are unique so comparing them should be sufficient
        a.id.repr.cmp(&b.id.repr)
    });

    // Build a mapping from package ID to the index of that package in the Vec
    // because serializable representation doesn't store IDs
    let mut id_to_index = HashMap::new();
    for (index, package) in packages.iter().enumerate() {
        id_to_index.insert(package.id.repr.as_str(), index);
    }

    // Convert packages from cargo-metadata representation to our representation
    let mut packages: Vec<Package> = packages
        .into_iter()
        .map(|p| Package {
            name: p.name.to_owned(),
            version: p.version.clone(),
            source: p.source.as_ref().map_or(Source::Local, source_from_meta),
            kind: (*metadata_package_dep_kind(p).unwrap()).into(),
            dependencies: Vec::new(),
            root: p.id.repr == toplevel_crate_id,
        })
        .collect();

    // Fill in dependency info from resolved dependency graph
    for node in metadata.resolve.as_ref().unwrap().nodes.iter() {
        let package_id = node.id.repr.as_str();
        if id_to_index.contains_key(package_id) {
            // dev-dependencies are not included
            let package: &mut Package = &mut packages[id_to_index[package_id]];
            // Dependencies
            for dep in node.deps.iter() {
                // Omit the graph edge if this is a development dependency
                // to fix https://github.com/rustsec/rustsec/issues/1043
                // It is possible that something that we depend on normally
                // is also a dev-dependency for something,
                // and dev-dependencies are allowed to have cycles,
                // so we may end up encoding cyclic graph if we don't handle that.
                let dep_id = dep.pkg.repr.as_str();
                if strongest_dep_kind(&dep.dep_kinds) != PrivateDepKind::Development {
                    package.dependencies.push(id_to_index[dep_id]);
                }
            }
            // .sort_unstable() is fine because they're all integers
            package.dependencies.sort_unstable();
        }
    }
    Ok(VersionInfo { packages })
}

fn strongest_dep_kind(deps: &[cargo_metadata::DepKindInfo]) -> PrivateDepKind {
    deps.iter()
        .map(|d| PrivateDepKind::from(&d.kind))
        .max()
        .unwrap_or(PrivateDepKind::Runtime) // for compatibility with Rust earlier than 1.41
}
