## rust-audit

Know exact library versions used to build your Rust executable. Audit binaries for known bugs or security vulnerabilities at scale, in production, with zero bookkeeping.

This works by embedding contents of Cargo.lock in the compiled executable, which already contains versions and hashes of all dependencies and has good tooling around it.

The implementation is currently a **proof of concept,** but I'd like to evolve it into an actually usable system and get some real-world experience with it. The end goal is to get Cargo itself to encode this information in binaries instead of relying on an external crate. PRs are welcome.

RFC for a proper implementation in Cargo, for which `auditable` crate paves the way: https://github.com/rust-lang/rfcs/pull/2801

## Usage

 1. Add `auditable` as a dependency to your crate.
 1. Run `objcopy -O binary --only-section=.rust-audit-dep-list path/to/file` (or your platform equivalent) to recover the Cargo.lock used when compiling the executable.
 1. Feed the recovered file to [cargo-audit](https://github.com/RustSec/cargo-audit) to audit the binary for known vulnerabilities in it and its dependencies.

Optional: access the version info from within the binary itself by calling `auditable::version_info()`. See "hello-auditable" folder for an example.

## Demo

```bash
# clone this repository
git clone https://github.com/Shnatsel/rust-audit.git
cd rust-audit
# compile a binary with Cargo.lock embedded in it
cd hello-auditable
cargo build --release
# recover the Cargo.lock we've just embedded
objcopy -O binary --only-section=.rust-audit-dep-list target/release/hello-auditable Cargo.lock.extracted
# audit the compiled `hello-auditable` executable for known vulnerabilities
cargo install cargo-audit
cargo run -- ../hello-auditable/target/release/hello-auditable | cargo audit -f /dev/stdin
```

## How it works

Your Cargo.lock is embedded in your executable as `&'static str` at build time into a dedicated linker section. We can then recover it with `objcopy` on Linux or the appropriate tools on other operating systems (a pure-Rust extractor for all major platforms would be welcome). The code is quite trivial, so I encourage you to check it out.

## FAQ

### Doesn't this bloat my binary?

Not really. A "Hello World" on x86 Linux compiles into a ~1Mb file in the best case (recent Rust without jemalloc, LTO enabled). Its Cargo.lock even with a couple of dependencies is < 1Kb, that's under 1/1000 of the size. The size of Cargo.lock grows linearly with the number of dependencies, so it will keep being negligible.

### What about embedded platforms?

Embedded platforms where you cannot spare a byte should not add anything in the executable. Instead they should record the hash of every executable in a database and associate the hash with its Cargo.lock, compiler and LLVM version, build date, etc. This would make for an excellent Cargo wrapper or plugin. Since that can be done in a 5-line shell script, writing that tool is left as an exercise to the reader.

### What about embedding compiler version?

It's already there. Run `strings your_executable | grep 'rustc version'` to see it. [Don't try this on files you didn't compile yourself](https://lcamtuf.blogspot.com/2014/10/psa-dont-run-strings-on-untrusted-files.html). Also, `strings` is due for a rewrite in safe Rust.

### What about keeping track of versions of statically linked C libraries?

Good question. I don't think they are exposed in any reasonable way right now. Would be a great addition, but not required for the initial launch. We can add it later in a backwards-compatible way by appending it to the existing Cargo.lock data.

### What is blocking uplifting this into Cargo?

 1. Implementing the design as a crate (this one, it's WIP) and getting some real-world experience with it
 1. Trying out a stable format (Cargo.lock is unstable) and seeing how that works in practice before committing to it

**Help on these points would be greatly appreciated.**

This is not blocked on getting [cargo-audit](https://github.com/RustSec/cargo-audit) an official status or anything like that, since rust-audit is already useful on its own.

