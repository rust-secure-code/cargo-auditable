static COMPRESSED_DEPENDENCY_LIST: &[u8] = auditable::inject_dependency_list!();

fn main() {
    println!("Hello, world!");
    // Actually use the data to work around a bug in rustc:
    // https://github.com/rust-lang/rust/issues/47384
    // On nightly you can use `test::black_box` instead of `println!`
    println!("{}", COMPRESSED_DEPENDENCY_LIST[0]);
}
