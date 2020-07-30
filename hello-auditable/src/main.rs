fn main() {
    auditable::annotate_this_executable();
    println!("Hello, world!");
    println!("My version info is:\n{}", std::str::from_utf8(auditable::version_info()).unwrap());
}
