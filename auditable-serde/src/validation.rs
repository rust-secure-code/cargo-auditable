use std::{convert::TryFrom, fmt::Display};
use serde::{Deserialize, Serialize};
use crate::{Package, VersionInfo};

#[derive(Serialize, Deserialize, Debug, PartialEq, Eq, PartialOrd, Ord, Clone)]
pub struct RawVersionInfo {
    pub packages: Vec<Package>,
}

pub enum ValidationError {
    MultipleRoots,
    CyclicDependency,
}

impl Display for ValidationError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ValidationError::MultipleRoots => write!(f, "Multiple root packages specified in the input JSON"),
            ValidationError::CyclicDependency => write!(f, "The input JSON specifies a cyclic dependency graph"),
        }
    }
}

impl TryFrom<RawVersionInfo> for VersionInfo {
    type Error = ValidationError;

    fn try_from(v: RawVersionInfo) -> Result<Self, Self::Error> {
        if has_multiple_root_packages(&v) {
            Err(ValidationError::MultipleRoots)
        } else if has_cylic_dependencies(&v) {
            Err(ValidationError::CyclicDependency)
        } else {
            Ok(VersionInfo { packages: v.packages })
        }
    }
}

fn has_multiple_root_packages(v: &RawVersionInfo) -> bool {
    let mut seen_a_root = false;
    for package in &v.packages {
        if package.root {
            if seen_a_root {
                return true
            } else {
                seen_a_root = true;
            }
        }
    }
    false
}

fn has_cylic_dependencies(v: &RawVersionInfo) -> bool {
    // I've reviewed the `topological_sort` crate and it appears to be high-quality,
    // so I'm not concerned about having it exposed to untrusted input.
    // It's better than my hand-rolled version would have been.

    // populate the topological sorting map
    let mut ts = topological_sort::TopologicalSort::<usize>::new();
    for (index, package) in v.packages.iter().enumerate() {
        for dep in &package.dependencies {
            ts.add_dependency(*dep, index);
        }
    }
    // drain all elements that are not part of a cycle
    while ts.pop().is_some() {};
    // if the set isn't empty, the graph has cycles
    ! ts.is_empty()
}