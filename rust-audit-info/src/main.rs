#![forbid(unsafe_code)]

use auditable_info::{Limits, json_from_file};
use std::env::args_os;
use std::io::Write;
use std::path::PathBuf;
use std::error::Error;

const USAGE: &'static str = "\
Usage: rust-audit-info FILE [INPUT_SIZE_LIMIT] [OUTPUT_SIZE_LIMIT]

The limits are specified in bytes. The default values are:

    INPUT_SIZE_LIMIT: 1073741824 (1 GiB)
    OUTPUT_SIZE_LIMIT: 8388608 (8 MiB)
";

fn main() {
    if let Err(e) = actual_main() {
        eprintln!("{}", e);
        std::process::exit(1);
    }
}

fn actual_main() -> Result<(), Box<dyn Error>> {
    let (input, limits) = parse_args()?;
    let decompressed_data: String = json_from_file(&input, limits)?;

    let stdout = std::io::stdout();
    let mut stdout = stdout.lock();
    stdout.write_all(&decompressed_data.as_bytes())?;

    Ok(())
}

fn parse_args() -> Result<(PathBuf, Limits), Box<dyn Error>> {
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
    Ok((input.into(), limits))
}