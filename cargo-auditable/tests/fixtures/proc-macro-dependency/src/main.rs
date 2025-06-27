use serde::{Deserialize, Serialize};

fn main() {
    println!("{:?}", Hello("Hello, world!"));
}

#[derive(Serialize, Deserialize, Debug)]
struct Hello (&'static str);
