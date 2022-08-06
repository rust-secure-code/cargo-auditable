#![forbid(unsafe_code)]

use auditable_extract::raw_auditable_data;
use miniz_oxide::inflate::decompress_to_vec_zlib_with_limit;
use std::env::args_os;
use std::ffi::OsString;
use std::io::{BufReader, Read, Write};
use std::{error::Error, fs::File};

const USAGE: &'static str = "\
Usage: rust-audit-info FILE [INPUT_SIZE_LIMIT] [OUTPUT_SIZE_LIMIT]

The limits are specified in bytes. The default values are:

    INPUT_SIZE_LIMIT: 1073741824 (1 GiB)
    OUTPUT_SIZE_LIMIT: 67108864 (64 MiB)
";

fn main() {
    if let Err(e) = actual_main() {
        eprintln!("{}", e);
        std::process::exit(1);
    }
}

fn actual_main() -> Result<(), Box<dyn Error>> {
    let (input, limits) = parse_args()?;

    let compressed_audit_data: Vec<u8> = {
        let f = File::open(input)?;
        let mut f = BufReader::new(f);
        extract_compressed_audit_data(&mut f, limits)?
    };

    let decompressed_data =
        decompress_to_vec_zlib_with_limit(&compressed_audit_data, limits.decompressed_json_size)
            .map_err(|_| "Failed to decompress audit data")?;

    let stdout = std::io::stdout();
    let mut stdout = stdout.lock();
    stdout.write_all(&decompressed_data)?;

    Ok(())
}

fn extract_compressed_audit_data<T: Read>(
    reader: &mut T,
    limits: Limits,
) -> Result<Vec<u8>, Box<dyn Error>> {
    // In case you're wondering why the check for the limit is weird like that:
    // When .take() returns EOF, it doesn't tell you if that's because it reached the limit
    // or because the underlying reader ran out of data.
    // And we need to return an error when the reader is over limit, else we'll truncate the audit data.
    // So it would be reasonable to run `into_inner()` and check if that reader has any data remaining...
    // But readers can return EOF sporadically - a reader may return EOF,
    // then get more data and return bytes again instead of EOF!
    // So instead we read as many bytes as the limit allows, plus one.
    // If we've read the limit-plus-one bytes, that means the underlying reader was at least one byte over the limit.
    // That way we avoid any time-of-check/time-of-use issues.
    let incremented_limit = u64::saturating_add(limits.input_file_size as u64, 1);
    let f = BufReader::new(reader);
    let mut f = f.take(incremented_limit);
    let mut input_binary = Vec::new();
    f.read_to_end(&mut input_binary)?;
    if input_binary.len() as u64 == incremented_limit {
        Err("The input file is too large. Increase the input size limit to scan it")?
    }
    let compressed_audit_data = raw_auditable_data(&input_binary)?;
    if compressed_audit_data.len() > limits.decompressed_json_size {
        Err("Audit data size is over the limit even before decompression")?;
    }
    Ok(compressed_audit_data.to_owned())
}

fn parse_args() -> Result<(OsString, Limits), Box<dyn Error>> {
    let input = args_os().nth(1).ok_or(USAGE)?;
    let mut limits: Limits = Default::default();
    if let Some(s) = args_os().nth(2) {
        let utf8_s = s
            .to_str()
            .ok_or("Invalid UTF-8 in input size limit argument")?;
        limits.input_file_size = utf8_s.parse::<usize>()?
    }
    if let Some(s) = args_os().nth(3) {
        let utf8_s = s
            .to_str()
            .ok_or("Invalid UTF-8 in output size limit argument")?;
        limits.decompressed_json_size = utf8_s.parse::<usize>()?
    }
    Ok((input, limits))
}

#[derive(Copy, Clone, Eq, PartialEq, Hash)]
pub struct Limits {
    input_file_size: usize,
    decompressed_json_size: usize,
}

impl Default for Limits {
    fn default() -> Self {
        Self {
            input_file_size: 1024 * 1024 * 1024,      // 1GiB
            decompressed_json_size: 1024 * 1024 * 16, // 8MiB
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn input_file_limits() {
        let limits = Limits {
            input_file_size: 128,
            decompressed_json_size: 99999,
        };
        let fake_data = vec![0; 1024];
        let mut reader = std::io::Cursor::new(fake_data);
        let result = extract_compressed_audit_data(&mut reader, limits);
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("The input file is too large"));
    }
}
