## rust-audit

Know exact library versions used to build your Rust executable. Audit binaries for known bugs or security vulnerabilities in production, at scale, with zero bookkeeping. This works by embedding data about the dependency tree in JSON format into a dedicated linker section of the compiled executable.

The implementation has gotten to the point where it's time to get some real-world experience with it, but the format is not yet stable. Linux, Windows and Mac OS are currently supported.

The end goal is to get Cargo itself to encode this information in binaries instead of relying on an external crate. RFC for a proper implementation in Cargo, for which this project paves the way: https://github.com/rust-lang/rfcs/pull/2801

## Demo

```bash
# clone this repository
git clone https://github.com/Shnatsel/rust-audit.git
cd rust-audit
# compile the tooling and a sample binary with dependency tree embedded
cargo build --release
# recover the dependency tree we've just embedded
# Note: pure-Rust extractor is Linux-only for now. Support for other platforms is WIP.
target/release/rust-audit-info target/release/hello-auditable
# Or use pre-existing platform-specific tooling for data extraction, e.g. on Linux:
objcopy -O binary --only-section=.rust-deps-v0 target/release/hello-auditable /dev/stdout | pigz -zd -
```

You can also audit the recovered dependency tree for known vulnerabilities using `cargo-audit`:
```bash
(cd auditable-serde && cargo build --release --features "toml" --example json-to-toml)
cargo install cargo-audit
target/release/rust-audit-info target/release/hello-auditable > dependency-tree.json
target/release/examples/json-to-toml dependency-tree.json | cargo audit -f -
```

## Make your crate auditable

Add the following to your `Cargo.toml`:

```toml
build = "build.rs"

[dependencies]
auditable = "0.1"

[build-dependencies]
auditable-build = "0.1"
```

Create a build.rs file next to `Cargo.toml` with the following contents:
```rust
fn main() {
    auditable_build::collect_dependency_list();
}
```

Add the following to your `main.rs` (or any other file for that matter):

```rust
static COMPRESSED_DEPENDENCY_LIST: &[u8] = auditable::inject_dependency_list!();
```

Put the following in some reachable location in the code, e.g. in `fn main()`:
```rust
    // Actually use the data to work around a bug in rustc:
    // https://github.com/rust-lang/rust/issues/47384
    // On nightly you can use `test::black_box` instead of `println!`
    println!("{}", COMPRESSED_DEPENDENCY_LIST[0]);
```

Now you can `cargo build` and the dependency data will be embedded in the final binary automatically. You can verify that the data is actually embedded using the steps from [the demo](#Demo).

See the [auditable "Hello, world!"](https://github.com/Shnatsel/rust-audit/tree/master/hello-auditable) project for an example of how it all fits together.

## FAQ

### Doesn't this bloat my binary?

Not really. A "Hello World" on x86 Linux compiles into a ~1Mb file in the best case (recent Rust without jemalloc, LTO enabled). Its dependency tree even with a couple of dependencies is < 1Kb, that's under 1/1000 of the size. We also compress it with zlib to drive the size down further. Since the size of dependency tree info grows linearly with the number of dependencies, it will keep being negligible.

### What about embedded platforms?

Embedded platforms where you cannot spare a byte should not add anything in the executable. Instead they should record the hash of every executable in a database and associate the hash with its Cargo.lock, compiler and LLVM version, build date, etc. This would make for an excellent Cargo wrapper or plugin. Since that can be done in a 5-line shell script, writing that tool is left as an exercise to the reader.

### What about recording compiler version?

It's already there. Run `strings your_executable | grep 'rustc version'` to see it. [Don't try this on files you didn't compile yourself](https://lcamtuf.blogspot.com/2014/10/psa-dont-run-strings-on-untrusted-files.html) - `strings` is overdue for a rewrite in safe Rust.

### What about reproducible builds?

The data format is designed not to disrupt reproducible builds. It contains no timestamps, and the generated JSON is sorted to make sure it is identical between compilations.

### What about keeping track of versions of statically linked C libraries?

Good question. I don't think they are exposed in any reasonable way right now. Would be a great addition, but not required for the initial launch. We can add it later in a backwards-compatible way later.

### Is there any tooling to consume this data?

It is interoperable with existing tooling that consumes Cargo.lock via the JSON-to-TOML convertor. You can also write your own tooling fairly easily - `auditable-extract` and `auditable-serde` crates handle all the data extraction and parsing for you.

### What is blocking uplifting this into Cargo?

 1. Getting some real-world experience with this before committing to a stable data format
 1. The rustc bug that makes this non-ergonomic is also an issue: https://github.com/rust-lang/rust/issues/47384

**Help on these points would be greatly appreciated.**

