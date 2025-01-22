use crate::fields::TomlFields;
use serde::Deserialize;

#[derive(Deserialize, Debug, Clone, Eq, PartialEq)]
struct Package {
    resolver: Option<String>,
    edition: Option<Edition>,
}

#[derive(Deserialize, Debug, Clone, Eq, PartialEq)]
#[serde(untagged)]
enum Edition {
    Edition(String),
    InheritWorkspace { workspace: bool },
}

#[derive(Deserialize, Debug, Clone, Eq, PartialEq)]
struct Workspace {
    package: Option<Package>,
    resolver: Option<String>,
}

/// Raw deserialized TOML fields, before resolving workspace inheritance
#[derive(Deserialize, Debug, Clone, Eq, PartialEq)]
pub(crate) struct RawTomlFields {
    package: Option<Package>,
    workspace: Option<Workspace>,
}

impl RawTomlFields {
    pub fn resolve(self) -> TomlFields {
        let resolver: Option<String> = {
            // Cargo rejects files that specify both package.resolver and workspace.resolver
            if let Some(version) = self.workspace.clone().and_then(|w| w.resolver) {
                Some(version)
            } else {
                self.package.clone().and_then(|pkg| pkg.resolver)
            }
        };

        let edition: Option<String> = self.package.and_then(|pkg| {
            pkg.edition.and_then(|ed| {
                match ed {
                    Edition::Edition(value) => Some(value),
                    Edition::InheritWorkspace { workspace } => match workspace {
                        true => self
                            .workspace
                            .and_then(|w| w.package)
                            .and_then(|pkg| pkg.edition)
                            .and_then(|ed| match ed {
                                Edition::Edition(value) => Some(value),
                                // `edition.workspace = true` cannot appear in workspace definition (under [workspace])
                                Edition::InheritWorkspace { .. } => None,
                            }),
                        false => None,
                    },
                }
            })
        });

        TomlFields { resolver, edition }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn all_the_fields() {
        let toml = "
[package]
name = \"sample-package\"
version = \"0.1.0\"
edition.workspace = true
resolver = \"1\"

[dependencies]

[workspace]
package.edition = \"2021\"
";

        let expected = RawTomlFields {
            package: Some(Package {
                resolver: Some("1".to_owned()),
                edition: Some(Edition::InheritWorkspace { workspace: true }),
            }),
            workspace: Some(Workspace {
                package: Some(Package {
                    resolver: None,
                    edition: Some(Edition::Edition("2021".to_owned())),
                }),
                resolver: None,
            }),
        };

        let parsed: RawTomlFields = toml::from_str(&toml).unwrap();
        assert_eq!(parsed, expected);

        let resolved_expected = TomlFields {
            resolver: Some("1".to_owned()),
            edition: Some("2021".to_owned()),
        };
        let resolved = parsed.resolve();
        assert_eq!(resolved, resolved_expected);
    }
}

#[test]
fn all_the_other_fields() {
    let toml = "
[package]
name = \"sample-package\"
version = \"0.1.0\"
edition = \"2015\"

[dependencies]

[workspace]
resolver = \"2\"
";

    let expected = RawTomlFields {
        package: Some(Package {
            resolver: None,
            edition: Some(Edition::Edition("2015".to_owned())),
        }),
        workspace: Some(Workspace {
            package: None,
            resolver: Some("2".to_owned()),
        }),
    };

    let parsed: RawTomlFields = toml::from_str(&toml).unwrap();
    assert_eq!(parsed, expected);

    let resolved_expected = TomlFields {
        resolver: Some("2".to_owned()),
        edition: Some("2015".to_owned()),
    };
    let resolved = parsed.resolve();
    assert_eq!(resolved, resolved_expected);
}

#[test]
fn regular_package() {
    let toml = "
[package]
name = \"sample-package\"
version = \"0.1.0\"
edition = \"2021\"
";

    let expected = RawTomlFields {
        package: Some(Package {
            resolver: None,
            edition: Some(Edition::Edition("2021".to_owned())),
        }),
        workspace: None,
    };

    let parsed: RawTomlFields = toml::from_str(&toml).unwrap();
    assert_eq!(parsed, expected);
}

#[test]
fn barebones_package() {
    let toml = "
[package]
name = \"sample-package\"
version = \"0.1.0\"
";

    let expected = RawTomlFields {
        package: Some(Package {
            resolver: None,
            edition: None,
        }),
        workspace: None,
    };

    let parsed: RawTomlFields = toml::from_str(&toml).unwrap();
    assert_eq!(parsed, expected);

    let resolved_expected = TomlFields {
        resolver: None,
        edition: None,
    };
    let resolved = parsed.resolve();
    assert_eq!(resolved, resolved_expected);
}

#[test]
fn barebones_workspace() {
    let toml = "
[workspace]
members = [\"some-package\"]
";

    let expected = RawTomlFields {
        package: None,
        workspace: Some(Workspace {
            package: None,
            resolver: None,
        }),
    };

    let parsed: RawTomlFields = toml::from_str(&toml).unwrap();
    assert_eq!(parsed, expected);

    let resolved_expected = TomlFields {
        resolver: None,
        edition: None,
    };
    let resolved = parsed.resolve();
    assert_eq!(resolved, resolved_expected);
}
