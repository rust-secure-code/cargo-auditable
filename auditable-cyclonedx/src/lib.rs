#![forbid(unsafe_code)]

pub use auditable_serde;
use auditable_serde::Package;
pub use cyclonedx_bom;

use cyclonedx_bom::models::{component::Classification, component::Component, metadata::Metadata};
use cyclonedx_bom::prelude::*;

/// Converts the metadata embedded by `cargo auditable` to a minimal CycloneDX document
/// that is heavily optimized to reduce the size
pub fn auditable_to_minimal_cdx(input: &auditable_serde::VersionInfo) -> Bom {
    let mut bom = Bom::default();
    // Clear the serial number which would mess with reproducible builds
    // and also take up valuable space
    bom.serial_number = None;
    // The toplevel component goes into its own field, as per the spec:
    // https://cyclonedx.org/docs/1.5/json/#metadata_component
    let (root_idx, root_pkg) = root_package(input);
    let root_component = pkg_to_component(root_pkg, root_idx);
    let mut metadata = Metadata::default();
    metadata.component = Some(root_component);
    bom.metadata = Some(metadata);
    // Fill in the component list, excluding the toplevel component (already encoded)
    let components: Vec<Component> = input
        .packages
        .iter()
        .enumerate()
        .filter(|(_idx, pkg)| !pkg.root)
        .map(|(idx, pkg)| pkg_to_component(pkg, idx))
        .collect();
    let components = Components(components);
    bom.components = Some(components);
    // TODO: dependency tree
    if cfg!(debug_assertions) {
        assert_eq!(bom.validate(), ValidationResult::Passed);
    }
    bom
}

fn pkg_to_component(pkg: &auditable_serde::Package, idx: usize) -> Component {
    let component_type = if pkg.root {
        Classification::Application
    } else {
        Classification::Library
    };
    // The only requirement for `bom_ref` according to the spec is that it's unique,
    // so we just keep the unique numbering already used in the original
    let bom_ref = idx.to_string();
    Component::new(
        component_type,
        &pkg.name,
        &pkg.version.to_string(),
        Some(bom_ref),
    )
    // TODO: source
    // TODO: dependency kind
    // TODO: purl
}

fn root_package(input: &auditable_serde::VersionInfo) -> (usize, &Package) {
    // we can unwrap here because VersionInfo is already validated during deserialization
    input
        .packages
        .iter()
        .enumerate()
        .find(|(_idx, pkg)| pkg.root)
        .expect("VersionInfo contains no root package!")
}
