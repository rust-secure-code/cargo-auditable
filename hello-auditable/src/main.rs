fn main() {
    println!("Hello, world!");
    println!("My version info is:\n{}", std::str::from_utf8(auditable::version_info()).unwrap());
}
