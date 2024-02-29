use std::{io::Write, path::Path};

use auditable_cyclonedx::auditable_to_minimal_cdx;
use auditable_info::audit_info_from_file;

mod workarounds;

fn main() {
    let input_filename = std::env::args_os()
        .nth(1)
        .expect("No input file specified!");
    let info = audit_info_from_file(Path::new(&input_filename), Default::default()).unwrap();
    let cyclonedx = auditable_to_minimal_cdx(&info);
    let mut json_bytes: Vec<u8> = Vec::new();
    cyclonedx.output_as_json_v1_3(&mut json_bytes).unwrap();
    let min_json = workarounds::minify_bom(&json_bytes);
    let mut stdout = std::io::stdout().lock();
    stdout.write_all(min_json.as_bytes()).unwrap();
}
