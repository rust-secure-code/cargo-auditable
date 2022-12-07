# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

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
