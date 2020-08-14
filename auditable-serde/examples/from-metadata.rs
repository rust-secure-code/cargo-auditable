use auditable_serde::VersionInfo;
use cargo_metadata::{Metadata, MetadataCommand};
use serde_json;
use std::{error::Error, convert::TryFrom};

fn print_usage_and_exit() -> ! {
    eprintln!("Usage: from-metadata [TARGET_PLATFORM]

Prints the audit data for the package in the current directory,
assuming default features.

TARGET_PLATFORM is anything that Cargo accepts as target triple,
e.g. 'x86_64-unknown-linux-gnu'");
    std::process::exit(1);
}

fn get_metadata(platform: &str) -> Metadata {
    let mut metadata_command = MetadataCommand::new();
    // if you need to change the feature set, you would set it here
    metadata_command.other_options(vec!["--filter-platform=".to_owned() + &platform]);
    metadata_command.exec().unwrap()
}

fn main() -> Result<(), Box<dyn Error>> {
    let platform = std::env::args().nth(1);
    match platform {
        None => print_usage_and_exit(),
        Some(platform) => {
            let stdout = std::io::stdout();
            let stdout = stdout.lock();
            let metadata = get_metadata(&platform);
            let version_info = VersionInfo::try_from(&metadata)?;
            serde_json::to_writer(stdout, &version_info)?;
            Ok(())
        },
    }
}
