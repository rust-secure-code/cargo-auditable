## Parsing the data embedded by `cargo auditable`

Since the format simply uses Zlib and JSON, implementing a parser should be trivial. This is a barebones parser written in Python:

```python3
import lief, zlib, json
binary = lief.parse("/path/to/file")
audit_data_section = next(filter(lambda section: section.name == ".dep-v0", binary.sections))
json_string = zlib.decompress(audit_data_section.content)
audit_data = json.loads(json_string)
```
On Linux you can even kludge together a parser for Linux binaries in the shell, if you can't use [`rust-audit-info`](rust-audit-info/README.md):
```bash
objcopy --dump-section .dep-v0=/dev/stdout $1 | pigz -zd -
```

### Step 0: Check if a parser already exists

The following parsing libraries are available:

 - [`auditable-extract`]() in Rust
 - [`go-rustaudit`](https://github.com/microsoft/go-rustaudit) in Go

We also provide a standalone binary [`rust-audit-info`](rust-audit-info/README.md) that can be called as a subprocess from any language. It will handle all the binary wrangling for you and output the JSON. Unlike most binary parsers, it is designed for resilience and is written in 100% safe Rust, so the vulnerabilites that [plague other parsers](https://lcamtuf.blogspot.com/2014/10/psa-dont-run-strings-on-untrusted-files.html) are impossible in it.

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

