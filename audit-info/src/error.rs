#[derive(Debug)]
pub enum Error {
    NoAuditData,
    InputLimitExceeded,
    OutputLimitExceeded,
    Io(std::io::Error),
    BinaryParsing(auditable_extract::Error),
    Decompression(miniz_oxide::inflate::DecompressError),
    Json(serde_json::Error),
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Error::NoAuditData => write!(f, "No audit data found in the binary"),
            Error::InputLimitExceeded => write!(f, "The input file is too large. Increase the input size limit to scan it"),
            Error::OutputLimitExceeded => write!(f, "Audit data size is over the specified limit. Increase the output size limit to scan it."),
            Error::Io(e) => write!(f, "Failed to read the binary: {}", e),
            Error::BinaryParsing(e) => write!(f, "Failed to parse the binary: {}", e),
            Error::Decompression(e) => write!(f, "Failed to decompress audit data: {}", e),
            Error::Json(e) => write!(f, "Failed to deserialize audit data from JSON: {}", e),
        }
    }
}

impl std::error::Error for Error {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Error::NoAuditData => None,
            Error::InputLimitExceeded => None,
            Error::OutputLimitExceeded => None,
            Error::Io(e) => Some(e),
            Error::BinaryParsing(e) => Some(e),
            Error::Decompression(e) => Some(e),
            Error::Json(e) => Some(e),
        }
    }
}

impl From<std::io::Error> for Error {
    fn from(e: std::io::Error) -> Self {
        Self::Io(e)
    }
}

impl From<auditable_extract::Error> for Error {
    fn from(e: auditable_extract::Error) -> Self {
        match e {
            auditable_extract::Error::NoAuditData => Error::NoAuditData,
            other_err => Self::BinaryParsing(other_err),
        }
    }
}

impl From<miniz_oxide::inflate::DecompressError> for Error {
    fn from(e: miniz_oxide::inflate::DecompressError) -> Self {
        match e.status {
            miniz_oxide::inflate::TINFLStatus::HasMoreOutput => Error::OutputLimitExceeded,
            _ => Error::Decompression(e),
        }
    }
}

impl From<serde_json::Error> for Error {
    fn from(e: serde_json::Error) -> Self {
        Self::Json(e)
    }
}
