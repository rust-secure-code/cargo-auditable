use std::env;
use std::fs::File;
use std::io;
use std::{
    io::{BufRead, BufReader, Seek, SeekFrom, Write},
    path::{Path, PathBuf, MAIN_SEPARATOR},
};

const DIRECTORY_TRAVERSAL_LIMIT: u16 = 20;

fn main() {
    let out_dir = env::var("OUT_DIR").unwrap();
    let dest_dir = Path::new(&out_dir);
    let mut f = File::create(dest_dir.join("Cargo.lock.annotated")).unwrap();
    let cargo_lock_location = get_cargo_lock_location();
    let stuff_to_write = std::fs::read_to_string(cargo_lock_location).unwrap();
    write!(&mut f, "{}", stuff_to_write).unwrap();
}

fn get_cargo_lock_location() -> PathBuf {
    if let Ok(user_input) = env::var("RUST_AUDIT_CARGO_LOCK_PATH") {
        PathBuf::from(user_input) // TODO: sanity check?
    } else if let Ok(path) = guess_cargo_lock_location(
        Path::new(&env::var("OUT_DIR").unwrap()),
        DIRECTORY_TRAVERSAL_LIMIT,
    ) {
        path
    } else {
        panic!("Could not automatically locate Cargo.lock file!
Consider setting RUST_AUDIT_CARGO_LOCK_PATH environment variable to the name of the Cargo.lock file to embed in the executable")
    }
}

/// Walks upwards in the directory structure until it finds Cargo.lock
/// that depends on `auditable` or reaches `traversal_limit`
fn guess_cargo_lock_location(starting_dir: &Path, traversal_limit: u16) -> Result<PathBuf, ()> {
    // FIXME: this breaks if `CARGO_TARGET_DIR` env variable is overridden
    // and set to something outside the directory with the crate
    // Unfortunately there doesn't seem to be a nice way around this,
    // since CARGO_MANIFEST_DIR points to the Cargo.lock of `auditable` crate
    // instead of the toplevel crate that's being built
    let mut up = "..".to_owned();
    up.push(MAIN_SEPARATOR);
    let mut new_dir = starting_dir.to_owned();
    for _ in 0..traversal_limit {
        new_dir = new_dir.join(&up);
        let filename = dbg!(new_dir.join("Cargo.lock"));
        if let Ok(mut file) = File::open(new_dir.join("Cargo.lock")) {
            if let Ok(true) = is_cargo_lock_with_auditable(&mut file) {
                return Ok(filename);
            }
        }
    }
    Err(())
}

/// Don't call directly, use `is_toplevel_cargo_lock()` instead
fn is_really_cargo_lock_with_auditable(file: &mut File) -> io::Result<bool> {
    let mut reader = BufReader::new(file);
    let mut line = String::new();
    while 0 != reader.read_line(&mut line)? {
        // FIXME: use cargo-lock crate instead of substring search hack
        if line.contains("auditable") {
            return Ok(true);
        }
    }
    Ok(false)
}

fn is_cargo_lock_with_auditable(file: &mut File) -> io::Result<bool> {
    let res = is_really_cargo_lock_with_auditable(file);
    // rewind file after we've read from it so that it can be used later
    file.seek(SeekFrom::Start(0)).unwrap();
    res
}
