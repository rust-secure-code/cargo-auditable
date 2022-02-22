mod format_guess;
mod binary_file;

use format_guess::guess_format;
use binary_file::create_metadata_file;

fn main() {
    let target_triple = std::env::args().nth(1).unwrap_or_else(|| usage() );
    let in_filename = std::env::args().nth(2).unwrap_or_else(|| usage() );

    let contents = std::fs::read(in_filename).expect("Unable to read input file");

    let format = guess_format(&target_triple);
    let binfile = create_metadata_file(&format, &contents, "AUDITABLE_VERSION_INFO");
    std::fs::write("audit_data.o", binfile).expect("Unable to write output file");
}

fn usage() -> ! {
    eprintln!("Usage: auditable-inject target-triple /path/to/data_to_inject");
    eprintln!("Then use the following before compiling:");
    eprintln!("export RUSTFLAGS='-Clink-arg=audit_data.o -Clink-arg=-Wl,--require-defined=AUDITABLE_VERSION_INFO'");
    std::process::exit(1);
}