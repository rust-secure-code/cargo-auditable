#![forbid(unsafe_code)]

use miniz_oxide::inflate::decompress_to_vec_zlib_with_limit;
use auditable_extract::raw_auditable_data;
use std::io::Read;
use std::io::Write;
use std::{error::Error, fs::File, io::BufReader};

fn main() -> Result<(), Box<dyn Error>> {
    // TODO: use pico-args
    let input = std::env::args().nth(1).unwrap();
    //let output = std::env::args().nth(2).unwrap();
    // Copy the compressed data and drop the full binary we've read to reduce peak memory usage
    let compressed_audit_data: Vec<u8> = {
        let f = File::open(input)?;
        let mut f = BufReader::new(f);
        let mut input_binary = Vec::new();
        f.read_to_end(&mut input_binary)?;
        let compressed_audit_data = raw_auditable_data(&input_binary).ok_or("No audit data found in the supplied binary")?;
        compressed_audit_data.to_owned()
    };
    let decompressed_data = decompress_to_vec_zlib_with_limit(&compressed_audit_data, 1024 * 1024 * 128)
        .map_err(|_| "Failed to decompress audit data")?;
    let stdout = std::io::stdout();
    let mut stdout = stdout.lock();
    stdout.write_all(&decompressed_data)?;

    Ok(())
}
