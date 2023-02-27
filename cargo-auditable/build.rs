use std::io::prelude::*;

fn generate_man_page() -> String {
    man::prelude::Manual::new("cargo-auditable")
        .about("Embed a JSON formatted dependency tree into a dedicated linker section of the compiled executable")
        .author(man::prelude::Author::new("Sergey \"Shnatsel\" Davidoff").email("shnatsel@gmail.com"))
        .description("
Know the exact crate versions used to build your Rust executable. Audit binaries for known bugs or security vulnerabilities in production, at scale, with zero bookkeeping.

This works by embedding data about the dependency tree in JSON format into a dedicated linker section of the compiled executable.

Linux, Windows and Mac OS are officially supported. All other ELF targets should work, but are not tested on CI. WASM is currently not supported, but patches are welcome.

The end goal is to get Cargo itself to encode this information in binaries. There is an RFC for an implementation within Cargo, for which this project paves the way: https://github.com/rust-lang/rfcs/pull/2801
")
        .example(man::Example::new()
            .prompt("#")
            .text("Build your project with dependency lists embedded in the binaries")
            .command("cargo auditable build --release"))
        .custom(man::prelude::Section::new("FAQ")
            .paragraph("Doesn't this bloat my binary?

In a word, no. The embedded dependency list uses under 4kB even on large dependency trees with 400+ entries. This typically translates to between 1/1000 and 1/10,000 of the size of the binary.
")
            .paragraph("Can I make cargo always build with cargo auditable?

Yes! For example, on Linux/macOS/etc add this to your .bashrc:

alias cargo=\"cargo auditable\"

If you're using a shell other than bash, or if using an alias is not an option, see https://github.com/rust-secure-code/cargo-auditable/blob/HEAD/REPLACING_CARGO.md.
")
            .paragraph("Is there any tooling to consume this data?

Vulnerability reporting

    cargo audit v0.17.3+ can detect this data in binaries and report on vulnerabilities. See here for details.
    trivy v0.31.0+ detects this data in binaries and reports on vulnerabilities. See the v0.31.0 release notes for an end-to-end example.

Recovering the dependency list

    syft v0.53.0+ has experimental support for detecting this data in binaries. When used on images or directories, Rust audit support must be enabled by adding the --catalogers all CLI option, e.g syft --catalogers all <container image containing Rust auditable binary>.
    rust-audit-info recovers the dependency list from a binary and prints it in JSON.

It is also interoperable with existing tooling that consumes Cargo.lock via the JSON-to-TOML convertor. However, we recommend supporting the format natively; the format is designed to be very easy to parse, even if your language does not have a library for that yet.
")
            .paragraph("Can I read this data using a tool written in a different language?

Yes. The data format is designed for interoperability with alternative implementations. In fact, parsing it only takes 5 lines of Python. See https://github.com/rust-secure-code/cargo-auditable/blob/HEAD/PARSING.md for documentation on parsing the data.
")
            .paragraph("What is the data format, exactly?

The data format is described by the JSON schema https://github.com/rust-secure-code/cargo-auditable/blob/HEAD/cargo-auditable.schema.json. The JSON is Zlib-compressed and placed in a linker section named .dep-v0. You can find more info about parsing it here.
")
            .paragraph("What about embedded platforms?

Embedded platforms where you cannot spare a byte should not add anything in the executable. Instead they should record the hash of every executable in a database and associate the hash with its Cargo.lock, compiler and LLVM version, build date, etc. This would make for an excellent Cargo wrapper or plugin. Since that can be done in a 5-line shell script, writing that tool is left as an exercise to the reader.
")
            .paragraph("Does this impact reproducible builds?

The data format is specifically designed not to disrupt reproducible builds. It contains no timestamps, and the generated JSON is sorted to make sure it is identical between compilations. If anything, this helps with reproducible builds, since you know all the versions for a given binary now.
")
            .paragraph("Does this disclose any sensitive information?

No. All URLs and file paths are redacted, but the crate names and versions are recorded as-is. At present panic messages already disclose all this info and more. Also, chances are that you're legally obligated have to disclose use of specific open-source crates anyway, since MIT and many other licenses require it.
")
            .paragraph("What about recording the compiler version?

The compiler itself will start embedding it soon.

On older versions it's already there in the debug info. On Unix you can run strings your_executable | grep 'rustc version' to see it.
")
            .paragraph("What about keeping track of versions of statically linked C libraries?

Good question. I don't think they are exposed in any reasonable way right now. Would be a great addition, but not required for the initial launch. We can add it later in a backwards-compatible way. Adopting the -src crate convention would make it happen naturally, and will have other benefits as well, so that's probably the best route.
")
            .paragraph("What is blocking uplifting this into Cargo?

Cargo itself is currently in a feature freeze.
")
        )
        .render()
}

fn generate_man_page_file() -> Result<(), Box<dyn std::error::Error>> {
    let mut dest_path = std::path::PathBuf::from(std::env::var("OUT_DIR")?);
    dest_path.push("man-page");
    dest_path.push("cargo-auditable");
    std::fs::create_dir_all(&dest_path)?;

    dest_path.push("cargo-auditable.1");

    let mut file = std::fs::File::create(dest_path)?;
    file.write_all(generate_man_page().as_bytes())?;

    Ok(())
}

fn main() {
    generate_man_page_file().unwrap();
}
