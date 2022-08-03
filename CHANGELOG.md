# Changelog
All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

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
