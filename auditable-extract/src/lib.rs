#![forbid(unsafe_code)]

use binfarce;
use binfarce::Format;

pub fn raw_auditable_data<'a>(data: &'a [u8]) -> Result<Option<&'a [u8]>, Error> {
    match binfarce::detect_format(data) {
        Format::Elf32{byte_order} => {
            let section = binfarce::elf32::parse(data, byte_order)?
                .section_with_name(".rust-deps-v0")?;
            match section {
                Some(section) => Ok(Some(data.get(section.range()?).ok_or(Error::UnexpectedEof)?)),
                None => Ok(None),
            }
        },
        Format::Elf64{byte_order} => {
            let section = binfarce::elf64::parse(data, byte_order)?
                .section_with_name(".rust-deps-v0")?;
                match section {
                    Some(section) => Ok(Some(data.get(section.range()?).ok_or(Error::UnexpectedEof)?)),
                    None => Ok(None),
                }
        },
        Format::Macho => {
            let parsed = binfarce::macho::parse(data)?;
            let section = parsed.section_with_name("__TEXT", "rust-deps-v0")?;
            match section {
                Some(section) => Ok(Some(data.get(section.range()?).ok_or(Error::UnexpectedEof)?)),
                None => Ok(None),
            }
        },
        Format::PE => {
            let parsed = binfarce::pe::parse(data)?;
            let section = parsed.section_with_name("rdep-v0")?;
            match section {
                Some(section) => Ok(Some(data.get(section.range()?).ok_or(Error::UnexpectedEof)?)),
                None => Ok(None),
            }
        }
        _ => Err(Error::NotAnExecutable)
    }
}

#[derive(Debug, Copy, Clone)]
pub enum Error {
    NoAuditData,
    NotAnExecutable,
    UnexpectedEof,
    MalformedFile,
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let message = match self {
            Error::NoAuditData => "No audit data found in the executable",
            Error::NotAnExecutable => "Not an executable file",
            Error::UnexpectedEof => "Unexpected end of file",
            Error::MalformedFile => "Malformed executable file",
        };
        write!(f, "{}", message)
    }
}

impl From<binfarce::ParseError> for Error {
    fn from(e: binfarce::ParseError) -> Self {
        match e {
            binfarce::ParseError::MalformedInput => Error::MalformedFile,
            binfarce::ParseError::UnexpectedEof => Error::UnexpectedEof,
        }
    }   
}