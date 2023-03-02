## cargo-auditable

Know the exact crate versions used to build your Rust executable. Audit binaries for known bugs or security vulnerabilities in production, at scale, with zero bookkeeping.

This works by embedding data about the dependency tree in JSON format into a dedicated linker section of the compiled executable.

Linux, Windows and Mac OS are officially supported. All other ELF targets should work, but are not tested on CI. WASM is currently not supported, but patches are welcome.

The end goal is to get Cargo itself to encode this information in binaries. There is an RFC for an implementation within Cargo, for which this project paves the way: https://github.com/rust-lang/rfcs/pull/2801

## Usage

```bash
# Install the tools
cargo install cargo-auditable cargo-audit
# Build your project with dependency lists embedded in the binaries
cargo auditable build --release
# Scan the binary for vulnerabilities
cargo audit bin target/release/your-project
```

`cargo auditable` works with any Cargo command. All arguments are passed to `cargo` as-is.

## FAQ

### Doesn't this bloat my binary?

In a word, no. The embedded dependency list uses under 4kB even on large dependency trees with 400+ entries. This typically translates to between 1/1000 and 1/10,000 of the size of the binary.

### Can I make `cargo` always build with `cargo auditable`?

Yes! For example, on Linux/macOS/etc add this to your `.bashrc`:

```bash
alias cargo="cargo auditable"
```

If you're using a shell other than bash, or if using an alias is not an option, [see here.](REPLACING_CARGO.md)

### Is there any tooling to consume this data?

#### Vulnerability reporting

* [cargo audit](https://crates.io/crates/cargo-audit) v0.17.3+ can detect this data in binaries and report on vulnerabilities. See [here](https://github.com/rustsec/rustsec/tree/main/cargo-audit#cargo-audit-bin-subcommand) for details.
* [trivy](https://github.com/aquasecurity/trivy) v0.31.0+ detects this data in binaries and reports on vulnerabilities. See the [v0.31.0 release notes](https://github.com/aquasecurity/trivy/discussions/2716) for an end-to-end example.

#### Recovering the dependency list

* [syft](https://github.com/anchore/syft) v0.53.0+ has experimental support for detecting this data in binaries.
When used on images or directories, Rust audit support must be enabled by adding the `--catalogers all` CLI option, e.g `syft --catalogers all <container image containing Rust auditable binary>`.
* [rust-audit-info](https://crates.io/crates/rust-audit-info) recovers the dependency list from a binary and prints it in JSON.

It is also interoperable with existing tooling that consumes Cargo.lock via the [JSON-to-TOML convertor](auditable-serde/examples/json-to-toml.rs). However, we recommend supporting the format natively; the format is designed to be [very easy to parse](PARSING.md), even if your language does not have a library for that yet.

### Can I read this data using a tool written in a different language?

Yes. The data format is designed for interoperability with alternative implementations. In fact, parsing it only takes [5 lines of Python](PARSING.md). See [here](PARSING.md) for documentation on parsing the data.

### What is the data format, exactly?

The data format is described by the JSON schema [here](cargo-auditable.schema.json).
The JSON is Zlib-compressed and placed in a linker section named `.dep-v0`.
You can find more info about parsing it [here](PARSING.md).

### What about embedded platforms?

Embedded platforms where you cannot spare a byte should not add anything in the executable. Instead they should record the hash of every executable in a database and associate the hash with its Cargo.lock, compiler and LLVM version, build date, etc. This would make for an excellent Cargo wrapper or plugin. Since that can be done in a 5-line shell script, writing that tool is left as an exercise to the reader.

### Does this impact reproducible builds?

The data format is specifically designed not to disrupt reproducible builds. It contains no timestamps, and the generated JSON is sorted to make sure it is identical between compilations. If anything, this *helps* with reproducible builds, since you know all the versions for a given binary now.

### Does this disclose any sensitive information?

No. All URLs and file paths are redacted, but the crate names and versions are recorded as-is. At present panic messages already disclose all this info and more. Also, chances are that you're legally obligated have to disclose use of specific open-source crates anyway, since MIT and many other licenses require it.

### What about recording the compiler version?

The compiler itself [will start embedding it soon.](https://github.com/rust-lang/rust/pull/97550)

On older versions it's already there in the debug info. On Unix you can run `strings your_executable | grep 'rustc version'` to see it.

### What about keeping track of versions of statically linked C libraries?

Good question. I don't think they are exposed in any reasonable way right now. Would be a great addition, but not required for the initial launch. We can add it later in a backwards-compatible way. Adopting [the `-src` crate convention](https://internals.rust-lang.org/t/statically-linked-c-c-libraries/17175?u=shnatsel) would make it happen naturally, and will have other benefits as well, so that's probably the best route.

### Does this protect against supply chain attacks?

No. Use [`cargo-vet`](https://github.com/mozilla/cargo-vet) or [`cargo-crev`](https://github.com/crev-dev/cargo-crev) for that.

[Software Bills of Materials](https://en.wikipedia.org/wiki/Software_supply_chain) (SBOMs) do not prevent supply chain attacks. They cannot even be used to assess the impact of such an attack after it is discovered, because any malicious library worth its bytes will remove itself from the SBOM. This applies to nearly every language and build system, not just Rust and Cargo.

Do not rely on SBOMs when dealing with supply chain attacks!

### What is blocking uplifting this into Cargo?

Cargo itself is [currently in a feature freeze](https://blog.rust-lang.org/inside-rust/2022/03/31/cargo-team-changes.html).
