## rust-audit-info

Command-line tool to extract the dependency trees embedded in binaries by `cargo auditable`.

It takes care of parsing the platform-specific formats ([ELF](https://en.wikipedia.org/wiki/Executable_and_Linkable_Format), [PE](https://en.wikipedia.org/wiki/Portable_Executable), [Mach-O](https://en.wikipedia.org/wiki/Mach-O)) and outputs the decompressed JSON.

This tool is intentionally minimal and does not implement vulnerability scanning on its own. However, it is useful for building your own vulnerability scanner. If you're looking for a Rust library instead of a command-line tool, see [`auditable-extract`](https://docs.rs/auditable-extract/).

### Features

 - Parses binaries from any supported platform, not just the platform it's running on.
 - Compiles down to a ~400Kb self-contained executable with no external dependencies.
 - Binary parsing designed from the ground up for resilience to malicious inputs.
 - 100% memory-safe Rust, including all dependencies. No memory-unsafe code anywhere in the dependency tree.
 - Cross-platform, portable, easy to cross-compile. Runs on [any Rust target with `std`](https://doc.rust-lang.org/stable/rustc/platform-support.html).
 - Supports setting size limits for both input and output, to protect against [OOMs](https://en.wikipedia.org/wiki/Out_of_memory) and [zip bombs](https://en.wikipedia.org/wiki/Zip_bomb).

### Usage

```bash
Usage: rust-audit-info FILE [INPUT_SIZE_LIMIT] [OUTPUT_SIZE_LIMIT]

The limits are specified in bytes. The default values are:

    INPUT_SIZE_LIMIT: 1073741824 (1 GiB)
    OUTPUT_SIZE_LIMIT: 67108864 (64 MiB)

```

The highest possible RAM usage is `INPUT_SIZE_LIMIT + OUTPUT_SIZE_LIMIT`, plus up to 1MB of overhead.

If you need to read from the standard input, pass `/dev/stdin` as the `FILE`.

### Dependencies

```
$ cargo geiger

Metric output format: x/y
    x = unsafe code used by the build
    y = total unsafe code found in the crate

Symbols: 
    🔒  = No `unsafe` usage found, declares #![forbid(unsafe_code)]
    ❓  = No `unsafe` usage found, missing #![forbid(unsafe_code)]
    ☢️   = `unsafe` usage found

Functions  Expressions  Impls  Traits  Methods  Dependency

0/0        0/0          0/0    0/0     0/0      🔒 rust-audit-info 0.4.0
0/0        0/0          0/0    0/0     0/0      🔒 ├── auditable-extract 0.3.1
0/0        0/0          0/0    0/0     0/0      🔒 │   └── binfarce 0.2.1
0/0        0/0          0/0    0/0     0/0      🔒 └── miniz_oxide 0.5.3
0/0        0/0          0/0    0/0     0/0      🔒     └── adler 1.0.2

0/0        0/0          0/0    0/0     0/0    

```
