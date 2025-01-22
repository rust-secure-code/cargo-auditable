use crate::{error::UnrecognizedValue, Resolver};

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct TomlFields {
    pub resolver: Option<String>,
    pub edition: Option<String>,
}

impl TomlFields {
    pub fn resolver(self) -> Result<Resolver, UnrecognizedValue> {
        if let Some(ver) = self.resolver {
            match ver.as_str() {
                "1" => Ok(Resolver::V1),
                "2" => Ok(Resolver::V2),
                "3" => Ok(Resolver::V3),
                _ => Err(UnrecognizedValue::UnknownResolver(ver)),
            }
        } else if let Some(ed) = self.edition {
            match ed.as_str() {
                "2015" => Ok(Resolver::V1),
                "2018" | "2021" => Ok(Resolver::V2),
                "2024" => Ok(Resolver::V3),
                _ => Err(UnrecognizedValue::UnknownEdition(ed)),
            }
        } else {
            Ok(Resolver::V1)
        }
    }
}
