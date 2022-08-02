# Contributing to `cargo auditable`

We're happy to accept contributions! We're looking for issue reports and general feedback, **not just code.** If `cargo auditable` doesn't work for you or doesn't fulfill all your requirements, please let us know!

If you're planning a big change to the code and would like to check with us first, please [open an issue](https://github.com/rust-secure-code/cargo-auditable/issues/new).

If you need help, or would like to chat with us, please talk to us in [`#wg-secure-code` on Rust Zulip](https://rust-lang.zulipchat.com/#narrow/stream/146229-wg-secure-code).

## Tips and tricks

To avoid running `cargo install` every time you want to rebuild and test a change, you can invoke the binary directly. So instead of this:

```
cargo install --path .
cargo auditable FLAGS
```

you can use

```
cargo build --release
target/release/cargo-auditable auditable FLAGS
```

which does not replace the stable version of `cargo auditable` that you may have installed.
