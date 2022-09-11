## Parsing the data embedded by `cargo auditable`

This document describes the steps to implement your own parser, if your language doesn't have one yet. Since the format simply uses Zlib and JSON, implementing a parser should be quite trivial. To give you a sense, this is a parser for Linux binaries written in Bash:

```bash
objcopy --dump-section .dep-v0=/dev/stdout $1 | pigz -zd -
```

**Note:** we provide the cross-platform tool [`rust-audit-info`](rust-audit-info/README.md) which can be called as a subprocess from any language. It will handle all the binary wrangling for you and output the JSON embedded in the binary. It is designed for robustness and written in 100% safe Rust. You can use it to bypass implementing a parser entirely.

### Step 1: Obtain the compressed data from the binary

Use your language's recommended ELF/Mach-O/PE parser to extract the `.dep-v0` section from the executable. On Apple platforms (in Mach-O format) this section is in the `__DATA` segment; other formats do not have the concept of segments.

### Step 2: Decompress the data

The data is [Zlib](https://en.wikipedia.org/wiki/Zlib)-compressed. Simply decompress it.

If you want to protect your process from memory exhaustion, limit the size of the output to avoid [zip bombs](https://en.wikipedia.org/wiki/Zip_bomb). 8 MiB should be more than enough to hold any legitimate audit data.

### Step 3: Deserialize the JSON

Parse the decompressed data to JSON. A well-formed JSON is guaranteed to be UTF-8; rejecting non-UTF-8 data is valid behavior for the parser.

The JSON schema is available [here](cargo-auditable.schema.json).

### Step 4 (optional): Reconstruct the dependency tree

If your use case calls not just for obtaining the versions of the crates used in the build, but also for reconstructing the dependency tree, you need to validate the data first. The format technically allows encoding the following invalid states:

1. Zero root packages
1. More than one root package 
1. Cyclic dependencies

Before you walk the dependency tree, make sure that the dependency graph does not contain cycles - for example, by performing [topological sorting](https://en.wikipedia.org/wiki/Topological_sorting) - and that there is only one package with `root: true`.

(We have experimented with formats that do not allow encoding such invalid states, but they turned out no easier to work with - the same issues occur and have to be dealt with, just in different places. They were also less amenable to compression.)

