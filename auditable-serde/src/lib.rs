use cargo_lock;
use std::{str::FromStr, convert::TryInto, error::Error};
use serde::{Deserialize, Serialize};
use serde_json;

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RawVersionInfo {
    packages: Vec<Package>
}

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Package {
    #[serde(rename = "n")]
    name: String,
    #[serde(rename = "v")]
    version: String,
    #[serde(default)]
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(rename = "c")]
    checksum: Option<String>,
    #[serde(default)]
    #[serde(skip_serializing_if = "Vec::is_empty")]
    #[serde(rename = "d")]
    dependencies: Vec<Dependency>
}

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Dependency {
    #[serde(rename = "n")]
    name: String,
    #[serde(rename = "v")]
    version: String
}

impl RawVersionInfo {
    pub fn from_toml(toml: &str) -> Result<Self, cargo_lock::error::Error> {
        Ok(Self::from(&cargo_lock::lockfile::Lockfile::from_str(toml)?))
    }
}

impl FromStr for RawVersionInfo {
    type Err = serde_json::Error;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        serde_json::from_str(s)
    }
}

impl From<&cargo_lock::dependency::Dependency> for Dependency {
    fn from(source: &cargo_lock::dependency::Dependency) -> Self {
        Self {
            name: source.name.as_str().to_owned(),
            version: source.version.to_string()
        }
    }
}

impl From<&cargo_lock::package::Package> for Package {
    fn from(source: &cargo_lock::package::Package) -> Self {
        Self {
            name: source.name.as_str().to_owned(),
            version: source.version.to_string(),
            checksum: match &source.checksum {
                Some(value) => Some(value.to_string()),
                None => None
            },
            dependencies: source.dependencies.iter().map(|d| d.into()).collect()
        }
    }
}

impl From<&cargo_lock::lockfile::Lockfile> for RawVersionInfo {
    fn from(source: &cargo_lock::lockfile::Lockfile) -> Self {
        Self {
            packages: source.packages.iter().map(|p| p.into()).collect()
        }
    }
    
}

impl TryInto<cargo_lock::dependency::Dependency> for Dependency {
    type Error = cargo_lock::error::Error;
    fn try_into(self) -> Result<cargo_lock::dependency::Dependency, Self::Error> {
        Ok(cargo_lock::dependency::Dependency {
            name: cargo_lock::package::name::Name::from_str(&self.name)?,
            version: cargo_lock::package::Version::parse(&self.version)?,
            source: None
        })
    }
}

impl TryInto<cargo_lock::package::Package> for Package {
    type Error = cargo_lock::error::Error;
    fn try_into(self) -> Result<cargo_lock::package::Package, Self::Error> {
        Ok(cargo_lock::package::Package {
            name: cargo_lock::package::name::Name::from_str(&self.name)?,
            version: cargo_lock::package::Version::parse(&self.version)?,
            checksum: match self.checksum {
                Some(value ) => Some(cargo_lock::package::checksum::Checksum::from_str(&value)?),
                None => None
            },
            dependencies: vec_try_into::<Dependency, cargo_lock::dependency::Dependency, Self::Error>(self.dependencies)?,
            replace: None,
            source: None,
        })
    }
}

impl TryInto<cargo_lock::lockfile::Lockfile> for RawVersionInfo {
    type Error = cargo_lock::error::Error;
    fn try_into(self) -> Result<cargo_lock::lockfile::Lockfile, Self::Error> {
        Ok(cargo_lock::lockfile::Lockfile {
            version: cargo_lock::lockfile::version::ResolveVersion::V2,
            // The rant from conversion code from Package also applies here
            packages: vec_try_into::<Package, cargo_lock::package::Package, Self::Error>(self.packages)?,
            root: None,
            metadata: std::collections::BTreeMap::new(),
            patch: cargo_lock::patch::Patch {unused: Vec::new()},
        })
    }
    
}

// Surprisingly, TryInto on Vec requires the element to implement TryFrom:
// https://doc.rust-lang.org/std/vec/struct.Vec.html#impl-TryInto%3CU%3E
// But we cannot implement TryFrom due to orphan rules.
// A slightly saner version would be this:
// self.dependencies.into_iter().map(|x| x.try_into()?).collect()
// but it also doesn't compile. Hence this mess:
fn vec_try_into<A: TryInto<B>, B, E: Error + std::convert::From<<A as std::convert::TryInto<B>>::Error>>(source: Vec<A>) -> Result<Vec<B>, E>  {
        let mut result = Vec::with_capacity(source.len());
        for dep in source {
            result.push(dep.try_into()?);
        }
        Ok(result)
}

#[cfg(test)]
mod tests {
    #[test]
    fn it_works() {
        assert_eq!(2 + 2, 4);
    }
}
