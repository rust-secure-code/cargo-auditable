Extracts the dependency tree information embedded in executables by
[`cargo auditable`](https://github.com/rust-secure-code/cargo-auditable).

This crate parses platform-specific binary formats ([ELF](https://en.wikipedia.org/wiki/Executable_and_Linkable_Format),
[PE](https://en.wikipedia.org/wiki/Portable_Executable),
[Mach-O](https://en.wikipedia.org/wiki/Mach-O), [WASM](https://en.wikipedia.org/wiki/WebAssembly)) and obtains the compressed audit data.

Unlike other binary parsing crates, it is specifically designed to be resilient to malicious input.
It 100% safe Rust (including all dependencies) and performs no heap allocations.

## Usage

**Note:** this is a low-level crate that only implements binary parsing. It rarely should be used directly.
You probably want the higher-level [`auditable-info`](https://docs.rs/auditable-info) crate instead.

The following snippet demonstrates full extraction pipeline using this crate, including decompression
using the safe-Rust [`miniz_oxide`](http://docs.rs/miniz_oxide/) and optional JSON parsing
via [`auditable-serde`](http://docs.rs/auditable-serde/):

```rust,ignore
use std::io::{Read, BufReader};
use std::{error::Error, fs::File, str::FromStr};
!
fn main() -> Result<(), Box<dyn Error>> {
    // Read the input
    let f = File::open("target/release/hello-world")?;
    let mut f = BufReader::new(f);
    let mut input_binary = Vec::new();
    f.read_to_end(&mut input_binary)?;
    // Extract the compressed audit data
    let compressed_audit_data = auditable_extract::raw_auditable_data(&input_binary)?;
    // Decompress it with your Zlib implementation of choice. We recommend miniz_oxide
    use miniz_oxide::inflate::decompress_to_vec_zlib;
    let decompressed_data = decompress_to_vec_zlib(&compressed_audit_data)
        .map_err(|_| "Failed to decompress audit data")?;
    let decompressed_data = String::from_utf8(decompressed_data)?;
    println!("{}", decompressed_data);
    // Parse the audit data to Rust data structures
    let dependency_tree = auditable_serde::VersionInfo::from_str(&decompressed_data);
    Ok(())
}
```

## WebAssembly support

We use a third-party crate [`wasmparser`](https://crates.io/crates/wasmparser)
created by Bytecode Alliance for parsing WebAssembly.
It is a robust and high-quality parser, but its dependencies contain some `unsafe` code,
most of which is not actually used in our build configuration.

We have manually audited it and found it to be sound.
Still, the security guarantees for it are not as ironclad as for other parsers.
Because of that WebAssembly support is gated behind the optional `wasm` feature.
Be sure to [enable](https://doc.rust-lang.org/cargo/reference/features.html#dependency-features)
the `wasm` feature if you want to parse WebAssembly.
