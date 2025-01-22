# resolverver: Get the Cargo resolver version

Knowing the [Cargo resolver version](https://doc.rust-lang.org/cargo/reference/resolver.html#resolver-versions)
used in a given workspace is important to some tooling that interfaces with Cargo.
You'll know it when you need it.

### Usage

Since resolver version is a global property for the entire workspace,
it is important that you read the **workspace** Cargo.toml rather than
the Cargo.toml of an individual package.

Here's how to locate it using the [`cargo_metadata`](https://crates.io/crates/cargo_metadata) crate:

```rust
use cargo_metadata;
use resolverver;

// Locate and load the Cargo.toml for the workspace
let metadata = cargo_metadata::MetadataCommand::new().no_deps().exec().unwrap();
let toml = std::fs::read_to_string(metadata.workspace_root.join("Cargo.toml")).unwrap();

// Deduce the resolver version in use
let resolver_version = resolverver::from_toml(&toml).unwrap();
println!("Resolver version in this workspace is: {resolver_version:?}");
```

### Caveats

Cargo has a config option [`resolver.incompatible-rust-versions`](https://doc.rust-lang.org/cargo/reference/config.html#resolverincompatible-rust-versions)
that may enable V3 resolver even when everything else would indicate that V2 resolver should be used.

The only difference between V2 and V3 resolvers is the selected versions of some dependencies.
As long as you're letting Cargo generate the Cargo.lock file and aren't doing version resolution yourself,
this distinction doesn't matter.

If this does matter for your use case, you can use [`cargo-config2`](https://crates.io/crates/cargo-config2)
to read and resolve Cargo configuration.