#![forbid(unsafe_code)]

use cargo_subcommand::Subcommand;

fn main() {
    // TODO: refactor cargo-subcommand to use os_args and OsStr types. Paths can be non-UTF-8 on most platforms.
    // TODO: fix https://github.com/dvc94ch/cargo-subcommand/issues/9, it's a release blocker
    let cmd = Subcommand::new(std::env::args(), "auditable", |_, _| Ok(false)).unwrap();
    println!("{:#?}", cmd);
}