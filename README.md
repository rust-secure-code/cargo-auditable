## cargo-auditable

Know the exact crate versions used to build your Rust executable. Audit binaries for known bugs or security vulnerabilities in production, at scale, with zero bookkeeping.

This works by embedding data about the dependency tree in JSON format into a dedicated linker section of the compiled executable.

The implementation has gotten to the point where it's time to get some real-world experience with it, but the **data format is not yet stable.** Linux, Windows and Mac OS are currently supported.

The end goal is to get Cargo itself to encode this information in binaries. There is an RFC for an implementation within Cargo, for which this project paves the way: https://github.com/rust-lang/rfcs/pull/2801

## Usage

```bash
# Install the tooling
cargo install cargo-auditable rust-audit-info
# Open your Rust project
cd your_project
# Build your project with dependency lists embedded in the binaries
cargo auditable build --release
# Recover the dependency info from the compiled binary
rust-audit-info target/release/your-project
```

## FAQ

### Doesn't this bloat my binary?

In a word, no. The embedded dependency list uses under 5kB even on large dependency trees with 400+ entries. This typically translates to between 1/1000 and 1/10,000 of the size of the binary.

### Is there any tooling to consume this data?

[trivy](https://github.com/aquasecurity/trivy) v0.31.0+ has support for detecting this data in binaries and reporting on vulnerabilities. See the [v0.31.0 release notes](https://github.com/aquasecurity/trivy/discussions/2716) for an end-to-end example.

[syft](https://github.com/anchore/syft) v0.53.0+ has experimental support for detecting this data in binaries.
When used on images or directories, Rust audit support must be enabled by adding the `--catalogers all` CLI option, e.g `syft --catalogers all <container image containing Rust auditable binary>`.

[go-rustaudit](https://github.com/microsoft/go-rustaudit) is a golang library for parsing the dependency list from binaries, used in syft and trivy.

It is also interoperable with existing tooling that consumes Cargo.lock via the [JSON-to-TOML convertor](auditable-serde/examples/json-to-toml.rs). You can also write your own tooling fairly easily - `auditable-extract` and `auditable-serde` crates handle all the data extraction and parsing for you. See [the docs](https://docs.rs/auditable-extract/) to get started.

### What is the data format, exactly?

It is not yet stabilized, so we do not have extensive docs or a JSON schema. However, [these Rust data structures](https://docs.rs/auditable-serde/latest/auditable_serde/struct.Package.html) map to JSON one-to-one and are extensively commented. The JSON is Zlib-compressed and placed in a linker section named `.dep-v0`.

### Can I read this data using a tool written in a different language?

Yes. The data format is designed for interoperability with alternative implementations. You can also use pre-existing platform-specific tools or libraries for data extraction. E.g. on Linux:
```bash
objcopy --dump-section .dep-v0=/dev/stdout target/release/hello-auditable | pigz -zd -
```
However, [don't run legacy tools on untrusted files](https://lcamtuf.blogspot.com/2014/10/psa-dont-run-strings-on-untrusted-files.html). Use the `auditable-extract` crate or the `rust-audit-info` command-line tool if possible - they are written in 100% safe Rust, so they will not have such vulnerabilities.

### What about embedded platforms?

Embedded platforms where you cannot spare a byte should not add anything in the executable. Instead they should record the hash of every executable in a database and associate the hash with its Cargo.lock, compiler and LLVM version, build date, etc. This would make for an excellent Cargo wrapper or plugin. Since that can be done in a 5-line shell script, writing that tool is left as an exercise to the reader.

### Does this impact reproducible builds?

The data format is designed not to disrupt reproducible builds. It contains no timestamps, and the generated JSON is sorted to make sure it is identical between compilations. If anything, this *helps* with reproducible builds, since you know all the versions for a given binary now.

### Does this disclose any sensitive information?

The list of enabled features is the only newly disclosed information.

All URLs and file paths are redacted, but the crate names, feature names and versions are recorded as-is. At present panic messages already disclose all this info and more, except feature names. Also, chances are that you're legally obligated have to disclose use of specific open-source crates anyway, since MIT and many other licenses require it.

### What about recording the compiler version?

It's already there, in the `.rustc` section. Run `strings your_executable | grep 'rustc version'` to see it. [Don't try this on files you didn't compile yourself](https://lcamtuf.blogspot.com/2014/10/psa-dont-run-strings-on-untrusted-files.html) - `strings` is overdue for a rewrite in a memory-safe language.

### What about keeping track of versions of statically linked C libraries?

Good question. I don't think they are exposed in any reasonable way right now. Would be a great addition, but not required for the initial launch. We can add it later in a backwards-compatible way.

### What is blocking uplifting this into Cargo?

We need to get some real-world experience with this before committing to a stable data format.

If you're looking to use it, we'd be happy to hear about your requirements so that we can accommodate them!
