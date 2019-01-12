#![forbid(unsafe_code)]

use std::env;
use std::io::prelude::*;
use std::fs::File;
use std::io;
use subslice::bmh;

fn main() {
    let argument = env::args().skip(1).next().expect("No file provided on command line");
    let mut f = File::open(argument).expect("Could not open provided file");
    let mut buffer = Vec::new();
    f.read_to_end(&mut buffer).expect("Reading the file failed");
    io::stdout().write(extract_auditable_info(&buffer)).unwrap();
}

const START_MARKER: &[u8] = b"CARGO_AUDIT_INFO_START;v0;\n";
const END_MARKER: &[u8] = b"\nCARGO_AUDIT_INFO_END\0";

fn extract_auditable_info(executable: &[u8]) -> &[u8] {
    let start_index = bmh::find(executable, START_MARKER)
                      .expect("No auditable informmation in the executable")
                      + START_MARKER.len();
    let content_length = bmh::find(&executable[start_index..], END_MARKER)
                         .expect("Malformed audit information: no end marker found");
    &executable[start_index..start_index+content_length]
}
