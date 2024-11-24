#![forbid(unsafe_code)]

use std::str::FromStr;

pub use auditable_serde;
use auditable_serde::{Package, Source};
pub use cyclonedx_bom;

use cyclonedx_bom::models::property::{Properties, Property};
use cyclonedx_bom::prelude::*;
use cyclonedx_bom::{
    external_models::uri::Purl,
    models::{
        component::Classification,
        component::Component,
        dependency::{Dependencies, Dependency},
        metadata::Metadata,
    },
};

/// Converts the metadata embedded by `cargo auditable` to a minimal CycloneDX document
/// that is heavily optimized to reduce the size
pub fn auditable_to_minimal_cdx(input: &auditable_serde::VersionInfo) -> Bom {
    let mut bom = Bom {
        serial_number: None, // the serial number would mess with reproducible builds
        ..Default::default()
    };

    // The toplevel component goes into its own field, as per the spec:
    // https://cyclonedx.org/docs/1.5/json/#metadata_component
    let (root_idx, root_pkg) = root_package(input);
    let root_component = pkg_to_component(root_pkg, root_idx);
    let metadata = Metadata {
        component: Some(root_component),
        ..Default::default()
    };
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

    // Populate the dependency tree. Actually really easy, it's the same format as ours!
    let dependencies: Vec<Dependency> = input
        .packages
        .iter()
        .enumerate()
        .map(|(idx, pkg)| Dependency {
            dependency_ref: idx.to_string(),
            dependencies: pkg.dependencies.iter().map(|idx| idx.to_string()).collect(),
        })
        .collect();
    let dependencies = Dependencies(dependencies);
    bom.dependencies = Some(dependencies);

    // Validate the generated SBOM if running in debug mode (or release with debug assertions)
    if cfg!(debug_assertions) {
        assert!(bom.validate().passed());
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
    let mut result = Component::new(
        component_type,
        &pkg.name,
        &pkg.version.to_string(),
        Some(bom_ref),
    );
    // PURL encodes the package origin (registry, git, local) - sort of, anyway
    let purl = purl(pkg);
    let purl = Purl::from_str(&purl).unwrap();
    result.purl = Some(purl);
    // Record the dependency kind
    match pkg.kind {
        // `Runtime` is the default and does not need to be recorded.
        auditable_serde::DependencyKind::Runtime => (),
        auditable_serde::DependencyKind::Build => {
            let p = Property::new("cdx:rustc:dependency_kind".to_owned(), "build");
            result.properties = Some(Properties(vec![p]));
        }
    }
    result
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

fn purl(pkg: &auditable_serde::Package) -> String {
    // The purl crate exposed by `cyclonedx-bom` doesn't support the qualifiers we need,
    // so we just build the PURL as a string.
    // Yeah, we could use *yet another* dependency to build the PURL,
    // but we use it such trivial ways that it isn't worth the trouble.
    // Specifically, the crate names that crates.io accepts don't need percent-encoding
    // and the fixed values we put in arguments don't either
    // (but percent-encoding is underspecified and not interoperable anyway,
    // see e.g. https://github.com/package-url/purl-spec/pull/261)
    let mut purl = format!("pkg:cargo/{}@{}", pkg.name, pkg.version);
    purl.push_str(match &pkg.source {
        Source::CratesIo => "", // this is the default, nothing to qualify
        Source::Git => "&vcs_url=redacted",
        Source::Local => "&download_url=redacted",
        Source::Registry => "&repository_url=redacted",
        Source::Other(_) => "&download_url=redacted",
        unknown => panic!("Unknown source: {:?}", unknown),
    });
    purl
}
