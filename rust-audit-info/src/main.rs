#![forbid(unsafe_code)]

use miniz_oxide::inflate::decompress_to_vec_zlib_with_limit;
use auditable_extract::raw_auditable_data;
use std::io::Read;
use std::io::Write;
use std::{error::Error, fs::File, io::BufReader};

fn main() {
    if let Err(e) = do_work() {
        eprintln!("{}", e);
        std::process::exit(1);
    }
}

fn do_work() -> Result<(), Box<dyn Error>> {
    // TODO: use pico-args
    let input = std::env::args().nth(1)
        .ok_or("Usage: rust-audit-info FILE")?;

    let limits: Limits = Default::default(); // TODO: read from CLI arguments

    let compressed_audit_data: Vec<u8> = {
        let f = File::open(input)?;
        let mut f = BufReader::new(f);
        extract_compressed_audit_data(&mut f, Default::default())?
    };

    let decompressed_data = decompress_to_vec_zlib_with_limit(
        &compressed_audit_data,
        limits.decompressed_json_size)
        .map_err(|_| "Failed to decompress audit data")?;

    let stdout = std::io::stdout();
    let mut stdout = stdout.lock();
    stdout.write_all(&decompressed_data)?;

    Ok(())
}

fn extract_compressed_audit_data<T: Read>(reader: &mut T, limits: Limits) -> Result<Vec<u8>, Box<dyn Error>> {
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

#[derive(Copy, Clone, Eq, PartialEq, Hash)]
pub struct Limits {
    input_file_size: usize,
    decompressed_json_size: usize,
}

impl Default for Limits {
    fn default() -> Self {
        Self {
            input_file_size: 1024 * 1024 * 1024, // 1Gib
            decompressed_json_size: 1024 * 1024 * 64, // 64Mib
        }
    }
}