## rust-audit

Know exact library versions used to build your Rust executable. Audit binaries for known bugs or security vulnerabilities in production, at scale, with zero bookkeeping.

This works by embedding contents of Cargo.lock in the compiled executable, which already contains versions and hashes of all dependencies and has good tooling around it. `auditable` crate embeds this info in executables and `rust-audit` recovers it for analysis. 

The implementation is a **proof of concept.** It's full of `unwrap()`s and I'm not sanitizing paths *at all.* Do not use in production just yet, but PRs are welcome. The end goal is to get Cargo itself to encode this information in binaries instead of relying on an external crate.

## Usage

 1. Add `auditable` as a dependency to your crate. 
 1. Add a call to `auditable::annotate_this_executable()` to your `main()` or any other reachable location in the code. Don't worry about performance, it will be compiled to a no-op.
 1. Run `rust-audit path/to/file` to recover the Cargo.lock used when compiling the executable. Feed the recovered file to [cargo-audit](https://github.com/RustSec/cargo-audit) to audit the binary for known vulnerabilities in it and its dependencies. 

**NB:** `auditable` currently requires nightly Rust due to the use of [test::black_box](https://doc.rust-lang.org/1.1.0/test/fn.black_box.html).

## How it works

Your Cargo.lock is embedded in your executable as `&'static str` at build time, with an added start and end markers. The code is exceedingly trivial, so I encourage to check it out.

The "call a no-op function" requirement is a hack to keep our info from getting optimized out by rustc. It even survives LTO, but is not ergonomic. Despite Rust stabilizing `#[used]` annotation, you still need [low-level platform-specific hacks](https://github.com/rust-lang/rust/issues/47384) to preserve an unused static that comes from a library. Hopefully we'll get cooperation from the compiler if/when this functionality is uplifted in Cargo.

## Demo

```bash
# clone this repository
git clone https://github.com/Shnatsel/rust-audit.git
cd rust-audit
# compile a binary with Cargo.lock embedded in it
cd hello-auditable
cargo build --release
# recover the Cargo.lock we've just embedded
cd ../rust-audit
cargo run -- ../hello-auditable/target/release/hello-auditable
# audit the compiled `hello-auditable` executable for known vulnerabilities
cargo install cargo-audit
cargo run -- ../hello-auditable/target/release/hello-auditable | cargo audit -f /dev/stdin
```

## FAQ

### Doesn't this bloat my binary?

Not really. A "Hello World" on x86 Linux compiles into a ~1Mb file in the best case (nightly without jemalloc, LTO enabled). Its Cargo.lock even with a couple of dependencies is < 1Kb, that's under 1/1000 of the size. The size of Cargo.lock grows linearly with the number of dependencies, so it will keep being negligible.

### What about embedded platforms?

Embedded platforms where you cannot spare a byte should not add anything in the executable. Instead they should record the hash of every executable in a database and associate the hash with its Cargo.lock, compiler and LLVM version, build date, etc. This would make for an excellent Cargo wrapper or plugin. Since that can be done in a 5-line shell script, writing that tool is left as an exercise to the reader.

### What about embedding compiler version?

It's already there. Run `strings your_executable | grep 'rustc version'` to see it. [Don't try this on files you didn't compile yourself](https://lcamtuf.blogspot.com/2014/10/psa-dont-run-strings-on-untrusted-files.html). Also, `strings` is due for a rewrite in safe Rust.

### What about keeping track of versions of statically linked C libraries?

Good question. I don't think they are exposed in any reasonable way right now. Would be a great addition, but not required for the initial launch. We can add it later in a backwards-compatible way by appending it to the existing Cargo.lock data.

### What is blocking uplifting this into Cargo?

Two things:
 1. Figuring out a way to get rustc to cooperate and not optimize out our info without code modifications in the target crate (i.e. no more "call a no-op function" weirdness). This is much easier if we're allowed to make modifications to Cargo and the compiler.
 1. Actually writing an RFC to make the proposal official.

**Help on these points would be greatly appreciated.**

This is not blocked on getting [cargo-audit](https://github.com/RustSec/cargo-audit) an official status or anything like that, since it is already useful on its own.

