# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.6.6] - 2024-11-24

### Changed

 - Audit data is now injected when `--print` argument is passed to `rustc` if `--emit=link` is also present in the same invocation. This adds support for `cargo c` third-party subcommand.
 - When `--emit` argument is passed to `rustc`, audit data will only be injected if one of the values passed is `link`. This should avoid messing with modes that emit assembly or LLVM bitcode.
 - Upgraded to `object` crate from v0.30 to v0.36 in order to reduce the dependency footprint.

### Fixed

 - Arguments to `rustc` in the style of `--key=value` (as opposed to `--key value`) are now parsed correctly. This was never an issue in practice because Cargo passes the arguments we care about separated by space, not `=`.

## [0.6.5] - 2024-11-11

### Added

 - Upgraded the `cargo_metadata` dependency to gain support for Rust 2024 edition

### Fixed

 - Fixed build on `riscv64-linux-android` target and certain custom RISC-V targets

## [0.6.4] - 2024-05-08

### Added

 - LoongArch support

## [0.6.3] - 2024-05-03

### Added

 - WebAssembly support

### Fixed

 - Pass the correct flag to MSVC link.exe to preserve the symbol containing audit data
   - This is not known to cause issues in practice - the symbol was preserved anyway, even with LTO.
 - Tests no longer fail on Rust 1.77 and later. The issue affected test code only.

### Changed

 - Refactored platform detection to be more robust

## [0.6.2] - 2024-02-19

### Fixed
 - Fixed `cargo auditable` encoding a cyclic dependency graph under [certain conditions](https://github.com/rustsec/rustsec/issues/1043)
 - Fixed an integration test failing intermittently on recent Rust versions

### Changed

 - No longer attempt to add audit info if `--print` arguments are passed to `rustc`, which disable code generation
 - Print a more meaningful error when invoking `rustc` fails

## [0.6.1] - 2023-03-06

### Added
 - A Unix manpage
 - An explanation of how the project relates to supply chain attacks to the README
 - Keywords to the Cargo manifest to make discovering the project easier

### Changed
 - Updated to `object` crate version 0.30 to enable packaging for Debian
 - Synced to the latest object writing code from the Rust compiler. This should improve support for very obscure architectures.

## [0.6.0] - 2022-12-07

### Changed

 - A build with `cargo auditable` no longer fails when targeting an unsupported architecture. Instead a warning is printed.
 - The `CARGO` environment variable is now read and honored; calls to Cargo will go through the binary specified in this variable instead of just `cargo`.

### Added

 - Added documentation on using `cargo auditable` as a drop-in replacement for `cargo`.

### Fixed

- Fixed build failures when the `RUSTC` environment variable or the `build.rustc` configuration option is set.

## [0.5.5] - 2022-12-01

### Fixed

- Long builds with `sccache` now work as expected. They require additional quirks compared to regular Cargo builds, see [#87](https://github.com/rust-secure-code/cargo-auditable/issues/87).
    - Note that `sccache` v0.3.1 or later is required even with this fix - earlier versions have a [bug](https://github.com/mozilla/sccache/issues/1274) that prevents them from working with `cargo auditable`.

## [0.5.4] - 2022-11-12

### Changed

- Updated README.md

## [0.5.3] - 2022-11-12

### Fixed

- `--offline`, `--locked`, `--frozen` and `--config` flags now work as expected. Previously they were not forwarded to `cargo metadata`, so it could still access the network, etc.

### Added 

- Re-introduced CHANGELOG.md
