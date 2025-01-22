#![doc = include_str!("../README.md")]

mod error;
mod fields;
mod raw_fields;
mod resolver;

pub use error::Error;
use raw_fields::RawTomlFields;
pub use resolver::Resolver;

pub fn from_toml(workspace_root_cargo_toml: &str) -> Result<Resolver, crate::Error> {
    let parsed: RawTomlFields = toml::from_str(workspace_root_cargo_toml)?;
    Ok(parsed.resolve().resolver()?)
}
