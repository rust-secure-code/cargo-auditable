## cargo-auditable

Know the exact crate versions used to build your Rust executable. Audit binaries for known bugs or security vulnerabilities in production, at scale, with zero bookkeeping.

This works by embedding data about the dependency tree in JSON format into a dedicated linker section of the compiled executable.

Linux, Windows and Mac OS are officially supported. [WebAssembly](https://en.wikipedia.org/wiki/WebAssembly) is also supported starting with v0.6.3. All other ELF targets should work, but are not tested on CI.

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

### On nightly Rust

On nightly we can take advantage of Cargo's [native SBOM precursor](https://doc.rust-lang.org/cargo/reference/unstable.html#sbom) to record dependencies more accurately:

```bash
CARGO_BUILD_SBOM=true cargo +nightly auditable build -Z sbom --release
```

Due to [a bug in Cargo](https://github.com/rust-lang/cargo/issues/15695) you may have to `touch src/*` or `cargo clean` first if you also used `cargo auditable` without `-Z sbom` in the same project.

### Through other tools

If you're not calling `cargo` directly and cannot change how it's invoked, you can use `cargo auditable` as a drop-in replacement for `cargo`. See [here](REPLACING_CARGO.md) for details.

## Adoption

Microsoft uses `cargo auditable` internally and maintains the [data extraction library for Go](https://github.com/microsoft/go-rustaudit).

Multiple Linux distributions build their Rust packages with `cargo auditable`: [Alpine Linux](https://www.alpinelinux.org/), [NixOS](https://nixos.org/), [openSUSE](https://www.opensuse.org/), [Void Linux](https://voidlinux.org/) and [Chimera Linux](https://chimera-linux.org/). If you install packages from their repositories, you can audit them!

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
* [osv-scanner](https://github.com/google/osv-scanner/) v2.0.1+ [reads this data](https://github.com/google/osv-scalibr/pull/377) when scanning container images.
* [grype](https://github.com/anchore/grype) v0.83.0+ embeds syft, which detects this data in binaries and container images and reports on vulnerabilities.

#### Recovering the dependency list

* [syft](https://github.com/anchore/syft) v1.15.0+ has support for detecting this data in binaries, directories and container images and printing it in various formats.
* [blint](https://github.com/owasp-dep-scan/blint) v2.1.3+ can recover this data and output it as CycloneDX.
* [wasm-tools](https://github.com/bytecodealliance/wasm-tools) v1.227.0+ can recover this data from WebAssembly. Try `wasm-tools metadata show`.
* [rust-audit-info](https://crates.io/crates/rust-audit-info) recovers the dependency list from a binary and prints it in JSON.
* [auditable2cdx](https://crates.io/crates/auditable2cdx) recovers the dependency list from a binary and prints it in CycloneDX.
* [docker](https://docs.docker.com/build/metadata/attestations/sbom/) supports embedding CycloneDX documents into container images. These are recovered using [BuildKit Syft scanner](https://github.com/docker/buildkit-syft-scanner), which embeds syft. If you build a container image with `docker buildx build --tag <namespace>/<image>:<version> --attest type=sbom --push .` and use `cargo auditable` to build rust binaries in the `Dockerfile`, the SBOM attestation attached to the container image will include your rust dependencies.

### Can I read this data using a tool written in a different language?

Yes. The data format is designed for interoperability with alternative implementations. In fact, parsing it only takes [5 lines of Python](PARSING.md). See [here](PARSING.md) for documentation on parsing the data.

Besides that, Syft can read it and convert it to a multitude of formats. `auditable2cdx` can convert it to CycloneDX, which is understood by most tools. This conversion lets you feed this data even to tools you cannot modify.

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

The compiler itself [embeds it](https://github.com/rust-lang/rust/pull/97550) in v1.73 and later.

On older versions it's already there in the debug info. On Unix you can run `strings your_executable | grep 'rustc version'` to see it.

### What about keeping track of versions of statically linked C libraries?

Good question. I don't think they are exposed in any reasonable way right now. Would be a great addition, but not required for the initial launch. We can add it later in a backwards-compatible way. Adopting [the `-src` crate convention](https://internals.rust-lang.org/t/statically-linked-c-c-libraries/17175?u=shnatsel) would make it happen naturally, and will have other benefits as well, so that's probably the best route.

### Does this protect against supply chain attacks?

No. Use [`cargo-vet`](https://github.com/mozilla/cargo-vet) or [`cargo-crev`](https://github.com/crev-dev/cargo-crev) for that.

[Software Bills of Materials](https://en.wikipedia.org/wiki/Software_supply_chain) (SBOMs) do not prevent supply chain attacks. They cannot even be used to assess the impact of such an attack after it is discovered, because any malicious library worth its bytes will remove itself from the SBOM. This applies to nearly every language and build system, not just Rust and Cargo.

Do not rely on SBOMs when dealing with supply chain attacks!

### What is blocking uplifting this into Cargo?

The [RFC for this functionality in Cargo itself](https://github.com/rust-lang/rfcs/pull/2801) has been [postponed](https://github.com/rust-lang/rfcs/pull/2801#issuecomment-2122880841) by the Cargo team until the [more foundational SBOM RFC](https://github.com/rust-lang/rfcs/pull/3553).

That RFC has now been implemented and is available via an [unstable feature](https://doc.rust-lang.org/cargo/reference/unstable.html#sbom). This opens the door to submitting an RFC for this functionality into `cargo` itself once again.
