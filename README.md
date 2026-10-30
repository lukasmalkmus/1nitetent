# 1nitetent

`1nt` queries free one-night camping spots from [1nitetent.com](https://1nitetent.com).

```sh
cargo install one-nite-tent --locked
1nt near Koblenz --search Dusche
```

## Agent plugins

The repository is an [Agent Plugins 1.0](https://agent-plugins.org) package. Add this repository through your client's plugin installer. Portable clients load root `plugin.json`, discover the `1nitetent` skill under `skills/`, and start the stdio MCP server from `mcp.json`.

Codex can install it through the repository's marketplace:

```sh
codex plugin marketplace add https://github.com/lukasmalkmus/1nitetent.git
codex plugin add 1nitetent@1nitetent
```

Claude Code retains its native marketplace and plugin configuration:

```sh
claude plugin marketplace add lukasmalkmus/1nitetent
claude plugin install 1nitetent@1nitetent
```

Both integrations share one skill and one binary launcher. The launcher prefers an installed `1nt`. Otherwise it caches or downloads the release matching the plugin version. Automatic downloads require Bash, curl, and tar on macOS or Linux. The optional startup hook prefetches that binary, but MCP startup also works without hooks. Hooks may require the client's trust review. The skill-reminder hook additionally requires jq.

The standalone Windows CLI remains available, but the bundled Bash launcher is not a native Windows launcher. Windows MCP hosts can run an installed `1nt mcp` directly.

The portable MCP descriptor places the campground cache below `PLUGIN_DATA/cache/1nt`. Standalone commands keep their existing `XDG_CACHE_HOME` or home-directory cache. Data refreshes need network access but no account or credentials. Host inheritance of custom environment-only configuration is client-dependent.

The shared [Agent Skill](skills/1nitetent/SKILL.md) follows the portable format. Clients decide how to expose manual skill invocation and interpret optional tool preapprovals.

## Packaging checks

```sh
uv run --python 3.13 --script scripts/validate_plugin.py
cargo build --locked
PLUGIN_TEST_BINARY=target/debug/1nt python3 scripts/test_plugin.py
```

The runtime checks initialize the actual bundled MCP server and list its tools without fetching campground data. Launcher tests use isolated caches and local download fixtures.

## Release ordering

Keep the Cargo, root plugin, Claude plugin, and Claude marketplace versions synchronized. Prepare and review a release commit off `main`, then tag that exact commit after publication is authorized. Verify every expected binary archive is available before promoting the package to marketplace `main`. Refresh installed plugin copies afterward. Never overwrite a published tag or fall back silently to an older binary.
