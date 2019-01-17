#![forbid(unsafe_code)]

use std::env;
use std::io::prelude::*;
use std::fs::File;
use std::io;
use subslice::bmh;

fn main() {
    let argument = env::args().skip(1).next().expect("No file provided on command line");
    // TODO: exclude special files such as sockets, pipes and /dev/random
    let mut f = File::open(&argument).expect("Could not open provided file");
    let mut buffer = Vec::new();
    // TODO: read file in 1Mb chunks instead of reading it all at once
    // This has no memory limit and can cause memory exhaustion
    f.read_to_end(&mut buffer).expect("Reading the file failed");
    if is_an_executable(&buffer) {
        io::stdout().write(extract_auditable_info(&buffer)).unwrap();
    } else {
        eprintln!("'{}' does not seem to be an executable file, skipping.", &argument);
    }
    
}

const START_MARKER: &[u8] = b"CARGO_AUDIT_INFO_START;v0;\n";
const END_MARKER: &[u8] = b"\nCARGO_AUDIT_INFO_END\0";

fn extract_auditable_info(executable: &[u8]) -> &[u8] {
    let start_index = bmh::find(executable, START_MARKER)
                      .expect("No auditable information in the executable")
                      + START_MARKER.len();
    let content_length = bmh::find(&executable[start_index..], END_MARKER)
                         .expect("Malformed audit information: no end marker found");
    &executable[start_index..start_index+content_length]
}

// https://en.wikipedia.org/wiki/List_of_file_signatures
const EXECUTABLE_MAGIC_BYTES: [&[u8]; 7] = [
    b"\x7FELF", // UNIX ELF
    b"MZ", // DOS and Windows PE
    b"\xCA\xFE\xBA\xBE", // multi-architecture macOS
    b"\xFE\xED\xFA\xCE", // 32-bit macOS
    b"\xFE\xED\xFA\xCF", // 64-bit macOS
    b"\xCE\xFA\xED\xFE", // and now the same in reverse order
    b"\xCF\xFA\xED\xFE", // because they could
];

/// Checks start of the given slice for magic bytes indicating an executable file
fn is_an_executable(data: &[u8]) -> bool {
    for prefix in &EXECUTABLE_MAGIC_BYTES {
        if data.starts_with(prefix) {
            return true;
        }
    }
    false
}
