use std::fmt;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum UnrecognizedValue {
    UnknownResolver(String),
    UnknownEdition(String),
}

impl std::fmt::Display for UnrecognizedValue {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            UnrecognizedValue::UnknownResolver(resolver) => {
                write!(f, "Unrecognized resolver version: {}", resolver)
            }
            UnrecognizedValue::UnknownEdition(edition) => {
                write!(f, "Unrecognized Rust edition: {}", edition)
            }
        }
    }
}

impl std::error::Error for UnrecognizedValue {}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Error {
    UnrecognizedValue(UnrecognizedValue),
    TomlParseError(toml::de::Error), // Adjust for the specific Serde library in use (e.g., `serde_json` or `serde_yaml`)
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Error::UnrecognizedValue(msg) => write!(f, "{}", msg),
            Error::TomlParseError(err) => write!(f, "Failed to parse Cargo.toml: {}", err),
        }
    }
}

impl std::error::Error for Error {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Error::UnrecognizedValue(err) => Some(err),
            Error::TomlParseError(err) => Some(err),
        }
    }
}

impl From<UnrecognizedValue> for Error {
    fn from(value: UnrecognizedValue) -> Self {
        Error::UnrecognizedValue(value)
    }
}

impl From<toml::de::Error> for Error {
    fn from(err: toml::de::Error) -> Self {
        Error::TomlParseError(err)
    }
}
