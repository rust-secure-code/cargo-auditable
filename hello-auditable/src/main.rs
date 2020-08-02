auditable::inject_dependency_list!(COMPRESSED_DEPENDENCY_LIST);

fn main() {
    println!("Hello, world!");
    // Actually use the data to work around a bug in rustc:
    // https://github.com/rust-lang/rust/issues/47384
    // on nightly `test::black_box` works better than `println!`
    println!("{}", COMPRESSED_DEPENDENCY_LIST[0]);
}
