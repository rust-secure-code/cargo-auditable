# Changelog
All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [UNRELEASED]
### Changed
- Fixed changelog formatting

## [0.5.2] - 2022-10-24
### Changed
- `toml` feature: Versions are no longer roundtripped through `&str`, resulting in faster conversion.
- `toml` feature: `cargo_lock::Dependency.source` field is now populated when when converting into `cargo-lock` crate format.

### Added
- This changelog file

## [0.5.1] - 2022-10-02
### Added
- JSON schema (thanks to @tofay)
- A mention of the `auditable-info` crate in the crate documentation

## [0.5.0] - 2022-08-08
### Changed
- This is the first feature-complete release