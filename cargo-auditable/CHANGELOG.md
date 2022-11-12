# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.5.3] - 2022-11-12

### Fixed

- `--offline`, `--locked`, `--frozen` and `--config` flags now work as expected. Previously they were not forwarded to `cargo metadata`, so it could still access the network, etc.

### Added 

- Re-introduced CHANGELOG.md
