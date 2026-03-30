# 1nt

Agent-native CLI for querying free one-night camping spots from
[1nitetent.com](https://1nitetent.com).

**Repository:** `~/Code/Private/1nitetent`

## CLI Shape

```
1nt [--output json|ndjson|table] [-F fields] [--json-errors]

1nt near <place|lat,lon> [--radius 50] [--search term] [--limit 30]
1nt search <term> [--near place|lat,lon] [--radius km] [--limit 30]
1nt spot <id>
1nt list [--limit 30]
1nt refresh
1nt version
```

## Output

- **Table** (default in TTY), **JSON** (default when piped), **NDJSON**
- JSON envelope: `{"results": [...], "total_count": N, "showing": N, "has_more": bool}`
- Field filtering: `-F name,location,link`
- Structured errors: `{"error": "...", "code": "..."}` on stderr

## Exit Codes

| Code | Meaning |
|------|---------|
| 0 | Success |
| 1 | General error |
| 2 | Usage / field filter error |
| 3 | Not found (spot ID, geocoding) |
| 4 | Network error |
| 5 | Cache error |

## Build

```bash
cargo build
cargo test
cargo clippy -- -D warnings
cargo fmt -- --check
```

**MSRV:** 1.94 (edition 2024)

## Commit Format

`scope: description` (e.g., `cache: add GeoNames pipeline`, `commands/near: add --search flag`)

## Dependencies

| Crate | Purpose |
|-------|---------|
| `clap` | CLI argument parsing (derive) |
| `serde` / `serde_json` | JSON serialization |
| `geojson` | GeoJSON parsing |
| `geo` | Haversine distance calculations |
| `reverse_geocoder` | Offline reverse geocoding (GeoNames) |
| `reqwest` | HTTP fetching (blocking) |
| `comfy-table` | Markdown table rendering |
| `anyhow` | Application error propagation |
| `thiserror` | Typed error definitions |
| `zip` | Extract cities1000.zip |

## Skills

- `skills/1nitetent/SKILL.md` — agent workflow guide
