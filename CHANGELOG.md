# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.1.0] — 2026-04-29

First tagged release. `exp` is a CLI experiment tracker for agent runs,
prompt testing, and simulations — single binary, single SQLite file, no
runtime dependencies.

### Added

- Experiment lifecycle: `create`, `var set` (control / independent),
  `run start`, `run record`, `compare`.
- Domain-agnostic data model — works for LLM prompt evaluation, agent
  strategy comparison, parameter sweeps, and scientific simulations.
- Self-contained storage in a single SQLite database with artifacts
  stored as blobs.
- Agent-friendly `guide` and `describe` commands that let autonomous
  agents discover capabilities at runtime.
- Composable stdin/stdout interface — pipes cleanly with shell scripts
  or any language.
- Prebuilt binaries on each tagged release for Linux (x86_64, aarch64),
  macOS (x86_64, aarch64), and Windows (x86_64).

[0.1.0]: https://github.com/Battle-Creek-LLC/exp/releases/tag/v0.1.0
