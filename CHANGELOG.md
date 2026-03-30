# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

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

[Unreleased]: https://github.com/lukasmalkmus/1nitetent/compare/v0.2.0...HEAD
[0.2.0]: https://github.com/lukasmalkmus/1nitetent/compare/v0.1.0...v0.2.0
[0.1.0]: https://github.com/lukasmalkmus/1nitetent/releases/tag/v0.1.0
