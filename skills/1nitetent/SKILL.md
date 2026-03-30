---
name: 1nitetent
description: |
  Query 1nitetent.com campground spots (free one-night tent camping). Use when
  asked about 1nitetent, campgrounds, camping spots, tent spots, or "where can
  I camp near X". Supports near, search, spot, list, refresh subcommands.
user-invocable: true
argument-hint: <search-query>
allowed-tools: Bash(1nt *)
---

# 1nitetent Campground Query

Query 2,480+ free one-night camping spots from 1nitetent.com via the `1nt` CLI.

Process: `$ARGUMENTS`

## Decision Tree

```
User wants campground info
  ├── Near a location? ──────────── 1nt near <place> [--radius km]
  │     └── With text filter? ───── 1nt near <place> --search <term>
  ├── Text search? ──────────────── 1nt search <term>
  │     └── Near a location? ────── 1nt search <term> --near <place>
  ├── Specific spot detail? ─────── 1nt spot <id>
  ├── Browse all spots? ─────────── 1nt list [--limit N]
  └── Refresh data? ─────────────── 1nt refresh
```

## Compound Queries

`near` and `search` accept each other's filter as an optional flag:

```bash
1nt near Koblenz --search Dusche      # Spots near Koblenz with showers
1nt search Toilette --near Frankfurt   # Spots mentioning toilets near Frankfurt
```

## Output Formats

Output defaults to **table** in terminal, **JSON** when piped (agent use).
Override with `--output json|ndjson|table`.

Use `-F name,location,link` to filter output fields.

## JSON Envelope

```json
{"results": [...], "total_count": N, "showing": N, "has_more": true}
```

## Common Pitfalls

| Wrong | Right | Why |
|-------|-------|-----|
| Parsing table output | Use `--output json` | Structured data for programmatic use |
| `1nt near Frankfurt \| jq ...` | `1nt near Frankfurt --output json` | Avoid piping when flags exist |
| Multiple commands for compound queries | `1nt near X --search Y` | Single command, single result set |

## Ad-hoc Queries

For questions the subcommands don't cover, use `jq` on the enriched cache:

```bash
jq '<filter>' ~/.cache/1nt/campgrounds.enriched.geojson
```

The enriched GeoJSON has all original properties plus `location` (e.g.,
"Beverungen, Lower Saxony, DE").

## Exit Codes

| Code | Meaning |
|------|---------|
| 0 | Success |
| 1 | General error |
| 2 | Usage error |
| 3 | Not found |
| 4 | Network error |
| 5 | Cache error |
