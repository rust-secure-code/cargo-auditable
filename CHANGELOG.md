# Changelog
All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## 0.5.1 - 2022-08-10

### Changed

 - `cargo auditable` now only records the dependencies used on the target platform. No more `windows` crates in dependency lists on Linux!

## 0.5.0 - 2022-08-08

### Added
 - `root: true` annotation to the root package in the serialized JSON, to make traversing the dependency graph easier.
 - When deserializing JSON, we validate that there is no more than one root package.
 - When deserializing JSON, we validate that there are no cycles in the dependency graph.
 - When converting to Cargo.lock format, source is now passed through if it's crates.io. The root package is also specified.

### Removed
 - The list of enabled features is no longer recorded in JSON, significantly reducing the binary size overhead. There didn't seem to be any use cases for it. If you need it, please contact us.

## 0.4.0 - 2022-08-06

### Added
 - "crates.io" is now recognized and encoded as a source, distinct from "registry"
 - `rust-audit-info` now supports custom limits set on the command line
 - `i686-unknown-linux-gnu`, `x86_64-unknown-linux-musl`, `x86_64-pc-windows-gnu` targets are now tested on CI

### Changed
 - `rust-audit-info` now defaults to the limit of 8MiB for the output size
 - `auditable-serde` has a strongly-typed `Source` type - an `enum` instead of a `String`

## 0.3.0 - 2022-08-03

### Added
 - Integration tests
 - Configured CI to run tests on x86_64 Linux, Mac and Windows

### Changed
 - `cargo auditable` now works via setting `RUSTC_WORKSPACE_WRAPPER` and acting as a workspace wrapper. This fixes workspace handling and numerous other issues.
 - On Apple platforms the audit data is now placed in the `__DATA` segment instead of `__TEXT` segment.
 - `auditable-serde::DependencyKind` enum variants are now CamelCase

### Removed
 - Dropped support for the `auditable` crate and `build.rs`-based injection. Please use `cargo auditable` instead.

## 0.2.0 - 2022-07-30
### Added
- An **experimental** `cargo auditable` subcommand to easily inject the audit data, without modifying build.rs

### Changed
- The section name is now `.dep-v0` across all platforms.

## 0.1.0 - 2020-09-07
 - Initial release
