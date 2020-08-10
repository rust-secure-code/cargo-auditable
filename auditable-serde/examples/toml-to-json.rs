use auditable_serde::VersionInfo;
use cargo_lock::Lockfile;
use serde_json;

fn main() {
    let path = std::env::args().skip(1).next().expect("No file specified");
    let parsed_toml = Lockfile::load(path).unwrap();
    let version_info: VersionInfo = (&parsed_toml).into();
    let stdout = std::io::stdout();
    let stdout = stdout.lock();
    serde_json::to_writer(stdout, &version_info).unwrap();
}
