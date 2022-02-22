mod format_guess;
mod binary_file;

use format_guess::{FormatDescription, guess_format};
use binary_file::create_metadata_file;

use object::BinaryFormat;

fn main() {
    let target_triple = std::env::args().nth(1).unwrap_or_else(|| usage() );
    let in_filename = std::env::args().nth(2).unwrap_or_else(|| usage() );
    let out_filename = std::env::args().nth(3).unwrap_or_else(|| usage() );

    let contents = std::fs::read("/etc/hosts").expect("Unable to read input file");

    let format = guess_format(&target_triple);
    let binfile = create_metadata_file(&format, &contents, "AUDITABLE_VERSION_INFO");
    std::fs::write("audit_data.o", binfile).expect("Unable to write output file");
}

fn usage() -> ! {
    eprintln!("Usage: auditable-inject target-triple /path/to/data_to_inject /path/to/output_file");
    std::process::exit(1);
}

// /// Section name for the audit data
// fn section_name(format: BinaryFormat) -> &'static str {
//     // referenced from
//     // https://github.com/Shnatsel/rust-audit/blob/995d3b11a38b540187684171a33ddd6c1f701612/auditable/src/lib.rs#L60-L62
//     match format {
//         BinaryFormat::Elf => ".rust-deps-v0",
//         BinaryFormat::MachO => "rust-deps-v0",
//         BinaryFormat::Pe => "rdep-v0",
//         _ => panic!("Unsupported binary format"),
//     }
// }