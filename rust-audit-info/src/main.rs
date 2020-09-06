#![forbid(unsafe_code)]

use miniz_oxide::inflate::decompress_to_vec_zlib_with_limit;
use auditable_extract::raw_auditable_data;
use std::io::Read;
use std::io::Write;
use std::{error::Error, fs::File, io::BufReader};

const INPUT_FILE_LENGTH_LIMIT: u64 = (1024 + 512) * 1024 * 1024; // 1.5Gib
const DECOMPRESSED_AUDIT_DATA_SIZE_LIMIT: usize = 1024 * 1024 * 64; // 64Mib

fn main() -> Result<(), Box<dyn Error>> {
    // TODO: use pico-args
    let input = std::env::args().nth(1).unwrap();

    // Copy the compressed data and drop the full binary we've read to reduce peak memory usage
    let compressed_audit_data: Vec<u8> = {
        let f = File::open(input)?;
        let f = BufReader::new(f);
        let mut f = f.take(INPUT_FILE_LENGTH_LIMIT); //TODO: nice error message
        let mut input_binary = Vec::new();
        f.read_to_end(&mut input_binary)?;
        let compressed_audit_data = raw_auditable_data(&input_binary)?;
        if compressed_audit_data.len() > DECOMPRESSED_AUDIT_DATA_SIZE_LIMIT {
            Err("Audit data size is over the limit even before decompression")?;
        }
        compressed_audit_data.to_owned()
    };
    let decompressed_data = decompress_to_vec_zlib_with_limit(
        &compressed_audit_data,
        DECOMPRESSED_AUDIT_DATA_SIZE_LIMIT)
        .map_err(|_| "Failed to decompress audit data")?;
    let stdout = std::io::stdout();
    let mut stdout = stdout.lock();
    stdout.write_all(&decompressed_data)?;

    Ok(())
}
