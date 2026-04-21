# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.4.1] - 2026-04-21

### Fixed

- Switch cache TTL constants to `Duration::from_hours` and the filter
  check in `output.rs` to `Option::is_none_or`, clearing two new clippy
  lints introduced in Rust 1.95 (`duration_suboptimal_units`,
  `manual_is_variant_and`).
- Refresh transitive `rustls-webpki` to 0.103.12 to clear
  RUSTSEC-2026-0098 and RUSTSEC-2026-0099 (name-constraint handling).

## [0.4.0] - 2026-04-21

### Added

- Ship `bin/1nt` shim so the Claude Code plugin works without a separately
  installed `1nt` binary. The shim prefers any user-installed `1nt` on
  `PATH`; otherwise it reuses (or lazily downloads) a plugin-managed copy
  from GitHub releases matching the plugin's declared version.
- Add `SessionStart` hook (`hooks/ensure-binary.sh`) that keeps the
  plugin-managed binary in sync in the background, never blocking session
  startup.

### Notes

- The shim and hook prefer `jq` to parse `plugin.json` but fall back to a
  pure-bash `sed` extractor when `jq` is not on `PATH`, so first-run works
  on stock macOS, Alpine, and slim Linux containers.

## [0.3.0] - 2026-03-30

### Changed

- Bumped `actions/checkout` from v4 to v6
- Bumped `actions/upload-artifact` from v4 to v7
- Bumped `actions/download-artifact` from v4 to v8
- Removed private-repo `checks: write` permission workaround from CI
  (repo is now public)

## [0.2.0] - 2026-03-30

### Fixed

- Geocoding failure now returns exit code 3 (`not_found`) instead of 1
- Plugin skill permission now includes unnamespaced `Skill(1nitetent)` for
  auto-invocation without user approval
- CI audit job now has `checks: write` permission for private repos

### Changed

- Bumped `zip` dependency from v2 to v8
- Bumped MSRV from 1.85 to 1.94

## [0.1.0] - 2026-03-30

### Added

- `near` command with Haversine distance filtering, Nominatim geocoding, and
  optional `--search` text filter
- `search` command with case-insensitive text matching and optional `--near`
  spatial filter
- `spot` command for full detail view with HTML-stripped descriptions
- `list` command with configurable limit
- `refresh` command for forced cache re-download and re-enrichment
- `version` command showing cache status and spot count
- GeoJSON caching with 1-day TTL
- GeoNames reverse geocoding with 30-day TTL, admin1 name resolution
- Output formats: table (TTY default), JSON (pipe default), NDJSON
- Field filtering with `-F` flag
- Structured JSON error output for agent consumption
- Semantic exit codes (0-5)
- Claude Code plugin with pre-approved permissions
- Skill with decision tree and compound query documentation
- PostToolUse nudge hook for skill discovery

[Unreleased]: https://github.com/lukasmalkmus/1nitetent/compare/v0.4.1...HEAD
[0.4.1]: https://github.com/lukasmalkmus/1nitetent/compare/v0.4.0...v0.4.1
[0.4.0]: https://github.com/lukasmalkmus/1nitetent/compare/v0.3.0...v0.4.0
[0.3.0]: https://github.com/lukasmalkmus/1nitetent/compare/v0.2.0...v0.3.0
[0.2.0]: https://github.com/lukasmalkmus/1nitetent/compare/v0.1.0...v0.2.0
[0.1.0]: https://github.com/lukasmalkmus/1nitetent/releases/tag/v0.1.0
