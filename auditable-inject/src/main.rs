mod format_guess;
mod binary_file;

use format_guess::{FormatDescription, guess_format};

fn main() {
    let target_triple = std::env::args().nth(1).unwrap_or_else(|| usage() );
    let format = guess_format(&target_triple);
    

}

fn usage() -> ! {
    eprintln!("Usage: auditable-inject target-triple /path/to/data_to_inject /path/to/output_file");
    std::process::exit(1);
}